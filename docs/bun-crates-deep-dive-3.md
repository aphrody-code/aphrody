# Architectural Deep-Dive 3: Valkey, CSS Parser, and HTTP Stack of Bun

This document concludes the deep-dive series on the internal Rust crates within the Bun workspace (`/tmp/bun`), mapping the Valkey RESP parser, the CSS AST transpiler, and the high-performance HTTP request engine.

---

## 1. Valkey (Redis-compatible) Engine (`src/valkey`)

Bun includes native support for Valkey/Redis databases, running a custom RESP parser in Rust.

```
 [ JS Application (ValkeyClient) ]
                 |
                 v (Send commands)
        [ bun_valkey_protocol ]
                 |
                 +-----> Parses RESP2 / RESP3
                 |       (SimpleString, BulkString, BigNumber, Maps, Push)
                 |
                 v (Direct TCP socket write)
           [ Valkey Server ]
```

### Key Architectures & Designs:
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
* **Inline Error Mapping:**
  Maps Redis protocol errors directly to JS Exception objects via dynamic error tag macro expansions (`bun_core::impl_tag_error!`).

---

## 2. CSS AST Parser & Bundler (`src/css` & `src/css_jsc`)

Bun treats CSS as a first-class citizen of its bundler engine, processing styles alongside JS/TS files.

* **CSS Modules & Nesting:**
  Implements standard CSS nesting and CSS Module class name hashing. Hashing is performed directly in Rust at tokenization time.
* **JSC Style AST Bridge:**
  Exposes the CSS AST to JavaScriptCore (`css_jsc.rs`) so plugins or bundler callbacks can inspect style declarations, calculate selector specificity, and extract CSS variables.
* **Asset Embedding:**
  When a CSS file contains `url(...)` declarations pointing to local assets (images, fonts), the bundler intercepts it, rewrites the URL to the built target path, and copies the asset file automatically.

---

## 3. High-Performance HTTP Stack (`src/http`, `src/picohttp`)

Bun's HTTP performance relies on wrapping C-based micro-parsers with safe Rust abstractions.

* **Picohttpparser Bridge (`picohttp`):**
  Bun wraps the C library `picohttpparser` into a Rust crate. This library parses HTTP headers using SIMD vector instructions, parsing HTTP requests in single CPU passes.
* **HTTP Context Glue (`http_jsc`):**
  Exposes incoming request properties (headers, method, URL, body stream) directly to JSC JSObjects. It resolves header lookups using prefix-indexed string search rather than allocating hash maps per request.
* **Network Streams:**
  Integrates HTTP response writing directly with the shared memory allocator (`mimalloc`), enabling zero-copy writes of JS array buffers straight to TCP sockets.
