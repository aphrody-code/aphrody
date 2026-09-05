# Architectural Deep-Dive: Bun Rust Crates

This document provides a deep-dive analysis of the internal Rust crates within the Bun workspace (`/tmp/bun`), mapping their directories, files, algorithms, and design choices.

---

## 1. Abstract Syntax Tree & Parser (`src/ast` & `src/js_parser`)

Bun implements a highly optimized custom compiler front-end in Rust, written to achieve maximum parsing speeds (millions of lines of JS/TS per second).

```
[ JS/TS Source Code ]
        |
        v  (Lexer / Token stream)
  [ src/js_parser ]
        |
        v  (AST generation using Arena allocator)
    [ src/ast ]
```

### Key Components & Designs:
* **The Packed `Ref` (`src/ast/lib.rs`):**
  Instead of bulky structs representing AST node variables, Bun represents symbol variables using a packed `u64` struct called `Ref`:
  ```rust
  pub struct Ref(u64);
  ```
  * It packs `{inner_index: u28, user: u3, tag: u2, source_index: u31}`.
  * Hashing and equality comparisons are performed by masking out the 3 user bits.
  * The 3 user bits are reused to store boolean side-flags (e.g., whether the symbol is in a `with` statement, or is a pure global hint) directly on the node, shrinking the AST node expression memory footprint.
* **Arena Allocations (`src/ast/ast_memory_allocator.rs`):**
  To avoid the overhead of the system allocator, AST node generation relies on a thread-local memory arena (`ast_memory_allocator`). Nodes are allocated linearly in a continuous block, allowing fast bulk-freeing when compilation is done.
* **String Maps (`src/codegen/generate-string-map.ts`):**
  Bun converts lexical tokens and identifiers into static perfect hash maps (`.generated.rs`) to enable $O(1)$ keyword matching at parse time.

---

## 2. JavaScriptCore Glue Layer (`src/jsc`)

The `bun_jsc` crate is one of the largest in the repository. It manages the lifecycle of WebKit’s JavaScriptCore engine and exposes Javascript primitives to Rust.

### Memory Representation & NaN Boxing (`JSValue.rs`)
To bridge JS types efficiently to the CPU register level, Bun uses the 64-bit double NaN-boxing technique for JSValues:
* A double-precision float NaN has $2^{51}$ bit combinations that represent a non-number (NaN).
* Bun uses these extra bits to encode pointers, integers, booleans, and null/undefined values directly in a single `u64` register:
  ```rust
  #[repr(transparent)]
  pub struct JSValue(u64);
  ```

### Key Architectures:
* **Virtual Machine Context (`VirtualMachine.rs`):**
  Maintains the global VM handle, driving JS task loop callbacks, microtasks, uncaught exceptions, and module loading scopes.
* **Garbage Collection Hooks (`GarbageCollectionController.rs`):**
  Wraps the JSC Mark-and-Sweep garbage collector, exposing API calls to force collection, trace memory footprint, and register weak references.
* **String Marshaling (`ZigString.rs` & `bun_string_jsc.rs`):**
  Exposes the `ZigString` C-ABI representation, enabling fast UTF-8 / UTF-16 string conversion between Javascript objects and Rust strings.

---

## 3. Runtime Event Loop & Standard APIs (`src/runtime`)

The `bun_runtime` crate implements Web APIs and Node.js-compat APIs on top of JSC.

```
       [ JavaScript Runtime Space ]
           |                   ^
           v (Calls)           | (Callbacks)
    [ src/runtime ] <---> [ src/event_loop ]
           |                   |
           v (Syscalls)        v (Polling)
       [ mimalloc ]        [ libuv / epoll ]
```

* **Event Loop (`src/event_loop`):**
  Integrates `libuv` (and native interfaces like `epoll` on Linux, `kqueue` on macOS, and IOCP on Windows) with the JavaScriptCore task scheduler.
* **File System (`src/runtime/node/`):**
  Fast emulation of Node.js `fs` module, bypassing the typical libuv threadpool when possible using async direct syscalls (like `io_uring` or asynchronous file access).
* **Process Lifecycles:**
  Manages process signals, child process spawning (`src/spawn`), and terminal interfaces (`src/runtime/shell/`).

---

## 4. Package Manager Subsystem (`src/install`)

The package manager (`bun install`) is completely implemented in Rust under `src/install`.

* **Concurrency Model (`PackageManager.rs`):**
  Uses a multi-threaded dependency resolver that downloads registry manifests and schedules HTTP connections concurrently.
* **Registry Client (`npm.rs`):**
  Direct HTTP client leveraging optimized compression checks. It queries metadata from npm-compatible registries and parses JSON payloads with high speed.
* **Locker & Extractor (`lockfile.rs` & `extract_tarball.rs`):**
  Reads and writes Bun's binary lockfile, and extracts `.tar.gz` package archives in parallel directly into `node_modules` without temporary files.

---

## 5. WebSockets & Cryptography (`src/uws`, `src/boringssl`)

Bun incorporates native high-performance libraries directly as Rust dependencies:

* **uWebSockets (`src/uws` & `src/uws_sys`):**
  Links the high-performance C++ `uWebSockets` library into Rust, exposing low-latency WebSockets server/client structures (`uws_dispatch.rs`).
* **BoringSSL (`src/boringssl` & `src/boringssl_sys`):**
  Statically builds Google’s `BoringSSL` library into the binary to handle TLS handshakes and SHA/crypto operations with minimal latency.
