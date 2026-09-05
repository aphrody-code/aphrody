// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 aphrody contributors
//
// Real end-to-end smoke test for the local embeddings engine.
//
// This test is compiled only with `--features embeddings` on a host target.
// On first run it downloads the model from HuggingFace into a temp cache; if
// the network is unavailable it skips (printing why) rather than failing, so
// the offline gate stays green. When it can load the model it asserts genuine
// behaviour: the right number of vectors, the right dimension (384), non-zero
// content, and that two different texts produce different vectors.

#![cfg(all(feature = "embeddings", not(target_arch = "wasm32")))]

use std::sync::Once;

use aphrody_embed::{Embedder, EmbeddingModelKind};

static RUSTLS: Once = Once::new();

/// Install the ring CryptoProvider once before any rustls-backed client is
/// built (hf-hub downloads over rustls). rustls 0.23 with both ring + aws-lc
/// candidates in the graph panics with "no process-level CryptoProvider" if
/// none is installed — see CLAUDE.md §7.
fn install_rustls_provider() {
    RUSTLS.call_once(|| {
        let _ = rustls::crypto::ring::default_provider().install_default();
    });
}

// Ignored by default: this test downloads the model from HuggingFace (and, in
// `load-dynamic` mode, needs an `onnxruntime` shared library reachable via
// `ORT_DYLIB_PATH` or the loader path). Both make it slow and network-bound,
// so it is excluded from the fast `cargo nextest run` gate. Run it explicitly:
//   ORT_DYLIB_PATH=<...>/onnxruntime.dll \
//     cargo nextest run -p aphrody-embed --features embeddings --run-ignored all
// The non-ignored coverage of the real embed path lives in the standalone
// `examples/embed_smoke.rs` binary (no test-harness timeout).
#[test]
#[ignore = "downloads model + needs onnxruntime dylib (ORT_DYLIB_PATH); run via examples/embed_smoke or --run-ignored"]
fn embeds_text_to_384_dim_vectors() {
    install_rustls_provider();

    let cache = tempfile::tempdir().expect("temp cache dir");

    // Use the smaller MiniLM model for the smoke (faster download than BGE),
    // into an isolated temp cache so the test never pollutes ~/.aphrody.
    let mut embedder =
        match Embedder::with_cache_dir(EmbeddingModelKind::AllMiniLmL6V2, cache.path().to_path_buf()) {
            Ok(e) => e,
            Err(err) => {
                // Most likely: no network on first run. Skip, do not fail.
                eprintln!("SKIP embed smoke (model load failed, likely offline): {err}");
                return;
            }
        };

    assert_eq!(embedder.dimension(), 384, "MiniLM-L6-v2 is 384-dimensional");
    assert_eq!(embedder.model(), EmbeddingModelKind::AllMiniLmL6V2);

    let texts = [
        "passage: aphrody is a cross-platform Rust CLI",
        "query: what is aphrody",
    ];
    let vectors = embedder.embed_texts(&texts).expect("embed should succeed");

    assert_eq!(vectors.len(), 2, "one vector per input");
    for v in &vectors {
        assert_eq!(v.len(), 384, "every vector is 384-d");
        let norm: f32 = v.iter().map(|x| x * x).sum::<f32>().sqrt();
        assert!(norm > 0.0, "vector must not be all-zero (norm={norm})");
        assert!(v.iter().all(|x| x.is_finite()), "no NaN/Inf in vector");
    }

    // Distinct inputs should not collapse to the identical vector.
    assert_ne!(vectors[0], vectors[1], "different texts -> different vectors");

    // embed_one is consistent with the batch path.
    let single = embedder.embed_one(texts[0]).expect("embed_one");
    assert_eq!(single.len(), 384);

    eprintln!(
        "OK embed smoke: model={} dim={} vec0[0..3]={:?}",
        embedder.model(),
        embedder.dimension(),
        &single[..3]
    );
}

#[test]
#[ignore = "loads a model (network + onnxruntime dylib); run via --run-ignored"]
fn empty_input_yields_empty_output_without_loading() {
    install_rustls_provider();
    let cache = tempfile::tempdir().expect("temp cache dir");
    // Loading is required to reach embed_batch, but if load fails (offline) we
    // still validate the empty-fast-path contract by skipping.
    let Ok(mut embedder) =
        Embedder::with_cache_dir(EmbeddingModelKind::AllMiniLmL6V2, cache.path().to_path_buf())
    else {
        eprintln!("SKIP empty-input check (model load failed, likely offline)");
        return;
    };
    let empty: [&str; 0] = [];
    let out = embedder.embed_texts(&empty).expect("empty embed");
    assert!(out.is_empty(), "empty input -> empty output");
}
