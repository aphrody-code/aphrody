// SPDX-License-Identifier: Apache-2.0
//! `aphrody ocr …` — read images with a local vision-language model.
//!
//! Built only with `--features ocr`. Drives a GGUF vision model through
//! llama.cpp (resolved by `aphrody-infer`) and turns each page into markdown,
//! or into an explicit "no text" verdict.
//!
//! # Why JSONL for batches
//!
//! `batch` streams one JSON object per line as each page finishes, flushing as
//! it goes. A run over ten thousand plates takes hours; a format that is only
//! valid once complete would mean losing everything to one interruption, and
//! would make resuming impossible. With JSONL the already-read pages are on
//! disk and `--skip-done` can pick the run back up.

use std::io::{BufRead as _, Write as _};
use std::path::{Path, PathBuf};

use aphrody_ocr::{OcrOptions, PageResult, ServerRunner, VlmRunner};

use crate::model_cmd::OutputOpts;

/// Actions for the `ocr` subcommand.
#[derive(clap::Subcommand, Debug, Clone)]
pub(crate) enum OcrAction {
    /// Read one image and print its markdown transcription.
    ///
    /// Example: aphrody ocr page plate.jpg
    Page {
        /// Image to read.
        image: PathBuf,
        /// Catalog id of the vision model.
        #[arg(long, default_value = "granite-docling-258m")]
        model: String,
        /// Override the prompt handed to the model.
        #[arg(long)]
        prompt: Option<String>,
        /// Token budget for the page.
        #[arg(long, default_value_t = 1024)]
        max_tokens: u32,
        /// Include the model's raw output.
        #[arg(long)]
        raw: bool,
        #[command(flatten)]
        output: OutputOpts,
    },
    /// Re-run the text cleanup over an existing JSONL, in place.
    ///
    /// The model output is not touched — the images are not read again. Only
    /// the parsing and filtering are redone, which is what changes when a
    /// cleanup rule is added. Use it to bring results produced before a rule
    /// existed up to the current pipeline instead of re-reading them.
    ///
    /// Example: aphrody ocr clean lot-001.jsonl --out lot-001-clean.jsonl
    Clean {
        /// JSONL produced by `aphrody ocr batch --raw`.
        input: PathBuf,
        /// Where to write the cleaned JSONL. Defaults to overwriting `input`.
        #[arg(long)]
        out: Option<PathBuf>,
    },
    /// Read every image in a directory, streaming one JSON object per line.
    ///
    /// Example: aphrody ocr batch ./lot-001/images --out lot-001.jsonl
    Batch {
        /// Directory of images.
        dir: PathBuf,
        /// JSONL file to append results to. Without it, results go to stdout.
        #[arg(long)]
        out: Option<PathBuf>,
        /// Stop after this many images.
        #[arg(long)]
        limit: Option<usize>,
        /// Skip images already present in `--out`, so an interrupted run can
        /// be resumed without redoing work.
        #[arg(long)]
        skip_done: bool,
        /// Catalog id of the vision model.
        #[arg(long, default_value = "granite-docling-258m")]
        model: String,
        /// Override the prompt handed to the model.
        #[arg(long)]
        prompt: Option<String>,
        /// Token budget per page.
        #[arg(long, default_value_t = 1024)]
        max_tokens: u32,
        /// Keep each page's raw model output in the JSONL. Costs disk, but it
        /// is what makes `aphrody ocr clean` able to re-apply a cleanup rule
        /// later without reading the images again.
        #[arg(long)]
        raw: bool,
        /// Keep the model resident behind a llama-server instead of spawning
        /// one process per page. Several times faster over a long batch, at
        /// the cost of crash isolation: a server that dies takes the run.
        #[arg(long)]
        server: bool,
        /// Loopback port for --server.
        #[arg(long, default_value_t = 8791)]
        server_port: u16,
    },
}

/// Run an `ocr` action.
///
/// # Errors
///
/// Returns a `miette` report when the model or the llama.cpp runner is
/// missing, or when the output file cannot be written.
pub(crate) fn run(action: OcrAction) -> miette::Result<()> {
    match action {
        OcrAction::Page { image, model, prompt, max_tokens, raw, output } => {
            page(&image, &model, prompt.as_deref(), max_tokens, raw, &output)
        }
        OcrAction::Clean { input, out } => clean(&input, out.as_deref()),
        OcrAction::Batch {
            dir,
            out,
            limit,
            skip_done,
            model,
            prompt,
            max_tokens,
            raw,
            server,
            server_port,
        } => batch(
            &dir,
            out.as_deref(),
            limit,
            skip_done,
            &model,
            prompt.as_deref(),
            max_tokens,
            raw,
            server.then_some(server_port),
        ),
    }
}

