<!-- SPDX-License-Identifier: Apache-2.0 -->

# Bun + Rust FFI: best practices extracted from Bun's source

Read-only research deliverable for the `crates/aphrody-ffi` crate (the native
C-ABI `cdylib` that exposes the whole aphrody CLI in-process to Bun via
`bun:ffi`). Every claim below is grounded in a clone of the Bun source tree at
`var/bun` (shallow clone of `oven-sh/bun`, gitignored, never committed) and the
official `bun:ffi` documentation shipped in that tree.

Author: aphrody-code. No external code blocks longer than a handful of lines are
reproduced; patterns are paraphrased and cited as `path:line`.

Conventions used for citations: paths are relative to the Bun clone root
`var/bun/` unless noted.

---

## (a) Inventory of Rust crates in Bun

A foundational and surprising-to-some fact: **modern Bun is overwhelmingly
Rust**, not Zig. The repo's `CLAUDE.md` and `src/CLAUDE.md` state it directly:
`src/` is a Cargo workspace of "~200 member crates", the runtime ships as
`libbun_rust.a` built from `bun_bin`, and the `.zig` files that sit next to many
`.rs` files are "the original Zig implementation, kept only as a porting
reference — not compiled and not shipped" (`var/bun/CLAUDE.md`,
`var/bun/src/CLAUDE.md`). This means Bun's own FFI/dlopen and N-API surfaces are
real, current Rust and are directly applicable as a reference implementation for
aphrody-ffi.

The workspace root (`var/bun/Cargo.toml:1-111`) enumerates ~112 path members.
Edition is 2024 (`workspace.package.edition = "2024"`, matching aphrody). The
crates most relevant to FFI/ABI work:

| Crate (path) | Role | crate-type / shape |
| --- | --- | --- |
| `src/bun_bin` (`bun_bin`) | Cargo entrypoint; produces `libbun_rust.a` linked into the final binary | staticlib root |
| `src/runtime` (`bun_runtime`) | JS-visible APIs; hosts `runtime/ffi/` (`bun:ffi` impl) and `runtime/napi/` (N-API impl) | lib, linked into `bun_bin` |
| `src/jsc` (`bun_jsc`) | JSC value types, `Strong`/`Weak` GC handles, FFI imports, `URL` | lib |
| `src/sys` (`bun_sys`) | Cross-platform syscall wrappers; **owns `DynLib`** (the `dlopen`/`LoadLibrary` wrapper) | lib |
| `src/bun_alloc` (`bun_alloc`) | The mimalloc `#[global_allocator]` payload (`bun_alloc::Mimalloc`) + arena/vtable allocators | lib |
| `src/bun_core` (`bun_core`) | Strings, `heap::{into_raw, take, destroy}`, OOM handling, env vars | lib |
| `src/tcc_sys` (`bun_tcc_sys`) | TinyCC bindings — Bun JIT-compiles a per-symbol C trampoline for each FFI call | sys lib |
| `src/mimalloc_sys` (`bun_mimalloc_sys`) | Raw `mi_*` FFI | sys lib |

Beyond the workspace, two first-party Rust packages ship to crates.io and are
the cleanest references for "Rust author of a Bun-loaded native module":

- `packages/bun-native-plugin-rs` (`bun-native-plugin` 0.2.0, `var/bun/packages/bun-native-plugin-rs/Cargo.toml`)
  — a Rustified wrapper for writing **native bundler plugins**, which are
  technically N-API modules exporting a `#[no_mangle] extern "C"` lifecycle hook.
  Ships a companion proc-macro crate `bun-macro` (`#[bun]`).
- `packages/bun-build-mdx-rs` — an internal MDX compiler crate (not FFI-shaped;
  build tooling).

And a benchmark crate that is a perfect side-by-side of the two styles:

- `bench/ffi/src` (`ffi_napi_bench`, `crate-type = ["cdylib"]`,
  `var/bun/bench/ffi/src/Cargo.toml`) — exposes the *same* logic twice: once as
  a raw `#[no_mangle] unsafe extern "C" fn` for `bun:ffi`, once as a `#[napi]`
  function via `napi`/`napi-derive` 2.x. See section (b).

Release profile worth copying conceptually (`var/bun/Cargo.toml:146-188`): `lto =
"fat"`, `codegen-units = 1`, and notably `panic = "abort"` on both `release` and
`dev`. Bun deliberately runs `panic = "abort"`; the consequences for
`catch_unwind` are discussed in section (d).

