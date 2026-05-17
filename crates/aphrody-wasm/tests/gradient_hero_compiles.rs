// SPDX-License-Identifier: Apache-2.0
//
// Compile-time visibility test for the `gradient_hero` module.
//
// The `GradientHero` struct and its `GradientError` type live inside a
// `#![cfg(target_arch = "wasm32")]` file, so they cannot be instantiated
// in a native test runner.  This file therefore has two sections:
//
//   - `wasm32` target  : a `wasm_bindgen_test` that verifies the re-exported
//     type is accessible from the crate's public surface.
//
//   - native (non-wasm32) : a trivial marker test that documents why the
//     module is absent and keeps `nextest` happy (a test file with zero
//     test functions emits a linker warning on some hosts).
//
// Run the wasm32 variant:
//   wasm-pack test --headless --firefox crates/aphrody-wasm
//   cargo test --target wasm32-unknown-unknown -p aphrody-wasm

// ---------------------------------------------------------------------------
// wasm32 path — real type-visibility assertions
// ---------------------------------------------------------------------------
#[cfg(target_arch = "wasm32")]
mod wasm32_tests {
    use wasm_bindgen_test::*;

    wasm_bindgen_test_configure!(run_in_browser);

    /// Verify that `GradientHero` is accessible via the crate's public API.
    ///
    /// This test does NOT call `GradientHero::mount` (which requires a live
    /// browser GPU context with a real `<canvas>` element), but it confirms
    /// that the type is exported and that the module compiles correctly for
    /// the wasm32 target — which is the primary compile gate for this crate.
    #[wasm_bindgen_test]
    fn gradient_hero_type_is_accessible() {
        // `GradientHero` is re-exported at crate root; if this compiles and
        // links, the module structure and wasm_bindgen annotations are correct.
        // We use `core::any::type_name` so there is no runtime GPU call.
        let name = core::any::type_name::<aphrody_wasm::GradientHero>();
        assert!(
            name.contains("GradientHero"),
            "type_name should contain 'GradientHero', got: {name}",
        );
    }

    /// Verify that `GradientError::CanvasNotFound` formats as expected.
    ///
    /// This exercises the `Display` impl without touching the GPU — safe to
    /// run in any wasm32 environment (browser or WASI).
    #[wasm_bindgen_test]
    fn gradient_error_display_canvas_not_found() {
        use aphrody_wasm::gradient_hero::GradientError;
        let msg = GradientError::CanvasNotFound("my-canvas".to_owned()).to_string();
        assert!(
            msg.contains("my-canvas"),
            "error message should contain canvas id, got: {msg}",
        );
        assert!(
            msg.contains("not found"),
            "error message should mention 'not found', got: {msg}",
        );
    }

    /// Verify that `GradientError::WebGpuUnavailable` formats without panicking.
    #[wasm_bindgen_test]
    fn gradient_error_display_webgpu_unavailable() {
        use aphrody_wasm::gradient_hero::GradientError;
        let msg = GradientError::WebGpuUnavailable.to_string();
        assert!(!msg.is_empty(), "error message must be non-empty");
    }
}

// ---------------------------------------------------------------------------
// Native (non-wasm32) path — marker test
// ---------------------------------------------------------------------------
#[cfg(not(target_arch = "wasm32"))]
mod native_tests {
    /// Documents that the `gradient_hero` module is wasm32-only.
    ///
    /// `GradientHero` uses browser APIs (`navigator.gpu`, canvas, wgpu WebGPU
    /// backend) that have no native equivalent.  The module is gated with
    /// `#![cfg(target_arch = "wasm32")]` so it is entirely absent from native
    /// builds — attempting to instantiate it here would fail to compile.
    ///
    /// The workspace CI (`cargo ci-offline`) runs the native test suite;
    /// wasm32 tests are run separately via `wasm-pack test --headless`.
    #[test]
    fn gradient_hero_is_wasm32_only() {
        // Nothing to assert — this function exists so the file has at least
        // one test item visible to the native test harness.
    }
}