fn options(model: &str, prompt: Option<&str>, max_tokens: u32, raw: bool) -> OcrOptions {
    // Start from the model's own defaults so the trained prompt follows the
    // model choice; an explicit --prompt still wins.
    let mut options = OcrOptions { max_tokens, keep_raw: raw, ..OcrOptions::for_model(model) };
    if let Some(prompt) = prompt {
        options.prompt = prompt.to_owned();
    }
    options
}

fn page(
    image: &Path,
    model: &str,
    prompt: Option<&str>,
    max_tokens: u32,
    raw: bool,
    output: &OutputOpts,
) -> miette::Result<()> {
    let runner = VlmRunner::new(options(model, prompt, max_tokens, raw))
        .map_err(|e| miette::miette!("{e}"))?;
    let result = runner.read(image).map_err(|e| miette::miette!("{e}"))?;

    let json = serde_json::to_value(&result)
        .map_err(|e| miette::miette!("serialise result: {e}"))?;

    let mut report = aphrody_models::Report::new(
        format!("OCR {}", image.display()),
        &["FIELD", "VALUE"],
    );
    report.push(vec!["model".into(), model.to_owned()]);
    report.push(vec!["elapsed".into(), format!("{} ms", result.elapsed_ms)]);
    report.push(vec![
        "text".into(),
        result.text.markdown().map_or_else(|| "(none)".to_owned(), ToOwned::to_owned),
    ]);

    output.emit_report(&report, &json)
}

fn batch(
    dir: &Path,
    out: Option<&Path>,
    limit: Option<usize>,
    skip_done: bool,
    model: &str,
    prompt: Option<&str>,
    max_tokens: u32,
    keep_raw: bool,
    server_port: Option<u16>,
) -> miette::Result<()> {
    let opts = options(model, prompt, max_tokens, keep_raw);

    // Two backends, one loop: a resident server for throughput, a process per
    // page for isolation. The reader closure is the only thing that differs.
    let resident = match server_port {
        Some(port) => {
            eprintln!("starting llama-server on 127.0.0.1:{port} (loading {model})…");
            Some(ServerRunner::start(opts.clone(), port).map_err(|e| miette::miette!("{e}"))?)
        }
        None => None,
    };
    let runner = if resident.is_some() {
        None
    } else {
        Some(VlmRunner::new(opts.clone()).map_err(|e| miette::miette!("{e}"))?)
    };

    let done = if skip_done {
        out.map(already_done).transpose()?.unwrap_or_default()
    } else {
        std::collections::BTreeSet::new()
    };

    // Append, never truncate: resuming must not destroy the earlier pages.
    let mut sink: Box<dyn std::io::Write> = match out {
        Some(path) => {
            if let Some(parent) = path.parent() {
                if !parent.as_os_str().is_empty() {
                    std::fs::create_dir_all(parent)
                        .map_err(|e| miette::miette!("create {}: {e}", parent.display()))?;
                }
            }
            Box::new(
                std::fs::OpenOptions::new()
                    .create(true)
                    .append(true)
                    .open(path)
                    .map_err(|e| miette::miette!("open {}: {e}", path.display()))?,
            )
        }
        None => Box::new(std::io::stdout()),
    };

    eprintln!("reading {} with {model}", dir.display());
    if !done.is_empty() {
        eprintln!("skipping {} page(s) already in {}", done.len(), out.unwrap_or(Path::new("-")).display());
    }

    let mut read = 0_usize;
    let mut with_text = 0_usize;
    let mut failed = 0_usize;
    let started = std::time::Instant::now();

    // One loop over both backends: the file selection and its order have to be
    // identical, or a resumed run would skip different pages than it recorded.
    let images = aphrody_ocr::list_images_sorted(dir).map_err(|e| miette::miette!("{e}"))?;
    let pending: Vec<PathBuf> = images.into_iter().filter(|p| !done.contains(p)).collect();
    let total = pending.len();

    for image in pending.into_iter().take(limit.unwrap_or(usize::MAX)) {
        let outcome = match (&resident, &runner) {
            (Some(server), _) => server.read(&image),
            (None, Some(cli)) => cli.read(&image),
            // `batch` builds exactly one of the two above.
            (None, None) => unreachable!("no OCR backend was constructed"),
        };

        match outcome {
            Ok(result) => {
                read += 1;
                if result.text.has_text() {
                    with_text += 1;
                }
                if let Err(e) = write_line(&mut sink, &result) {
                    eprintln!("write failed: {e}");
                }
                if read % 10 == 0 {
                    let rate = started.elapsed().as_secs_f64() / f64::from(u32::try_from(read).unwrap_or(u32::MAX));
                    eprintln!(
                        "  {read}/{total} read, {with_text} with text, {failed} failed, {rate:.1}s/page"
                    );
                }
            }
            Err(e) => {
                // One unreadable plate must not cost the other 399.
                failed += 1;
                eprintln!("page failed ({}): {e}", image.display());
            }
        }
    }

    let _ = sink.flush();
    eprintln!(
        "\n{read}/{total} page(s) read in {:.1}s — {with_text} with text, {} textless, {failed} failed",
        started.elapsed().as_secs_f64(),
        read.saturating_sub(with_text)
    );
    Ok(())
}