Clippy posture worth mirroring for an FFI crate (`var/bun/Cargo.toml:204-271`):
`undocumented_unsafe_blocks = "deny"`, `not_unsafe_ptr_arg_deref = "deny"`,
`mem_forget = "deny"`, `ptr_as_ptr = "deny"`, `transmute_ptr_to_ptr = "deny"`,
`cast_ptr_alignment = "deny"`. These are exactly the lints that catch the
classic FFI footguns.

---

## (b) Best practices: `cdylib` + `bun:ffi`

### The ABI contract, end to end

`bun:ffi` is a **named-symbol, declare-the-signature** model (not self-
registering like N-API). The JS side calls `dlopen(path, symbols)` where
`symbols` maps each exported function name to `{ args: [...], returns: ... }`.
The native side just needs C-ABI exported symbols with matching arities.

1. **Library load.** `dlopen` (JS) normalizes the path then calls the native
   `FFI::open` (`var/bun/src/runtime/ffi/ffi_body.rs:1449`). It resolves the
   library extension per-OS — `so` on Linux/Android/FreeBSD, `dylib` on macOS,
   `dll` on Windows (`ffi_body.rs:1469-1478`) — then opens it via
   `bun_sys::DynLib::open(name)` and, on failure, retries against
   `FileSystem::abs(name)` (cwd-relative) before reporting `ERR_DLOPEN_FAILED`
   (`ffi_body.rs:1519-1555`).

2. **Symbol resolution.** For each declared symbol, Bun does
   `dylib.lookup::<*mut c_void>(&function_name)` and errors with `Symbol "X" not
   found in "lib"` if missing (`ffi_body.rs:1566-1584`). `DynLib`
   (`var/bun/src/sys/lib.rs:6033-6090`) is `dlopen(path, RTLD_LAZY)` /
   `LoadLibraryA` on the open side, `dlsym` / `GetProcAddressA` on the lookup
   side; lookup is a `transmute_copy::<*mut c_void, T>` guarded by a `const`
   assert that `size_of::<T>() == size_of::<*mut c_void>()`.

3. **Per-symbol trampoline (the part most people miss).** Bun does **not** call
   your symbol through a generic `libffi`-style dispatcher. For each symbol it
   JIT-compiles a tiny C trampoline with TinyCC (`function.compile(...)`,
   `ffi_body.rs:1586-1620`; the C codegen lives in the `Function`/`CompileC`
   machinery in the same file, e.g. `ffi_body.rs:1985-2083`). The trampoline
   marshals JS values to the native calling convention and back. Consequence:
   in a Bun build with TinyCC disabled, `dlopen`/`linkSymbols`/`callback`/`cc`
   all throw "not available in this build" (`ffi_body.rs:1450-1455`,
   `1641-1646`, `1283-1287`). aphrody-ffi cannot assume the host Bun has TinyCC;
   it is on by default in official Bun builds, but the `aphrody_run_captured`
   JSON path (which needs no per-element marshaling) is the robust fallback.

4. **The result object.** Each compiled symbol becomes a JS function created by
   `Bun__CreateFFIFunctionValue` (`ffi_body.rs:1609-1618`), exposed under
   `result.symbols`. The JS wrapper `FFIBuilder` (`var/bun/src/js/bun/ffi.ts:340-415`)
   inlines per-argument coercion closures and special-cases `cstring` returns by
   wrapping them in `new CString(...)`. Up to 9 arguments are monomorphized into
   non-variadic arrow wrappers for speed (`ffi.ts:373-411`).

### FFIType marshaling (the full table)

The canonical enum is `var/bun/src/js/bun/ffi.ts:1-61`. Numeric tags:

- Integers: `i8=1 u8=2 i16=3 u16=4 c_int/i32/int=5 c_uint/u32=6 i64=7 u64=8`.
- Floats: `f64/double=9 f32/float=10`. `bool=11`.
- Pointers: `ptr/pointer/void*/char*=12`. `void=13`.
- `cstring=14`. `i64_fast=15`, `u64_fast=16` (return as JS number when in safe
  range, else BigInt). `function/callback/fn=17`.
- N-API bridge types: `napi_env=18`, `napi_value=19`, `buffer=20`.

Marshaling rules to design exports around (`ffi.ts:159-338`):

- **`i64`/`u64` use BigInt** at the boundary; `i64_fast`/`u64_fast` opportunis-
  tically return a JS number when the value fits in `Number.MAX_SAFE_INTEGER`
  (`ffi.ts:185-251`). For aphrody's `int32_t`/`size_t`-style returns prefer
  `i32`/`u32`; only reach for 64-bit when you actually need the range.
