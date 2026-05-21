// SPDX-License-Identifier: Apache-2.0
//! `aphrody-design-sidecar` CLI.
//!
//! Subcommands:
//!
//! * `parse <file|->` — stream artifacts from a file or stdin and print
//!   each one as a JSON line on stdout.
//! * `digest <dir>` — recursively read every file under `dir`, treat each
//!   as one artifact (the file name is the id, the extension picks the
//!   format), and emit the manifest.
//! * `validate <manifest.json>` — read an existing manifest and rerun the
//!   cross-row checks.

use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use aphrody_design::sidecar::adapters::{ArtifactFormat, adapter_for};
use aphrody_design::sidecar::digest::{digest_artifact, digest_manifest};
use aphrody_design::sidecar::parser::Artifact;
use aphrody_design::sidecar::pipeline::Pipeline;
use aphrody_design::sidecar::validate::{validate_artifact, validate_manifest};
use clap::{Parser, Subcommand};
use tokio::io::AsyncReadExt;

#[derive(Parser)]
#[command(name = "aphrody-design-sidecar")]
#[command(about = "Streaming <artifact> pipeline: parse / digest / validate.")]
struct Cli {
    #[command(subcommand)]
    command: Cmd,
}

#[derive(Subcommand)]
enum Cmd {
    /// Parse `<artifact>` blocks from a file (use `-` for stdin) and emit
    /// one JSON object per artifact on stdout (NDJSON).
    Parse {
        /// Path to read from. `-` means stdin.
        path: String,
    },
    /// Walk `<dir>` and treat each file as one artifact. The file name
    /// (without extension) is the id; the extension picks the format.
    /// Emits the manifest as a single JSON object on stdout.
    Digest {
        /// Directory to scan.
        dir: PathBuf,
    },
    /// Read an existing manifest from `<path>` and rerun the cross-row
    /// schema checks. Exits non-zero if validation fails.
    Validate {
        /// Manifest path.
        path: PathBuf,
    },
}

#[tokio::main(flavor = "multi_thread", worker_threads = 2)]
async fn main() -> Result<()> {
    tracing::subscriber::set_global_default(
        tracing_subscriber_fallback::FallbackSubscriber::default(),
    )
    .ok();

    let cli = Cli::parse();
    match cli.command {
        Cmd::Parse { path } => run_parse(&path).await,
        Cmd::Digest { dir } => run_digest(&dir),
        Cmd::Validate { path } => run_validate(&path),
    }
}

async fn run_parse(path: &str) -> Result<()> {
    let mut text = String::new();
    if path == "-" {
        tokio::io::stdin()
            .read_to_string(&mut text)
            .await
            .context("read stdin")?;
    } else {
        text = std::fs::read_to_string(path).with_context(|| format!("read {path}"))?;
    }

    let outcome = Pipeline::new()
        .run_sync(&text)
        .context("pipeline failed")?;

    // We re-parse to emit per-artifact JSON for the `parse` command (the
    // pipeline only returns digests). Cheap because we already have `text`.
    let mut parser = aphrody_design::sidecar::parser::ArtifactParser::new();
    let mut events = parser.feed(&text).unwrap_or_default();
    events.extend(parser.finish().unwrap_or_default());

    let mut stdout = std::io::BufWriter::new(std::io::stdout().lock());
    use std::io::Write as _;
    for ev in events {
        if let aphrody_design::sidecar::parser::ParserEvent::Artifact(a) = ev {
            let line = serde_json::to_string(&a).context("encode artifact")?;
            writeln!(stdout, "{line}")?;
        }
    }
    stdout.flush()?;

    if outcome.manifest.partial {
        eprintln!("pipeline finished in degraded mode ({} log entries)", outcome.log.len());
    }
    Ok(())
}