/// Re-run the cleanup rules over an existing JSONL.
///
/// Only lines that kept their raw model output can be recleaned: everything
/// else has already lost the text the rules act on. Those lines pass through
/// untouched rather than being dropped, so the file stays complete and a
/// caller never silently loses pages to a maintenance command.
fn clean(input: &Path, out: Option<&Path>) -> miette::Result<()> {
    let file = std::fs::File::open(input)
        .map_err(|e| miette::miette!("read {}: {e}", input.display()))?;

    let mut lines = Vec::new();
    let mut recleaned = 0_usize;
    let mut changed = 0_usize;
    let mut passthrough = 0_usize;

    for line in std::io::BufReader::new(file).lines() {
        let Ok(line) = line else { continue };
        if line.trim().is_empty() {
            continue;
        }
        let Ok(mut value) = serde_json::from_str::<serde_json::Value>(&line) else {
            // A torn last line from a killed run: keep it out rather than
            // writing back something unparseable.
            continue;
        };

        let Some(raw) = value.get("raw").and_then(serde_json::Value::as_str) else {
            passthrough += 1;
            lines.push(value);
            continue;
        };

        let before = value
            .get("text")
            .and_then(|t| t.get("markdown"))
            .and_then(serde_json::Value::as_str)
            .unwrap_or_default()
            .to_owned();

        let document = aphrody_ocr::Document::parse(raw);
        let after = document.to_markdown();
        if after.as_deref().unwrap_or_default() != before {
            changed += 1;
        }
        value["text"] = match &after {
            Some(markdown) => serde_json::json!({ "kind": "text", "markdown": markdown }),
            None => serde_json::json!({ "kind": "none" }),
        };
        recleaned += 1;
        lines.push(value);
    }

    let target = out.unwrap_or(input);
    let body = lines
        .iter()
        .map(|v| serde_json::to_string(v).unwrap_or_default())
        .collect::<Vec<_>>()
        .join("\n");
    std::fs::write(target, format!("{body}\n"))
        .map_err(|e| miette::miette!("write {}: {e}", target.display()))?;

    eprintln!(
        "{recleaned} page(s) recleaned ({changed} changed), {passthrough} without raw output kept as-is -> {}",
        target.display()
    );
    if passthrough > 0 && recleaned == 0 {
        eprintln!("note: no line carried `raw`; re-run the batch with --raw to make cleaning possible");
    }
    Ok(())
}

/// Image paths already recorded in a JSONL file.
fn already_done(path: &Path) -> miette::Result<std::collections::BTreeSet<PathBuf>> {
    let file = match std::fs::File::open(path) {
        Ok(f) => f,
        // No file yet simply means nothing is done.
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            return Ok(std::collections::BTreeSet::new());
        }
        Err(e) => return Err(miette::miette!("read {}: {e}", path.display())),
    };

    let mut done = std::collections::BTreeSet::new();
    for line in std::io::BufReader::new(file).lines() {
        let Ok(line) = line else { continue };
        // A run killed mid-write leaves a partial last line; skipping it is
        // correct — that page will simply be read again.
        let Ok(value) = serde_json::from_str::<serde_json::Value>(&line) else { continue };
        if let Some(image) = value.get("image").and_then(serde_json::Value::as_str) {
            done.insert(PathBuf::from(image));
        }
    }
    Ok(done)
}