- **Pointers arrive as JS numbers** (or BigInt). The coercion accepts a number,
  a TypedArray view, or an `ArrayBuffer` (calling `ptr()` on it), and *throws*
  on a JS string — "To convert a string to a pointer, encode it as a buffer"
  (`ffi.ts:293-312`). This is the load-bearing zero-copy rule: **you pass
  `Uint8Array`/its `ptr()`, never a JS string**.
- There is a hard ceiling: a pointer above `MAX_ADDRESSABLE_MEMORY = 2^56 - 1`
  is rejected as "outside max addressible memory" because JSC encodes the
  address inside a JS double (`var/bun/src/runtime/ffi/FFIObject.rs:1103-1108`,
  validated in `ptr_` at `FFIObject.rs:645-647`). 56-bit addresses are fine on
  all current 64-bit OSes; just be aware the FFI number is not a full 64-bit
  pointer.

### Zero-copy: how memory actually crosses the boundary

This is the section most relevant to aphrody's ZERO-ALLOCATION directive.

- **JS -> native (the preferred direction).** `ptr(uint8Array)` returns the
  backing data pointer as a JS number with no copy
  (`FFIObject.rs:593-599`: `JSValue::from_ptr_address((*array).ptr() as usize)`).
  Native code reconstructs a slice with `std::slice::from_raw_parts(ptr, len)`.
  The benchmark crate is the exemplar (`var/bun/bench/ffi/src/src/lib.rs:47-49`):

  ```rust
  #[no_mangle] unsafe extern "C" fn ffi_hash(ptr: *const u8, length: u32) -> u32 {
    return hash(std::slice::from_raw_parts(ptr, length as usize));
  }
  ```

  This is exactly the "allocate in JS, pass `ptr()`, mutate in place" model.
  aphrody-ffi's existing `Uint8Array`-in / mutate-in-place approach is correct
  and matches Bun's own fast path.

- **native -> JS view (zero-copy, but ownership-critical).** `toArrayBuffer` /
  `toBuffer` wrap a raw `(ptr, len)` as a JS `ArrayBuffer`/`Buffer` **without
  copying** (`FFIObject.rs:798-913`, calling `ArrayBuffer::from_bytes(...)` /
  `create_buffer_with_ctx(...)`). The catch is the deallocator. Two modes:
  - With a user-supplied `(deallocatorContext, JSTypedArrayBytesDeallocator)`
    pair, the bytes are freed by *your* callback when the GC collects the JS
    object (`FFIObject.rs:809-847`, `create_buffer_with_ctx` at
    `FFIObject.rs:34-60`). The C signature is the JSC standard
    `void (*)(void *bytes, void *deallocatorContext)`
    (`var/bun/docs/runtime/ffi.mdx:441-445`).
  - **With no callback**, `toBuffer` installs `MarkedArrayBuffer_deallocator`,
    which `mi_free`s the slice on GC. The Bun source flags this as a footgun in
    its own comment: it "matches Zig exactly (including the free-foreign-memory
    footgun)" (`FFIObject.rs:907-910`). If those bytes were not allocated by the
    same mimalloc the runtime uses, GC will corrupt the heap. **Rule for
    aphrody-ffi: never return a no-callback `toBuffer` view over memory you did
    not allocate with the runtime's allocator.** Prefer either (i) JS owns the
    buffer and you only mutate it, or (ii) you pass a real deallocator.

- **C strings.** Two distinct things:
  - As an **argument**, `cstring` is identical to `ptr` (`docs/runtime/ffi.mdx:217`).
  - As a **return type**, `cstring` coerces the returned `char*` into a JS string
    by *copying* it into a WTFString (`new_cstring` ->
    `bun_string_jsc::create_utf8_for_js`, `FFIObject.rs:84-98`). The JS
    `CString` class extends `String` and **clones** the C string, so it is safe
    to free the C pointer afterward — the docs are explicit:
    "The `new CString()` constructor clones the C string, so it is safe to
    continue using `myString` after `ptr` has been freed"
    (`docs/runtime/ffi.mdx:174-211`). Implication: a Rust function returning a
    `cstring` must still own and later free that buffer; the coercion does not
    take ownership.

- **Direct reads.** For short-lived scalar reads `bun:ffi` exposes `read.u8`,
  `read.i32`, `read.ptr`, etc. (`docs/runtime/ffi.mdx:399-427`) which read
  through the raw address with no `DataView`/`ArrayBuffer` allocation
  (`FFIObject.rs:295-329`, `read_unaligned_at`). Useful if aphrody returns a
  small `#[repr(C)]` struct by pointer and JS wants to peek a couple of fields
  without materializing a view.

### Concrete Rust export patterns Bun uses / endorses

