# Architectural Reference: The Rust-Bun Integration (bun-rs)

This document serves as the definitive reference guide to the integration between Rust and Bun inside the `bun` repository (`/tmp/bun`). It covers how Rust has replaced Zig as the primary system layer driving Bun's runtime, compilers, database client contexts, and compilation pipelines.

---

## 1. Executive Summary & Architecture Shift

Bun has completed a transition from a runtime written primarily in Zig to a **hybrid Rust + C++ architecture**.

```mermaid
graph TD
    subgraph Host OS Lifecycle
        A[OS Loader / crt1.o] -->|Spawns process| B[Rust main Entry point]
        B -->|Initialises| C[mimalloc Global Allocator]
        B -->|Configures| D[Crash Handler / Signals]
        B -->|Invokes CLI| E[bun_runtime::cli]
    end

    subgraph Runtime VM Execution
        E -->|Spawns JS Context| F[C++ JavaScriptCore Glue]
        F <-->|Zero-Copy C-ABI FFI| G[Rust Subsystems]
    end

    subgraph Rust Subsystems (libbun_rust.a)
        G --> H[AST Parser & Transpiler]
        G --> I[Package Manager]
        G --> J[Database Drivers]
        G --> K[FFI JIT Engine]
        G --> L[Valkey RESP3 Client]
    end
```

* **Rust's Role:** Acts as the process supervisor and main entry point. It manages system initialization, thread stacks, memory allocation boundaries, and houses all high-performance subsystems (compilers, bundler, package manager, test runner, Valkey, and databases).
* **C++'s Role:** Manages direct API bindings to the WebKit/JavaScriptCore (JSC) Virtual Machine.
* **Zig's Role (Deprecated):** The Zig compiler is no longer used in the build process. Sibling `.zig` files exist solely as a developer spec/reference for porting semantics.

---

## 2. Process Entry Point (`src/bun_bin/lib.rs`)

The process entry point is implemented in Rust under the `bun_bin` static library crate. The operating system loader calls Rust’s `main` symbol:

```rust
#[unsafe(no_mangle)]
pub unsafe extern "C" fn main(argc: c_int, argv: *const *const c_char) -> c_int
```

### Startup Initialization Chain
1. **Argument Capture:** Immediately calls `bun_core::init_argv` to grab `argv` pointers before constructors run, guaranteeing proper stack inspection.
2. **Crash & Panic Handler:** Initialises `bun_crash_handler::init()` to catch signals and dump symbolized stack traces.
3. **Signal Configurations:** Ignores `SIGPIPE` and `SIGXFSZ` to prevent crashing on interrupted socket writes.
4. **Allocator & OS Hooks (Windows):** Intercepts standard Libuv allocations using `uv_replace_allocator` mapping to `mimalloc`, and converts the host environment block to WTF-8.
5. **Stdio Buffering:** Configures stdio handles and locks flush guards.
6. **Thread Configuration:** Sets thread stack bounds via `StackCheck::configure_thread()` for JS recursion safety.
7. **CLI Dispatch:** Calls `bun_runtime::cli::Cli::start()` to run the command loop.

---

## 3. Shared Memory Allocator & Zero-Copy Interop

Rust and C++ share a single global memory allocator to enable zero-copy FFI without double-freeing or garbage collection bottlenecks.

```
       [ Rust Allocations ]                  [ JSC Allocations ]
                 \                                  /
                  \                                /
                   v                              v
            [ mimalloc global allocator (libbun_rust.a) ]
```

* **Global Allocation Override:** When ASAN is disabled, Rust registers `mimalloc` as its global allocator (`static ALLOC: bun_alloc::Mimalloc = bun_alloc::Mimalloc;`).
* **C Allocator Thunks (`src/bun_alloc/c_thunks.rs`):** Routing hooks are provided for C libraries (like zlib, brotli, and JSC) to delegate their memory lifecycle straight to mimalloc:
  * `mi_malloc_items`: Hooks zlib's allocation to mimalloc.
  * `mi_free_opaque` / `mi_free_ctx`: Frees opaque context buffers safely.
  * `mi_free_bytes`: Allows JavaScriptCore's `JSTypedArrayBytesDeallocator` to free typed arrays back to Rust's allocator space directly.

