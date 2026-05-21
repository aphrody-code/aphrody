// SPDX-License-Identifier: Apache-2.0
//! `aphrody image …` — Gemini (Nano Banana) image generation to disk.
//!
//! Backed by the `aphrody-images` crate (which builds on the `gemini-web`
//! Nano Banana SDK): generate from a prompt with the signed-in Google cookie
//! jar (no API key), download the result concurrently, and save it to disk with
//! the correct extension. Opt-in (`--features images`): links reqwest + the
//! gemini-web stack (host-only).

use std::path::PathBuf;

use aphrody_images::{generate_single, GenerateOptions, ImageClient};

/// Actions for the `image` subcommand.
#[derive(clap::Subcommand, Debug, Clone)]
pub(crate) enum ImageAction {
    /// Generate an image from a prompt and save it to a directory.
    ///
    /// Example: aphrody image generate "a banana mascot chef" --out ./out
    Generate {
        /// The image-generation prompt.
        prompt: String,
        /// Output directory (created if absent; default: current dir).
        #[arg(long, default_value = ".")]
        out: PathBuf,
        /// Optional model id (e.g. `flash`, `pro`); default = account default.
        #[arg(long)]
        model: Option<String>,
        /// Optional aspect ratio (e.g. `1:1`, `16:9`, `9:16`).
        #[arg(long)]
        aspect: Option<String>,
        /// Filename prefix for saved files.
        #[arg(long, default_value = "aphrody")]
        prefix: String,
        /// Emit JSON (saved paths + model + elapsed) instead of a path list.
        #[arg(long)]
        json: bool,
    },
}

/// Run an `image` action.
///
/// # Errors
///
/// Returns a `miette` report on auth / generation / IO failure.
pub(crate) async fn run(action: ImageAction) -> miette::Result<()> {
    match action {
        ImageAction::Generate { prompt, out, model, aspect, prefix, json } => {
            std::fs::create_dir_all(&out).map_err(|e| miette::miette!("create out dir: {e}"))?;

            let mut builder = GenerateOptions::builder();
            if let Some(m) = model {
                builder = builder.model(m);
            }
            if let Some(a) = aspect {
                let ar = aphrody_images::AspectRatio::parse(&a)
                    .map_err(|e| miette::miette!("invalid --aspect: {e}"))?;
                builder = builder.aspect_ratio(ar);
            }
            let opts = builder.build();

            let client = ImageClient::from_default_cookies("en")
                .await
                .map_err(|e| miette::miette!("gemini image connect: {e}"))?;
            let result = generate_single(&client, &prompt, &opts)
                .await
                .map_err(|e| miette::miette!("image generation: {e}"))?;

            let mut saved = Vec::with_capacity(result.images.len());
            for img in &result.images {
                let s = img
                    .save_to_dir(&out, &prefix)
                    .await
                    .map_err(|e| miette::miette!("save image: {e}"))?;
                saved.push(s);
            }

            if json {
                let payload = serde_json::json!({
                    "model_used": result.model_used,
                    "elapsed_ms": result.elapsed_ms,
                    "saved": saved.iter().map(|s| s.path.display().to_string()).collect::<Vec<_>>(),
                });
                println!(
                    "{}",
                    serde_json::to_string_pretty(&payload)
                        .map_err(|e| miette::miette!("encode: {e}"))?
                );
            } else if saved.is_empty() {
                println!("(no image returned for prompt {prompt:?})");
            } else {
                println!("model {} · {} ms", result.model_used, result.elapsed_ms);
                for s in &saved {
                    println!("{}", s.path.display());
                }
            }
            Ok(())
        },
    }
}