- **`#[no_mangle] unsafe extern "C" fn`** for every exported symbol
  (`bench/ffi/src/src/lib.rs:25,36,47`). aphrody is on edition 2024, so the
  spelling is `#[unsafe(no_mangle)]` — which aphrody-ffi already uses. Bun's
  bench crate is edition 2021, hence the bare `#[no_mangle]`; the semantics are
  identical.
- **Primitive types only at the boundary** — `*const u8`, `*mut c_void`, `u32`,
  `i32`, fixed-width ints, and raw pointers. The bench crate returns `*const u8`
  for a static string (`ffi_string`, `bench/ffi/src/src/lib.rs:36-38`) rather
  than any Rust `String`/`&str` type. aphrody-ffi's use of `uint8_t*`, `size_t`,
  `int32_t` and `*mut c_char` is exactly this.
- **Ownership round-trip for opaque handles via `CString::into_raw` /
  `from_raw` (or `Box`).** Bun's native-plugin crate hands an output buffer to
  the runtime by `String::leak()` then frees it later through a registered
  callback that calls `String::from_raw_parts(ptr, len, cap)` + `drop`
  (`var/bun/packages/bun-native-plugin-rs/src/lib.rs:582-625`). The key detail
  it tracks that a bare `into_raw`/`from_raw` does not: it stores **capacity**
  alongside ptr+len, because `String::from_raw_parts` needs the original
  capacity to free correctly. For aphrody's `aphrody_string_free`, the safe
  contract is: only ever free pointers produced by `CString::into_raw` from the
  same library, and reconstruct with `CString::from_raw` (which finds the NUL
  itself, sidestepping the capacity problem). Do **not** mix `Box<[u8]>` /
  `String` / `CString` raw pointers through one free function.
- **`#[repr(C)]` for any struct that crosses the boundary**
  (`ffi_body.rs:104-110` `struct Offsets`, written by C++ and read by Rust). The
  surrounding code shows the additional subtlety that a struct *mutated by the
  other language* must not be a plain immutable `static` — Bun wraps it in a
  `RacyCell<T>` (`#[repr(transparent)]` over `UnsafeCell`) so the layout is
  unchanged but the optimizer is not told the bytes are immutable
  (`ffi_body.rs:112-122`).
- **`catch_unwind` on every entry point.** See section (d) — Bun's `#[bun]`
  macro does exactly this.
- **mimalloc as `#[global_allocator]`** — Bun's payload is
  `bun_alloc::Mimalloc` implementing `GlobalAlloc` over `mi_malloc_auto_align` /
  `mi_free` (`var/bun/src/bun_alloc/lib.rs:810-867`), installed once at the
  binary root (`src/CLAUDE.md`: "`#[global_allocator] static ALLOC:
  bun_alloc::Mimalloc = bun_alloc::Mimalloc;` must be set at the binary root").
  aphrody-ffi installs its own mimalloc in the `cdylib`, which is correct
  because it depends on the cli *library* target (no binary `#[global_allocator]`
  is pulled in) — its `Cargo.toml:42-46` comment already documents this.

### Threading model (synchronous, but loader is process-global)

- **FFI symbol calls are synchronous and run on the calling JS thread.** The
  JS-side wrapper is a plain function call into the JIT trampoline
  (`ffi.ts:355-411`); there is no task posting. So a long-running
  `aphrody_run(...)` *blocks the JS thread/event loop* for its whole duration.
  This is the single biggest design consideration for exposing a full CLI: a
  command that does network or disk I/O will stall the Bun event loop. See
  recommendations.
- **The OS loader is thread-safe.** `DynLib` is `unsafe impl Send + Sync` with
  the rationale that "the underlying loader is process-global and internally
  synchronized; `dlsym`/`GetProcAddress` may be called from any thread"
  (`var/bun/src/sys/lib.rs:6036-6042`). So a JS Worker can independently
  `dlopen` the same aphrody `cdylib`; each gets its own symbol table view but
  shares one loaded image and one set of process-global statics (including the
  one mimalloc heap and any `OnceLock` you put in the crate).
- **Callbacks into JS (`FFIType.function`) need `threadsafe: true` if invoked
  off the JS thread.** `JSCallback` defaults `threadsafe=false`
  (`ffi.ts:82-116`, `docs/runtime/ffi.mdx:314-322`). The native side stores an
  opaque `ctx` pointer (`bun_core::heap::into_raw(Box::new(...))`,
  `ffi_body.rs:1328-1337`) that JS passes back to `closeCallback` for teardown
  (`ffi_body.rs:1272-1276`). If aphrody ever exposes a progress/streaming
  callback that fires from a tokio worker thread, it must be created
  `threadsafe`.

