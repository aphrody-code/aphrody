// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 aphrody contributors
//
// Build script: reconcile the prebuilt ONNX Runtime with the MSVC link.
//
// aphrody forces a STATIC C runtime on Windows (`-C target-feature=+crt-static`
// in `.cargo/config.toml`, a deliberate hardening choice so the `aphrody`
// binary has no MSVC runtime-DLL dependency). The prebuilt ONNX Runtime that
// `ort`'s `download-binaries` strategy fetches is, however, built against the
// DYNAMIC CRT (`/MD`). Statically linking it under `+crt-static` produces an
// unresolvable mix of `__imp_*` (dllimport) CRT externals and duplicate-symbol
// conflicts (`libcmt` vs `ucrt`).
//
// We therefore drive `ort` in `load-dynamic` mode (see Cargo.toml: the
// `ort-load-dynamic` fastembed feature, which sets `ort-sys/disable-linking`).
// No ONNX Runtime object code is linked into the aphrody binary; instead the
// `onnxruntime` shared library is loaded at runtime via `libloading`, carrying
// its own CRT. This sidesteps the static/dynamic CRT clash entirely while
// keeping aphrody's `+crt-static` posture for its own code intact, and is the
// strategy `ort` itself recommends for awkward link environments.
//
// The shared library is located at runtime; `ort` searches (in order) the
// `ORT_DYLIB_PATH` environment variable, then the executable's directory, then
// the system loader path. The companion `download.rs`/docs note how to obtain
// it offline. This script has nothing to emit for the link itself — it only
// documents the contract and re-runs when its inputs change.

fn main() {
    // Surface the chosen strategy as build metadata (visible with `-vv`); no
    // link directives are required because ONNX Runtime linking is disabled.
    if std::env::var_os("CARGO_FEATURE_EMBEDDINGS").is_some() {
        println!(
            "cargo:warning=aphrody-embed: ONNX Runtime in load-dynamic mode \
             (set ORT_DYLIB_PATH to the onnxruntime shared library, or place it \
             beside the executable)."
        );
    }

    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-env-changed=CARGO_FEATURE_EMBEDDINGS");
}
