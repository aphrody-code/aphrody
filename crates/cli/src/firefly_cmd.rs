// SPDX-License-Identifier: Apache-2.0
//! `aphrody firefly …` — Adobe Firefly Services image generation to disk.
//!
//! Backed by the `aphrody-firefly` crate: OAuth server-to-server (IMS) auth,
//! the Firefly v3 async generate endpoint, job polling, and concurrent output
//! download. Opt-in (`--features firefly`); host-only (links reqwest/rustls).
//!
//! Credentials come from the environment (never the CLI, to keep secrets out of
//! shell history): `FIREFLY_CLIENT_ID` and `FIREFLY_CLIENT_SECRET`.

use std::path::PathBuf;

use aphrody_firefly::{
    ContentClass, FireflyClient, GenerateImageRequest, JournalClient, Position, Size,
};

/// Actions for the `firefly` subcommand.
#[derive(clap::Subcommand, Debug, Clone)]
pub(crate) enum FireflyAction {
    /// Generate an image from a prompt and save it to a directory.
    ///
    /// Example: aphrody firefly generate "a realistic cat coding" --out ./out
    Generate {
        /// The image-generation prompt.
        prompt: String,
        /// Output directory (created if absent; default: current dir).
        #[arg(long, default_value = ".")]
        out: PathBuf,
        /// Number of variations (1–4).
        #[arg(long, default_value_t = 1)]
        variations: u8,
        /// Output size as `WxH` (e.g. `2048x2048`, `2688x1536`).
        #[arg(long)]
        size: Option<String>,
        /// Content class: `auto` (default), `photo`, or `art`.
        #[arg(long, default_value = "auto")]
        content_class: String,
        /// Concepts to avoid (negative prompt).
        #[arg(long)]
        negative: Option<String>,
        /// BCP-47 locale to bias the prompt (e.g. `en-US`, `fr-FR`).
        #[arg(long)]
        locale: Option<String>,
        /// Filename prefix for saved files.
        #[arg(long, default_value = "firefly")]
        prefix: String,
        /// Emit JSON (saved paths + seeds) instead of a path list.
        #[arg(long)]
        json: bool,
    },
    /// Drain the Adobe I/O Events journal (event-driven job completion).
    ///
    /// Reads FIREFLY_JOURNAL_URL + FIREFLY_IMS_ORG_ID (+ FIREFLY_CLIENT_ID/SECRET)
    /// from the environment. Example: aphrody firefly events --latest
    Events {
        /// Start from the latest position (only new events) instead of oldest.
        #[arg(long)]
        latest: bool,
        /// Resume strictly after this journal position.
        #[arg(long)]
        since: Option<String>,
        /// Maximum HTTP round-trips when draining the journal.
        #[arg(long, default_value_t = 20)]
        max_batches: u32,
        /// Emit JSON (events + next position) instead of a summary.
        #[arg(long)]
        json: bool,
    },
}

/// Run a `firefly` action.
///
/// # Errors
///
/// Returns a `miette` report on credential / auth / generation / IO failure.
pub(crate) async fn run(action: FireflyAction) -> miette::Result<()> {
    match action {
        FireflyAction::Generate {
            prompt,
            out,
            variations,
            size,
            content_class,
            negative,
            locale,
            prefix,
            json,
        } => {
            let class = ContentClass::parse(&content_class)
                .ok_or_else(|| miette::miette!("invalid --content-class (use auto|photo|art)"))?;

            let mut req = GenerateImageRequest::new(prompt.clone())
                .with_variations(variations)
                .with_content_class(class);
            if let Some(s) = size {
                let parsed =
                    Size::parse(&s).ok_or_else(|| miette::miette!("invalid --size (use WxH)"))?;
                req = req.with_size(parsed);
            }
            if let Some(neg) = negative {
                req = req.with_negative_prompt(neg);
            }
            if let Some(loc) = locale {
                req = req.with_locale(loc);
            }

            let client = FireflyClient::from_env()
                .map_err(|e| miette::miette!("Firefly auth setup: {e}"))?;
            let images = client
                .generate_and_download(&req)
                .await
                .map_err(|e| miette::miette!("Firefly generation: {e}"))?;

            let mut saved = Vec::with_capacity(images.len());
            for img in &images {
                let path = img
                    .save_to_dir(&out, &prefix)
                    .await
                    .map_err(|e| miette::miette!("save image: {e}"))?;
                saved.push((path, img.seed));
            }

            if json {
                let payload = serde_json::json!({
                    "prompt": prompt,
                    "count": saved.len(),
                    "outputs": saved.iter().map(|(p, seed)| serde_json::json!({
                        "path": p.display().to_string(),
                        "seed": seed,
                    })).collect::<Vec<_>>(),
                });
                println!(
                    "{}",
                    serde_json::to_string_pretty(&payload)
                        .map_err(|e| miette::miette!("encode: {e}"))?
                );
            } else if saved.is_empty() {
                println!("(no image returned for prompt {prompt:?})");
            } else {
                println!("Firefly · {} variation(s)", saved.len());
                for (path, seed) in &saved {
                    match seed {
                        Some(s) => println!("{} (seed {s})", path.display()),
                        None => println!("{}", path.display()),
                    }
                }
            }
            Ok(())
        },

        FireflyAction::Events { latest, since, max_batches, json } => {
            let start = if let Some(pos) = since {
                Position::Since(pos)
            } else if latest {
                Position::Latest
            } else {
                Position::Oldest
            };

            let client = JournalClient::from_env()
                .map_err(|e| miette::miette!("Firefly journaling setup: {e}"))?;
            let (events, next) = client
                .drain(start, max_batches)
                .await
                .map_err(|e| miette::miette!("journal read: {e}"))?;

            if json {
                let payload = serde_json::json!({
                    "count": events.len(),
                    "next": next,
                    "events": events.iter().map(|e| serde_json::json!({
                        "position": e.position,
                        "event": e.event,
                    })).collect::<Vec<_>>(),
                });
                println!(
                    "{}",
                    serde_json::to_string_pretty(&payload)
                        .map_err(|e| miette::miette!("encode: {e}"))?
                );
            } else if events.is_empty() {
                println!("(journal caught up — no new events)");
            } else {
                println!("{} event(s)", events.len());
                for e in &events {
                    let kind = e.event.get("type").and_then(|v| v.as_str()).unwrap_or("event");
                    println!("{}  {kind}", e.position);
                }
                if let Some(n) = next {
                    println!("next: {n}");
                }
            }
            Ok(())
        },
    }
}