---

## 4. AST Parser, Transpiler, and Compilation (`src/ast` & `src/js_parser`)

Bun implements a highly optimized custom compiler front-end in Rust, written to achieve parsing speeds of millions of lines of JS/TS per second.

### The Packed `Ref` Symbol Layout
Instead of bulky structs representing AST node variables, Bun represents symbol variables using a packed `u64` struct called `Ref` (`src/ast/lib.rs`):
```rust
pub struct Ref(u64);
```
* It packs `{inner_index: u28, user: u3, tag: u2, source_index: u31}`.
* Hashing and equality comparisons are performed by masking out the 3 user bits.
* The 3 user bits are reused to store boolean side-flags (e.g., whether the symbol is in a `with` statement, or is a pure global hint) directly on the node, shrinking the AST node expression memory footprint.

### Memory Arenas & Tokenization
* **Arena Allocations (`src/ast/ast_memory_allocator.rs`):**
  To avoid the overhead of the system allocator, AST node generation relies on a thread-local memory arena (`ast_memory_allocator`). Nodes are allocated linearly in a continuous block, allowing fast bulk-freeing when compilation is done.
* **String Maps (`src/codegen/generate-string-map.ts`):**
  Bun converts lexical tokens and identifiers into static perfect hash maps (`.generated.rs`) to enable $O(1)$ keyword matching at parse time.

---

## 5. JavaScriptCore Glue Layer (`src/jsc`)

The `bun_jsc` crate manages the lifecycle of WebKit’s JavaScriptCore engine and exposes Javascript primitives to Rust.

### Memory Representation & NaN Boxing (`JSValue.rs`)
To bridge JS types efficiently to the CPU register level, Bun uses the 64-bit double NaN-boxing technique for JSValues:
* A double-precision float NaN has $2^{51}$ bit combinations that represent a non-number (NaN).
* Bun uses these extra bits to encode pointers, integers, booleans, and null/undefined values directly in a single `u64` register:
  ```rust
  #[repr(transparent)]
  pub struct JSValue(u64);
  ```

### Glue Architecture:
* **Virtual Machine Context (`VirtualMachine.rs`):**
  Maintains the global VM handle, driving JS task loop callbacks, microtasks, uncaught exceptions, and module loading scopes.
* **Garbage Collection Hooks (`GarbageCollectionController.rs`):**
  Wraps the JSC Mark-and-Sweep garbage collector, exposing API calls to force collection, trace memory footprint, and register weak references.
* **String Marshaling (`ZigString.rs` & `bun_string_jsc.rs`):**
  Exposes the `ZigString` C-ABI representation, enabling fast UTF-8 / UTF-16 string conversion between Javascript objects and Rust strings.

---

## 6. Database & SQL Drivers (`src/sql` & `src/sql_jsc`)

Bun embeds direct support for PostgreSQL and MySQL engines in the runtime, bypassing standard Node.js driver layers.

### The Manual VTable (`SqlRuntimeHooks`)
To break Cargo dependency cycles between `bun_runtime` (which manages the global state) and `bun_sql_jsc` (which compiles the JS bindings), Bun uses a manual cold-path vtable:
```rust
pub struct SqlRuntimeHooks {
    pub sql_rare: unsafe fn(*mut VirtualMachine) -> *mut RareData,
    pub timer_heap: unsafe fn(*mut VirtualMachine) -> *mut c_void,
    pub timer_insert: unsafe fn(heap: *mut c_void, *mut EventLoopTimer),
    ...
}
```
The single static instance `__BUN_SQL_RUNTIME_HOOKS` is defined as a `#[no_mangle]` symbol in `bun_runtime` and link-time resolved in `bun_sql_jsc`, allowing clean division of compiler tiers.

### SQL Socket & TLS integration:
* **SSL Cryptography Integration (`BoringSSL.ERR_toJS`):**
  Database clients require SSL context configurations. Bun maps client socket TLS errors to JS Error objects by checking BoringSSL's error queues directly (`bun_boringssl_sys::ERR_get_error`).