fn run_digest(dir: &Path) -> Result<()> {
    if !dir.is_dir() {
        bail!("not a directory: {}", dir.display());
    }
    let mut rows = Vec::new();
    let mut log = Vec::new();
    for entry in std::fs::read_dir(dir).with_context(|| format!("readdir {}", dir.display()))? {
        let entry = entry?;
        let p = entry.path();
        if !p.is_file() {
            continue;
        }
        let id = p
            .file_stem()
            .map(|s| s.to_string_lossy().into_owned())
            .unwrap_or_default();
        let ext = p
            .extension()
            .map(|s| s.to_string_lossy().into_owned())
            .unwrap_or_default();
        let Some(format) = format_from_ext(&ext) else {
            log.push(format!("skip unsupported ext: {}", p.display()));
            continue;
        };
        let bytes = std::fs::read(&p).with_context(|| format!("read {}", p.display()))?;
        let adapter = adapter_for(format);
        let normalized = match adapter.normalize(&bytes) {
            Ok(n) => n,
            Err(e) => {
                log.push(format!("normalize {}: {e}", p.display()));
                continue;
            },
        };
        let artifact = Artifact {
            id: id.clone(),
            type_: format.canonical_mime().to_string(),
            title: id.clone(),
            content: String::new(), // not used by digest
            mime: normalized.mime.clone(),
        };
        if let Err(e) = validate_artifact(&artifact, &normalized) {
            log.push(format!("validate {}: {e}", p.display()));
            continue;
        }
        rows.push(digest_artifact(&artifact, &normalized));
    }
    let manifest = digest_manifest(rows, !log.is_empty());
    validate_manifest(&manifest).context("manifest validation")?;
    let s = serde_json::to_string_pretty(&manifest)?;
    println!("{s}");
    for entry in log {
        eprintln!("{entry}");
    }
    Ok(())
}

fn run_validate(path: &Path) -> Result<()> {
    let text = std::fs::read_to_string(path).with_context(|| format!("read {}", path.display()))?;
    let manifest: aphrody_design::sidecar::digest::DigestManifest =
        serde_json::from_str(&text).context("decode manifest json")?;
    validate_manifest(&manifest).context("manifest invalid")?;
    println!(
        "{}",
        serde_json::to_string_pretty(&serde_json::json!({
            "ok": true,
            "artifacts": manifest.artifacts.len(),
            "total_sha256": manifest.total_sha256,
            "partial": manifest.partial,
        }))?
    );
    Ok(())
}

fn format_from_ext(ext: &str) -> Option<ArtifactFormat> {
    Some(match ext.to_ascii_lowercase().as_str() {
        "html" | "htm" => ArtifactFormat::Html,
        "md" | "markdown" => ArtifactFormat::Markdown,
        "pdf" => ArtifactFormat::Pdf,
        "pptx" => ArtifactFormat::Pptx,
        "zip" => ArtifactFormat::Zip,
        _ => return None,
    })
}

/// Minimal no-deps tracing subscriber fallback. Avoids pulling in
/// `tracing-subscriber` (and its 30+ transitive deps) just for the CLI's
/// startup log. Emits to stderr in `key=value` form when something fires.
mod tracing_subscriber_fallback {
    use std::sync::atomic::{AtomicUsize, Ordering};
    use tracing::{Event, Metadata, Subscriber, span};

    #[derive(Default)]
    pub(crate) struct FallbackSubscriber {
        next_id: AtomicUsize,
    }

    impl Subscriber for FallbackSubscriber {
        fn enabled(&self, _: &Metadata<'_>) -> bool { true }
        fn new_span(&self, _: &span::Attributes<'_>) -> span::Id {
            let id = self.next_id.fetch_add(1, Ordering::Relaxed) + 1;
            span::Id::from_u64(id as u64)
        }
        fn record(&self, _: &span::Id, _: &span::Record<'_>) {}
        fn record_follows_from(&self, _: &span::Id, _: &span::Id) {}
        fn event(&self, event: &Event<'_>) {
            struct Visit(String);
            impl tracing::field::Visit for Visit {
                fn record_debug(&mut self, field: &tracing::field::Field, value: &dyn std::fmt::Debug) {
                    if !self.0.is_empty() { self.0.push(' '); }
                    self.0.push_str(field.name());
                    self.0.push('=');
                    self.0.push_str(&format!("{value:?}"));
                }
            }
            let mut v = Visit(String::new());
            event.record(&mut v);
            eprintln!("[{}] {} {}", event.metadata().level(), event.metadata().target(), v.0);
        }
        fn enter(&self, _: &span::Id) {}
        fn exit(&self, _: &span::Id) {}
    }
}