---

## (c) Best practices: node-api / napi-rs

Bun ships a from-scratch N-API implementation in Rust
(`var/bun/src/runtime/napi/napi_body.rs`, ~4257 lines) and exercises napi-rs as a
client (`bench/ffi`). Patterns worth adopting if aphrody ever ships a `.node`
addon instead of (or alongside) the raw `cdylib`:

- **Module registration is self-describing, unlike `bun:ffi`.** A `.node` addon
  either runs a static constructor that calls `napi_module_register`, or exports
  `napi_register_module_v1(env, exports)` (`var/bun/src/runtime/napi/node_api.h:58-103`).
  napi-rs emits the latter for you. The trade-off vs. `bun:ffi`: with N-API the
  addon *builds its own JS object graph* (functions, classes, properties) at
  load time; with `bun:ffi` the JS caller declares the surface. For "expose the
  whole CLI" N-API would let aphrody present a richer typed API, at the cost of
  linking against Node/Bun headers and the N-API ABI.
- **`#[napi]` macros + ergonomic types.** The bench crate shows the idiomatic
  shape (`bench/ffi/src/src/lib.rs:20-45`): `#[napi] pub fn napi_hash(buffer:
  Buffer) -> u32`. napi-rs marshals `Buffer`/`String`/numbers automatically; you
  do not touch raw pointers. `Buffer` is a borrowed view over JS-owned bytes —
  the zero-copy story is preserved without `from_raw_parts`.
- **`External<T>` for opaque state, and it must be `Sync`.** Bun's native-plugin
  guide is explicit: state passed to/from JS via `External` "must be threadsafe.
  This usually means that your state must be `Sync`"
  (`var/bun/packages/bun-native-plugin-rs/src/lib.rs:142-164`,
  `README.md:154-169`). The accessor enforces it in the type signature:
  `external<T: 'static + Sync>(...)` (`lib.rs:528-539`), and the `_mut` variant
  is `unsafe` precisely because "you must ensure that no other invocation of the
  plugin (or JS!) simultaneously holds a mutable reference" (`lib.rs:541-557`).
- **`ThreadsafeFunction` is the correct cross-thread callback primitive.** Bun's
  implementation (`napi_body.rs:2393-2890`) shows the canonical mechanics:
  - Off-thread callers `enqueue` a `ctx` pointer under a lock, with a blocking
    or non-blocking mode (returning `queue_full` when non-blocking and the queue
    is blocked) (`napi_body.rs:2626-2652`).
  - `schedule_dispatch` posts a `ConcurrentTask` onto the JS event loop exactly
    once per idle->pending transition (`napi_body.rs:2654-2671`).
  - The actual JS invocation happens **on the JS thread**, inside a
    `NapiHandleScope`, draining microtasks first, and reports any thrown
    exception as an unhandled rejection rather than propagating it
    (`napi_body.rs:2583-2624`).
  - Lifetime is refcounted via `acquire`/`release`/`ref`/`unref`; releasing the
    last reference (or `abort`) tears it down (`napi_body.rs:2723-2890`).

  The lesson for any Rust addon: never call a JS function from a non-JS thread
  directly; route through a `ThreadsafeFunction` that hops to the JS thread.
- **Panic safety: never unwind into the runtime.** napi-rs wraps `#[napi]` fn
  bodies in `catch_unwind` and converts panics to thrown JS errors. Bun's
  native-plugin macro does the same and additionally warns it "may catch *some*
  panics but [is] **not guaranteed to catch all**" — "avoid panics at all costs"
  (`var/bun/packages/bun-native-plugin-rs/README.md:141-152`). Combine
  `catch_unwind` with `panic = "abort"` awareness (section d).
- **Error handling: return errors, do not `.unwrap()`.** Bun's own Rust
  guidance: "Don't `.unwrap()` a fallible path that user input or the OS can hit
  at runtime — return the error. `.unwrap()` is for invariants you can prove"
  (`var/bun/src/CLAUDE.md`). The native-plugin crate logs `Err(...)` to the
  bundler instead of panicking (`lib.rs:559-579`, `README.md:143-147`).
- **GC handle discipline.** Bun's JSC notes (`var/bun/src/CLAUDE.md`, "Strong /
  Weak JS handles") apply to any addon holding JS values: a `Strong` keeps a
  value alive and is `!Send`/`!Sync` — it must be created and dropped on the JS
  thread; `to_js()`/`create()` that returns a wrapped pointer **transfers** the
  caller's `+1` refcount to the JS wrapper (an extra `ref` leaks, a missing one
  UAFs at GC).

