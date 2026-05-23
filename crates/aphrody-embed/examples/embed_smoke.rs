// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 aphrody contributors
//
// Standalone real-embedding smoke (no test-harness timeout).
//
// Run with:
//   cargo run -p aphrody-embed --features embeddings --example embed_smoke
//
// On Windows with aphrody's `+crt-static` posture the ONNX Runtime is loaded
// dynamically; point `ORT_DYLIB_PATH` at an `onnxruntime.dll` (e.g. the one
// shipped in the Microsoft.ML.OnnxRuntime NuGet package) if it is not already
// on the loader path. The first run downloads the model from HuggingFace into
// a temp cache; later runs reuse it.
//
// This binary embeds two short texts with the default model, prints the
// dimension and a preview of the first vector, asserts the dimension and a
// non-zero norm, and exits 0 on success / non-zero on failure.

#[cfg(not(all(feature = "embeddings", not(target_arch = "wasm32"))))]
fn main() {
    eprintln!(
        "embed_smoke requires `--features embeddings` on a non-wasm target; \
         this build cannot compute embeddings (aphrody_embed::is_available() == false)."
    );
    std::process::exit(2);
}

#[cfg(all(feature = "embeddings", not(target_arch = "wasm32")))]
fn main() -> aphrody_embed::Result<()> {
    use aphrody_embed::{Embedder, EmbeddingModelKind};

    // Install the rustls ring provider before any rustls client (hf-hub).
    let _ = rustls::crypto::ring::default_provider().install_default();

    let model = EmbeddingModelKind::default(); // bge-small-en-v1.5, 384 dims
    eprintln!("loading model {model} (dim={}) ...", model.dimension());

    let mut embedder = Embedder::new_with_progress(model)?;
    let dim = embedder.dimension();
    println!("model            = {}", embedder.model());
    println!("dimension        = {dim}");

    let texts = [
        "passage: aphrody is the ultimate cross-platform Rust CLI",
        "query: what is aphrody",
    ];
    let vectors = embedder.embed_texts(&texts)?;

    assert_eq!(vectors.len(), texts.len(), "one vector per input text");
    assert_eq!(dim, 384, "default model is 384-dimensional");

    for (i, v) in vectors.iter().enumerate() {
        assert_eq!(v.len(), dim, "vector {i} has the model dimension");
        let norm: f32 = v.iter().map(|x| x * x).sum::<f32>().sqrt();
        assert!(norm > 0.0, "vector {i} must be non-zero (norm={norm})");
        assert!(v.iter().all(|x| x.is_finite()), "vector {i} has NaN/Inf");
        println!(
            "vec[{i}] len={} norm={norm:.4} head={:?}",
            v.len(),
            &v[..v.len().min(4)]
        );
    }
    assert_ne!(vectors[0], vectors[1], "distinct texts -> distinct vectors");

    println!("SMOKE OK: embedded {} texts into {dim}-d vectors", texts.len());
    Ok(())
}
