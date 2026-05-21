// SPDX-License-Identifier: Apache-2.0
//! Build script for `antigravity-sdk`.
//!
//! Compiles the hand-written `exa.language_server_pb` proto into Rust tonic
//! client + prost message types, **only** when the `grpc` cargo feature is
//! enabled.  The default build (no `grpc`) is a no-op so that CI, wasm32, and
//! every downstream consumer that does not opt in stay free of `protoc` and
//! the gRPC dependency tree.
//!
//! Generated code is written to `OUT_DIR` (never the source tree) and included
//! by `src/local_ls.rs` via `include!`.

fn main() {
    // The whole module is gated behind `grpc`; without it there is nothing to
    // build.  We still emit the rerun directive unconditionally so that
    // toggling the feature triggers a rebuild.
    println!("cargo:rerun-if-changed=proto/exa_language_server.proto");
    println!("cargo:rerun-if-changed=build.rs");

    if std::env::var_os("CARGO_FEATURE_GRPC").is_none() {
        return;
    }

    #[cfg(feature = "grpc")]
    compile_protos();
}

#[cfg(feature = "grpc")]
fn compile_protos() {
    // tonic-prost-build shells out to `protoc`.  Use the vendored binary unless
    // the environment already points at one, mirroring `crates/a2a-pb/build.rs`.
    if std::env::var_os("PROTOC").is_none()
        && let Ok(protoc_path) = protoc_bin_vendored::protoc_bin_path()
    {
        // SAFETY: build scripts are single-threaded; setting an env var for the
        // child `protoc` invocation is the documented tonic-prost-build pattern.
        unsafe {
            #[allow(clippy::disallowed_methods)]
            std::env::set_var("PROTOC", protoc_path);
        }
    }

    tonic_prost_build::configure()
        .build_server(false)
        .build_client(true)
        .compile_protos(
            &["proto/exa_language_server.proto"],
            &["proto"],
        )
        .expect("failed to compile exa_language_server.proto");
}