---

## (d) Critical review and concrete recommendations for `crates/aphrody-ffi`

Current exported surface under review: `aphrody_abi_version()`,
`aphrody_version()`, `aphrody_run(argc, argv)`, `aphrody_run_json(args_json)`,
`aphrody_run_captured(args_json) -> JSON {code, stdout, stderr}`,
`aphrody_string_free()`, `aphrody_last_error()`; global mimalloc; stdout/stderr
capture via `dup2` (unix) / `SetStdHandle` (windows); `#[unsafe(no_mangle)]`
(edition 2024); `catch_unwind` on each entry. (Note: at the time of writing the
crate ships only `Cargo.toml` — these recommendations are for the `.rs` to be
written; the `Cargo.toml` choices are already sound.)

### What is already right (keep it)

1. **`crate-type = ["cdylib", "rlib"]`, staticlib intentionally omitted.**
   Matches Bun's distribution shape (one dynamic image for the loader). The
   `rlib` for in-process unit tests is a good call. (`Cargo.toml:14-20`.)
2. **Own mimalloc in the `cdylib`, depending on the cli *library* target.** This
   avoids a double `#[global_allocator]` and matches Bun's "set it at the binary
   root" rule applied to a `cdylib` root. (`Cargo.toml:40-46`.)
3. **`#[unsafe(no_mangle)] extern "C"` + primitive types.** This is precisely
   the bench-crate pattern (`bench/ffi/src/src/lib.rs`).
4. **`catch_unwind` on every entry.** This is the `#[bun]` macro's exact strategy
   (`bun-macro/src/lib.rs:28-49`). Keep it.
5. **wasm gating to an empty module.** Correct: `bun:ffi`'s own `open()` has a
   `TODO(port): wasm @compileError("TODO")` (`ffi_body.rs:1477`); the
   tokio/reqwest/rustls surface does not link on `wasm32-unknown-unknown`.

### Gaps and concrete improvements (actionable)

1. **`catch_unwind` is necessary but not sufficient — pin the panic strategy.**
   Bun runs `panic = "abort"` workspace-wide (`var/bun/Cargo.toml:151,154`) and
   its own macro warns `catch_unwind` "may catch some panics but [is] not
   guaranteed to catch all" (`README.md:149-150`). With the *default* `panic =
   "unwind"`, `catch_unwind` works but unwinding across the C-ABI is UB if a
   panic ever escapes the guard (e.g. from a drop during stack cleanup, or from
   code that set `panic=abort`). Recommendation: keep `catch_unwind` AND set a
   process-wide panic hook in an init function that logs into the
   `aphrody_last_error` slot, and document that the crate is built `panic =
   "unwind"`. Do not rely on `catch_unwind` alone to make a panic observable —
   capture the payload string into `aphrody_last_error` inside the guard's `Err`
   arm, exactly as the `#[bun]` macro formats `"Plugin crashed: {:?}"`
   (`bun-macro/src/lib.rs:38-49`).

2. **Add a `#[repr(C)]` struct variant of `aphrody_run_captured` to escape JSON
   on the hot path.** The JSON form is a fine default and the most robust one
   (it needs no per-field marshaling and survives a TinyCC-less Bun). But for
   ZERO-ALLOCATION it forces (a) a serde allocation in Rust and (b) a JSON parse
   in JS. Mirror Bun's `Offsets`/`SystemError` `#[repr(C)]` pattern
   (`ffi_body.rs:104-110`): expose

   ```rust
   #[repr(C)]
   pub struct AphrodyCaptured {
       code: i32,
       stdout_ptr: *mut u8, stdout_len: usize,
       stderr_ptr: *mut u8, stderr_len: usize,
   }
   ```

   returned by pointer, with `stdout_ptr`/`stderr_ptr` either (i) owned by Rust
   and freed by a paired `aphrody_captured_free`, or (ii) written into
   JS-provided `Uint8Array`s. JS reads the four scalars with `read.i32`/
   `read.ptr`/`read.u64` (`docs/runtime/ffi.mdx:399-427`) and views the bytes
   with `toArrayBuffer(ptr, 0, len, ctx, free_cb)` — true zero-copy. This keeps
   the JSON path as the portable default and adds a fast path.