* **Socket Groups (`bun_uws::SocketGroup`):**
  SQL connections are managed using asynchronous socket groups compiled via `uWebSockets`. This lets SQL queries participate directly in the fast epoll/kqueue network loop.

---

## 7. Fast FFI Engine & TinyCC JIT (`src/runtime/ffi`)

Bun's FFI engine is designed to run dynamic library symbols at near-native speeds.

* **Dynamic Code Compilation (`FFIObject.rs`):**
  When loading a library via `bun:ffi`, Bun uses JIT code-generation to avoid context-switching overheads.
* **TinyCC Bridge (`libtcc1.c`):**
  Bun embeds TinyCC (Tiny C Compiler) to compile C code templates dynamically at runtime. This generates thin wrappers around C functions that match JavaScriptCore calling conventions, letting JSC execute dynamic library calls with direct CPU register jumps.
* **Fast and Slow Call Paths:**
  JSC identifies JIT-compiled FFI functions and executes them via the "Fast Path" (direct CPU register mappings). If arguments mismatch or type checks fail, it falls back to the "Slow Path" (the generic FFI interpreter).

---

## 8. Valkey (Redis-compatible) Engine (`src/valkey`)

Bun includes native support for Valkey/Redis databases, running a custom RESP parser in Rust.

* **Hybrid RESP2 & RESP3 Parser (`valkey_protocol.rs`):**
  Instead of limiting connections to the older RESP2 protocol, Bun’s Rust client natively understands RESP3 types, including big numbers, sets, pushes, and inline maps:
  ```rust
  pub enum RESPValue {
      SimpleString(Box<[u8]>),
      Error(Box<[u8]>),
      Integer(i64),
      BulkString(Option<Box<[u8]>>),
      Array(Vec<RESPValue>),
      Null,
      Double(f64),
      Boolean(bool),
      BlobError(Box<[u8]>),
      VerbatimString(VerbatimString),
      Map(Vec<MapEntry>),
      Set(Vec<RESPValue>),
      Attribute(Attribute),
      Push(Push),
      BigNumber(Box<[u8]>),
  }
  ```
* **Box & Vec Allocations:**
  String payloads are represented as boxed slices (`Box<[u8]>`) instead of reference-counted strings, allowing automatic dropping upon loop termination without garbage collection overheads.

---

## 9. High-Performance HTTP Stack (`src/http`, `src/picohttp`)

Bun's HTTP performance relies on wrapping C-based micro-parsers with safe Rust abstractions.

* **Picohttpparser Bridge (`picohttp`):**
  Bun wraps the C library `picohttpparser` into a Rust crate. This library parses HTTP headers using SIMD vector instructions, parsing HTTP requests in single CPU passes.
* **HTTP Context Glue (`http_jsc`):**
  Exposes incoming request properties (headers, method, URL, body stream) directly to JSC JSObjects. It resolves header lookups using prefix-indexed string search rather than allocating hash maps per request.
* **Network Streams:**
  Integrates HTTP response writing directly with the shared memory allocator (`mimalloc`), enabling zero-copy writes of JS array buffers straight to TCP sockets.

---

## 10. Build Pipeline & Toolchain Linkage

Because Rust compilation links C/C++ objects, the build is orchestrated using custom scripts that link both toolchains together.

* **Advisory Cargo Config (`cargo-config.ts`):**
  Dynamically generates `.cargo/config.toml` at configure time. It resolves the absolute system paths to `clang++` and links targets to `-fuse-ld=lld` so compilers use the same linker.
* **Cargo Build Executor (`rust.ts`):**
  Invokes Cargo to build the `bun_bin` static library.
  * For release builds, LTO builds, and ASAN instrumentation, it appends `-Zbuild-std=core,alloc,std,proc_macro,panic_abort` to recompile the standard library with matching compiler flags.
  * The final output `libbun_rust.a` is merged directly with the C++ objects in the Ninja link stage.