/// Write one result as a JSONL line and flush it.
///
/// Flushing per line is what makes `--skip-done` trustworthy after a kill.
fn write_line(sink: &mut Box<dyn std::io::Write>, result: &PageResult) -> std::io::Result<()> {
    let line = serde_json::to_string(result)?;
    sink.write_all(line.as_bytes())?;
    sink.write_all(b"\n")?;
    sink.flush()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn options_carry_the_overrides_through() {
        let opts = options("dots-ocr", Some("read it"), 512, true);
        assert_eq!(opts.model_id, "dots-ocr");
        assert_eq!(opts.prompt, "read it");
        assert_eq!(opts.max_tokens, 512);
        assert!(opts.keep_raw);
    }

    #[test]
    fn options_without_a_prompt_keep_the_trained_instruction() {
        let opts = options("granite-docling-258m", None, 1024, false);
        assert!(opts.prompt.contains("docling"), "{}", opts.prompt);
    }

    #[test]
    fn resume_reads_back_the_images_already_recorded() {
        let dir = tempfile::tempdir().unwrap();
        let jsonl = dir.path().join("out.jsonl");
        std::fs::write(
            &jsonl,
            "{\"image\":\"a.jpg\",\"text\":{\"kind\":\"none\"}}\n\
             {\"image\":\"b.jpg\",\"text\":{\"kind\":\"text\",\"markdown\":\"hi\"}}\n",
        )
        .unwrap();

        let done = already_done(&jsonl).unwrap();
        assert_eq!(done.len(), 2);
        assert!(done.contains(&PathBuf::from("a.jpg")));
        assert!(done.contains(&PathBuf::from("b.jpg")));
    }

    #[test]
    fn a_truncated_last_line_is_skipped_not_fatal() {
        let dir = tempfile::tempdir().unwrap();
        let jsonl = dir.path().join("out.jsonl");
        // Exactly what a process killed mid-write leaves behind.
        std::fs::write(&jsonl, "{\"image\":\"a.jpg\"}\n{\"image\":\"b.jp").unwrap();
        let done = already_done(&jsonl).unwrap();
        assert_eq!(done, [PathBuf::from("a.jpg")].into_iter().collect());
    }

    #[test]
    fn an_absent_resume_file_means_nothing_is_done() {
        let dir = tempfile::tempdir().unwrap();
        assert!(already_done(&dir.path().join("absent.jsonl")).unwrap().is_empty());
    }

    #[test]
    fn cleaning_reapplies_the_rules_to_lines_that_kept_their_raw_output() {
        let dir = tempfile::tempdir().unwrap();
        let jsonl = dir.path().join("in.jsonl");
        // A watermark that predates the rule: `text` still carries it, `raw`
        // is what the rule can act on.
        std::fs::write(
            &jsonl,
            "{\"image\":\"a.jpg\",\"raw\":\"宿敵\\ncapsulecommentary.com\",\
             \"text\":{\"kind\":\"text\",\"markdown\":\"宿敵\\ncapsulecommentary.com\"}}\n",
        )
        .unwrap();

        let out = dir.path().join("out.jsonl");
        clean(&jsonl, Some(&out)).unwrap();

        let written = std::fs::read_to_string(&out).unwrap();
        let value: serde_json::Value = serde_json::from_str(written.trim()).unwrap();
        let markdown = value["text"]["markdown"].as_str().unwrap();
        assert!(markdown.contains("宿敵"), "{markdown}");
        assert!(!markdown.contains("capsulecommentary"), "{markdown}");
    }

    #[test]
    fn cleaning_keeps_lines_without_raw_output_instead_of_dropping_them() {
        let dir = tempfile::tempdir().unwrap();
        let jsonl = dir.path().join("in.jsonl");
        std::fs::write(
            &jsonl,
            "{\"image\":\"a.jpg\",\"text\":{\"kind\":\"text\",\"markdown\":\"gardé\"}}\n\
             {\"image\":\"b.jpg\",\"text\":{\"kind\":\"none\"}}\n",
        )
        .unwrap();

        let out = dir.path().join("out.jsonl");
        clean(&jsonl, Some(&out)).unwrap();

        // A maintenance command must never silently lose pages.
        let written = std::fs::read_to_string(&out).unwrap();
        assert_eq!(written.lines().count(), 2, "{written}");
        assert!(written.contains("gardé"), "{written}");
    }

    #[test]
    fn cleaning_defaults_to_rewriting_the_input() {
        let dir = tempfile::tempdir().unwrap();
        let jsonl = dir.path().join("in.jsonl");
        std::fs::write(&jsonl, "{\"image\":\"a.jpg\",\"raw\":\"texte réel\"}\n").unwrap();
        clean(&jsonl, None).unwrap();
        let written = std::fs::read_to_string(&jsonl).unwrap();
        assert!(written.contains("texte réel"), "{written}");
    }
}