3. **For stdout/stderr capture, prefer a caller-provided buffer over Rust-owned
   allocation where possible.** aphrody's ZERO-ALLOCATION directive says
   "allocate only in JS". A capture API that takes `out_ptr: *mut u8, out_cap:
   usize, out_len: *mut usize` and writes in place (truncating or returning
   "needs N bytes") removes the Rust-side allocation entirely. Where the output
   size is unknown up front, the `#[repr(C)]` + deallocator pattern from (2) is
   the next best thing. Either way, **never** hand a Rust-allocated buffer to a
   no-callback `toBuffer` — that triggers Bun's `mi_free`-foreign-memory footgun
   (`FFIObject.rs:907-910`); since aphrody and Bun may not share the *same*
   mimalloc instance (two images, two heaps), always free aphrody-allocated
   bytes through an aphrody-exported free function, not via JS GC.

4. **Expose an explicit runtime init/teardown for a persistent tokio runtime.**
   This is the most impactful missing piece. `aphrody_run*` per-call is fine for
   one-shots, but FFI calls are **synchronous and block the JS thread**
   (`ffi.ts:355-411`; there is no task posting), and standing up + tearing down a
   tokio runtime on every call is pure latency. Bun's pattern for expensive
   shared state is process-global init guarded by `Once`/`OnceLock`
   (`ffi_body.rs:184-195` `Offsets::get`, and the `dlsym_with_handle!` macro at
   `var/bun/src/sys/lib.rs:6144-6162` using `Once` + `AtomicPtr`). Recommend:
   - `aphrody_runtime_init() -> i32` builds a `tokio::runtime::Runtime` into a
     `OnceLock<Runtime>` (multi-thread, reused across calls). aphrody's own
     memory note (`latency-minimal-objective`) already calls for client reuse /
     cold-start minimization — this is the FFI manifestation.
   - `aphrody_runtime_shutdown()` for clean teardown (and so a Worker that
     `dlopen`s and drops the lib does not leak the runtime image-globally).
   - Keep per-call `aphrody_run*` working without explicit init by lazily
     initializing the runtime on first use (the `Once` pattern), so the API
     stays usable from a bare `dlopen` with no setup call.

5. **Decide and document the threading contract; consider an async/non-blocking
   variant.** Because the call blocks the event loop, a CLI command doing real
   I/O will stall Bun. Two complementary options:
   - Document loudly that `aphrody_run*` is blocking and should be called from a
     Worker for long commands. `DynLib` being `Send + Sync`
     (`var/bun/src/sys/lib.rs:6036-6042`) means a Worker can `dlopen` the lib
     independently — this is a legitimate, low-effort answer.
   - Or expose a non-blocking shape: `aphrody_run_async(args_json, done_cb_ptr,
     ctx)` where `done_cb` is a `FFIType.function` created `threadsafe: true`
     on the JS side, invoked from the tokio runtime when the command finishes.
     This is the `bun:ffi` callback + `JSCallback({ threadsafe: true })` path
     (`ffi.ts:82-116`, `docs/runtime/ffi.mdx:314-322`) and the N-API
     `ThreadsafeFunction` discipline (`napi_body.rs:2583-2671`): do the work on
     any thread, hop to the JS thread to fire the callback. This is more work
     but is the only way to expose a long-running CLI command without freezing
     the Bun event loop.

6. **Harden re-entrancy.** A single process-global capture-via-`dup2`/
   `SetStdHandle` is not safe under concurrent calls (two Workers, or a callback
   that re-enters `aphrody_run`). Bun explicitly designs its native entry points
   to be callable "on any thread at any time and multiple times at once"
   (`var/bun/packages/bun-native-plugin-rs/src/lib.rs:242-246`). Recommendations:
   - Guard the stdout/stderr redirection with a mutex (serialize captures), or
     better, scope the capture so concurrent callers do not clobber each other's
     fd dup state. A global fd swap is inherently process-wide; a `Mutex` around
     the capture window is the minimum.
   - Make `aphrody_last_error` thread-local, not a single global, so two threads'
     errors do not race. Bun keeps per-thread error state on the N-API `env`;
     the `cdylib` equivalent is `thread_local!`.
   - Route any heap round-trip through a single discipline
     (`bun_core::heap`-style `into_raw`/`take`/`destroy` analog) and never free a
     pointer that did not originate from the matching constructor. The clippy
     lints to enforce this are `not_unsafe_ptr_arg_deref`, `mem_forget`,
     `transmute_ptr_to_ptr` (all `deny` in Bun, `var/bun/Cargo.toml:204-271`).

