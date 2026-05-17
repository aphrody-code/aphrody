# wasm-bindgen Performance Optimization Report

Branch: `perf` (from `main` @ 7a7c403)

---

## Hotspot 1 — `write_class` inspectable fold
**File:** `crates/cli-support/src/js/mod.rs:1656`
**Commit:** `5a81fba`

The `readable_properties.fold(String::from("\n"), |fields, field_name| format!("{fields}{field_name}: this.{field_name},\n"))` pattern allocates a brand-new `String` on every iteration because `format!` always allocates. For a struct with N inspectable properties this is O(N) heap allocations in a tight loop.

**Fix:** Replace with a single pre-sized `String::with_capacity` + `push_str` loop. Also includes a rewrite of `format_doc_comments` (same file, line 5773) to use one pre-sized buffer instead of two intermediate `String`s plus a final `format!`.

---

## Hotspot 2 — Name-building helpers in `shared`
**File:** `crates/shared/src/lib.rs:224–273`
**Commit:** `6a589a1`

`new_function`, `free_function`, `unwrap_function`, `struct_function_export_name`, `struct_field_get`, `struct_field_set` all start with `"__wbg_".to_string()` or `String::from("...")` without a capacity hint. Since the total length is immediately knowable from the arguments, the internal `String` buffer grows at least once (doubling from the initial default capacity). These functions are called for every exported struct, method, and field, making them a hot path during codegen.

**Fix:** `String::with_capacity(prefix_len + struct_name.len() + suffix_len)` pre-allocates the exact upper-bound size, eliminating all reallocation. Also converted `struct_function_export_name` from `.collect::<String>()` to `extend()` to reuse the pre-allocated buffer.

---

## Benchmarks

The `benchmarks/` and `benches/` suites compile to wasm32 and execute in a browser via `wasm_bindgen_test`. No host-runnable Criterion benchmarks exist for the codegen path (`cargo bench -- --list` → `0 tests, 0 benchmarks`). A directional proxy is `cargo check -p wasm-bindgen-cli-support --release` (~18s on this VPS), but that measures compile time of the crate, not runtime codegen.

These optimizations are allocation-count reductions on hot codegen paths. The impact scales with the number of exported structs/fields per crate — typical crates with many exported types benefit most.

---

## Verdict

wasm-bindgen's codegen is generally well-optimized (`Cow<'static, str>` for intrinsics, `BTreeMap`/`HashSet` deduplication, guard flags). The two genuine hotspots found are:
1. The inspectable-class field accumulation (quadratic allocation pattern)
2. The shared name-building helpers (missing `with_capacity` on frequently-called functions)
