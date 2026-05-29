# Architectural Deep-Dive 2: Advanced Subsystems of Bun

This document continues the deep-dive analysis of the internal Rust crates within the Bun workspace (`/tmp/bun`), detailing the Database Engines, the FFI JIT, and Socket/Event-Loop boundaries.

---

## 1. Database & SQL Drivers (`src/sql` & `src/sql_jsc`)

Bun embeds direct support for PostgreSQL and MySQL engines in the runtime, bypassing standard Node.js driver layers.

```
       [ JavaScript Application ]
                   |
                   v (JS Call)
         [ bun_sql_jsc::jsc ]
                   |
        +----------+----------+
        |                     |
        v                     v
   [ mysql/ ]            [ postgres/ ]
        |                     |
        +----------+----------+
                   |
                   v
           [ uWebSockets / TLS ]
```

### Key Architectures & Designs:
* **The Manual VTable (`SqlRuntimeHooks`):**
  A classic problem in large monorepos is circular dependency loops. `bun_runtime` (which manages the global state) needs to call `bun_sql_jsc` (which compiles the JS bindings), but `bun_sql_jsc` also needs to access the global event loops and timers inside `bun_runtime`.
  To break this cargo dependency cycle, Bun uses a manual cold-path vtable:
  ```rust
  pub struct SqlRuntimeHooks {
      pub sql_rare: unsafe fn(*mut VirtualMachine) -> *mut RareData,
      pub timer_heap: unsafe fn(*mut VirtualMachine) -> *mut c_void,
      pub timer_insert: unsafe fn(heap: *mut c_void, *mut EventLoopTimer),
      ...
  }
  ```
  The single static instance `__BUN_SQL_RUNTIME_HOOKS` is defined as a `#[no_mangle]` symbol in `bun_runtime` and link-time resolved in `bun_sql_jsc`, allowing clean division of compiler tiers.
* **SSL Cryptography Integration (`BoringSSL.ERR_toJS`):**
  Database clients require SSL context configurations. Bun maps client socket TLS errors to JS Error objects by checking BoringSSL's error queues directly (`bun_boringssl_sys::ERR_get_error`).
* **Socket Groups (`bun_uws::SocketGroup`):**
  SQL connections are managed using asynchronous socket groups compiled via `uWebSockets`. This lets SQL queries participate directly in the fast epoll/kqueue network loop.

---

## 2. Fast FFI Engine & TinyCC JIT (`src/runtime/ffi`)

Bun's FFI engine is designed to run dynamic library symbols at near-native speeds.

* **Dynamic Code Compilation (`FFIObject.rs`):**
  When loading a library via `bun:ffi`, Bun uses JIT code-generation to avoid context-switching overheads.
* **TinyCC Bridge (`libtcc1.c`):**
  Bun embeds TinyCC (Tiny C Compiler) to compile C code templates dynamically at runtime. This generates thin wrappers around C functions that match JavaScriptCore calling conventions, letting JSC execute dynamic library calls with a direct jumps rather than through generic marshal loops.
* **Fast and Slow Call Paths:**
  JSC identifies JIT-compiled FFI functions and executes them via the "Fast Path" (direct CPU register mappings). If arguments mismatch or type checks fail, it falls back to the "Slow Path" (the generic FFI interpreter).

---

## 3. Network Sockets & Loop Dispatch (`src/event_loop`, `src/uws`)

Bun's performance is heavily derived from its optimized event loop.

* **Unified Polling Loop:**
  Instead of spawning separate threads or polling lists for timers and sockets, Bun integrates the `libuv` loop structure directly with the `uWebSockets` epoll descriptor list. Sockets, timers, files, and JS microtasks are checked in a single pass of the main thread.
* **Socket Context Groups:**
  Each `VirtualMachine` context manages socket context groups for TLS and raw TCP. This guarantees that socket connections are cleaned up safely during VM termination callbacks (`Bun__onExit`).