7. **`aphrody_string_free` contract must be exact and single-typed.** Bun's
   plugin crate frees its handed-out buffer with the matching
   `String::from_raw_parts(ptr, len, cap)` and tracks capacity to do so
   (`lib.rs:582-625`). For a NUL-terminated C string the clean equivalent is:
   produce with `CString::into_raw`, free with `CString::from_raw` (which locates
   the NUL and reconstructs the correct allocation without needing capacity).
   Document that `aphrody_string_free` accepts **only** pointers returned by
   aphrody's own `into_raw`-based exports, and never a pointer JS computed or a
   `toBuffer`-owned pointer. Add a `// SAFETY:` block per Bun's
   `undocumented_unsafe_blocks = "deny"` posture.

8. **Adopt Bun's FFI clippy lints in `[lints]`.** aphrody-ffi inherits
   `workspace = true`. Consider per-crate `#![deny(clippy::not_unsafe_ptr_arg_deref,
   clippy::mem_forget, clippy::transmute_ptr_to_ptr, clippy::cast_ptr_alignment)]`
   and `#![warn(clippy::undocumented_unsafe_blocks)]` to catch the exact bug
   classes this crate is most exposed to (`var/bun/Cargo.toml:204-271`).

9. **Version/ABI handshake is good — extend it.** `aphrody_abi_version()` is the
   right primitive; N-API formalizes this with `NAPI_MODULE_VERSION`
   (`node_api.h:58-67`). Recommend JS asserts `aphrody_abi_version()` against an
   expected integer on `dlopen` and refuses to proceed on mismatch, so a stale
   `.so`/`.dll` next to a newer TS wrapper fails loudly instead of corrupting via
   a shifted `#[repr(C)]` layout (the same hazard `Offsets` guards against,
   `ffi_body.rs:104-122`).

### Summary verdict

aphrody-ffi's core choices — `cdylib` + `rlib`, own mimalloc, `#[unsafe(no_mangle)]
extern "C"` with primitive types, `catch_unwind` per entry, the JSON
`aphrody_run_captured` as a portable default, and the JS-allocates / pass-`ptr()`
zero-copy direction — all match what Bun does in its own first-party Rust FFI and
N-API code. The meaningful additions are: a persistent lazily-initialized tokio
runtime behind `OnceLock`, a `#[repr(C)]` fast path beside the JSON one,
thread-local error state plus a capture mutex for re-entrancy, an
async/threadsafe-callback variant for long commands so the Bun event loop is not
blocked, and a strictly single-typed `string_free` contract. None require
touching the cli crate.

---

## Appendix: primary sources consulted (all under `var/bun/`)

- `src/js/bun/ffi.ts` — the `FFIType` enum, `FFIBuilder`, `dlopen`/`cc`/
  `linkSymbols`, `CString`/`JSCallback` JS classes (the public contract).
- `src/runtime/ffi/ffi_body.rs` — `FFI::open` (dlopen), symbol lookup +
  TinyCC compile, `callback`, `close`, `Offsets`/`RacyCell`, finalizer policy.
- `src/runtime/ffi/FFIObject.rs` — `ptr`, `toArrayBuffer`/`toBuffer`,
  `cstring`, `read.*`, the deallocator-context zero-copy path, the
  `MAX_ADDRESSABLE_MEMORY` (2^56-1) ceiling.
- `src/sys/lib.rs` — `DynLib` (`dlopen`/`dlsym` vs `LoadLibraryA`/
  `GetProcAddressA`), its `Send + Sync` rationale, `dlsym_with_handle!`.
- `src/bun_alloc/lib.rs` — `bun_alloc::Mimalloc` `GlobalAlloc` impl;
  `src/bun_alloc/MimallocArena.rs` — arena/Send/Sync notes.
- `src/runtime/napi/napi_body.rs` — `ThreadSafeFunction` enqueue/dispatch/call,
  refcount lifecycle; `src/runtime/napi/node_api.h` — module registration.
- `packages/bun-native-plugin-rs/src/lib.rs` + `bun-macro/src/lib.rs` +
  `README.md` — first-party Rust native module: `#[no_mangle] extern "C"`,
  `catch_unwind` macro, `External<T: Sync>`, leak/free-with-capacity pattern,
  "callable on any thread, multiple times at once" concurrency contract.
- `bench/ffi/src/src/lib.rs` — side-by-side raw-FFI vs `#[napi]` of identical
  logic (the `(ptr, len)` -> `from_raw_parts` zero-copy exemplar).
- `docs/runtime/ffi.mdx` — official contract: type table, `CString` clones,
  memory management / deallocator signature, pointer alignment, "FFI does not
  manage memory for you".
- `Cargo.toml`, `CLAUDE.md`, `src/CLAUDE.md` — workspace shape (~200 Rust
  crates), release profile (`panic = "abort"`, fat LTO), FFI-relevant clippy
  lints, and the `#[global_allocator]` rule.
