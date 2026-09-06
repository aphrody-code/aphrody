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

use std::{
    io::{BufRead as _, Write as _},
    path::{Path, PathBuf},
    time::Instant,
};

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
    /// Read one image with local PP-OCRv5 and emit text regions plus geometry.
    ///
    /// Unlike `page`, this is deterministic CTC OCR rather than document
    /// markdown reconstruction. It never downloads weights: install the model
    /// first with `aphrody model pull ppocr-v5-mobile`.
    ///
    /// Example: aphrody ocr ppocr plate.jpg --json
    Ppocr {
        /// Image to read.
        image: PathBuf,
        /// Catalog id containing paired PP-OCR detector and recognizer weights.
        #[arg(long, default_value = "ppocr-v5-mobile")]
        model: String,
        #[command(flatten)]
        output: OutputOpts,
    },
    /// Audit a JSONL for defects before depositing it anywhere.
    ///
    /// A deposit is hard to undo: writing four hundred plates of degenerate
    /// output into a corpus costs far more than the second this takes. Exits
    /// non-zero when a blocking defect is found (control token, stuck
    /// generation, surviving markup); watermarks are reported but do not fail.
    ///
    /// Example: aphrody ocr audit lot-001.jsonl --json
    Audit {
        /// JSONL produced by `aphrody ocr batch`.
        input: PathBuf,
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
        /// EXPERIMENTAL — keep the model resident behind a llama-server
        /// instead of spawning one process per page.
        ///
        /// Measured on this hardware with dots.ocr: the server starts and
        /// answers /health in three seconds, then no page completes. The
        /// per-process backend is what is known to work end to end, and stays
        /// the default. Do not use this for a real batch until the stall is
        /// understood.
        #[arg(long)]
        server: bool,
        /// Loopback port for --server.
        #[arg(long, default_value_t = 8791)]
        server_port: u16,
    },
    /// Transcribe a Shenron Dragon Ball databook lot with the safe preset.
    ///
    /// This is deliberately separate from generic `batch`: granite-docling is
    /// useful for general layout, but reads Japanese databook text as pictures.
    /// Shenron databooks require dots.ocr and raw output so cleanup can be
    /// replayed without spending GPU time on the same scans again.
    Databooks {
        /// Directory containing one exported Shenron lot's images.
        dir: PathBuf,
        /// JSONL file to append results to. Without it, results go to stdout.
        #[arg(long)]
        out: Option<PathBuf>,
        /// Stop after this many previously unread images.
        #[arg(long)]
        limit: Option<usize>,
        /// Resume an interrupted JSONL without reading completed pages again.
        #[arg(long)]
        skip_done: bool,
        /// Token budget per page. Dense Japanese reference sheets need room.
        #[arg(long, default_value_t = 2048)]
        max_tokens: u32,
        /// Keep dots.ocr resident behind llama-server (experimental).
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
        },
        OcrAction::Ppocr { image, model, output } => ppocr_page(&image, &model, &output),
        OcrAction::Audit { input, output } => audit(&input, &output),
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
        OcrAction::Databooks { dir, out, limit, skip_done, max_tokens, server, server_port } => {
            // Same PageResult JSONL as `batch`, accepted directly by Shenron.
            // Raw output is forced: later deterministic cleanup must not require
            // re-reading a long GPU batch.
            batch(
                &dir,
                out.as_deref(),
                limit,
                skip_done,
                "dots-ocr",
                None,
                max_tokens,
                true,
                server.then_some(server_port),
            )
        },
    }
}

fn ppocr_page(image: &Path, model: &str, output: &OutputOpts) -> miette::Result<()> {
    use aphrody_ocr::ocr_core::{
        Attempt, AttemptStatus, ImageIdentity, OcrResult, OcrStatus, Quality, RESULT_SCHEMA_V2,
        RunProvenance,
    };

    let config = aphrody_infer::SessionConfig::from_profile(&aphrody_models::accel::probe());
    let started = Instant::now();
    let run = |session: &aphrody_infer::SessionConfig| {
        let mut runner = aphrody_ocr::PpOcr::load(model, session)?;
        let regions = runner.recognise_path(image)?;
        Ok::<_, aphrody_ocr::onnx::OnnxOcrError>((regions, runner.providers()))
    };
    let (regions, providers, fallback_reason) = match run(&config) {
        Ok((regions, providers)) => (regions, providers, None),
        Err(error)
            if config
                .providers
                .first()
                .is_some_and(|provider| *provider != aphrody_models::Accelerator::Cpu) =>
        {
            let requested = config.providers[0].as_str();
            let cpu = aphrody_infer::SessionConfig::with_only(aphrody_models::Accelerator::Cpu);
            let (regions, providers) = run(&cpu).map_err(|fallback| {
                miette::miette!(
                    "PP-OCR failed on {requested} ({error}); CPU retry also failed: {fallback}"
                )
            })?;
            (regions, providers, Some(format!("provider fallback: {requested} failed: {error}")))
        },
        Err(error) => return Err(miette::miette!("{error}")),
    };
    let provider = (providers[0] == providers[1]).then(|| providers[0].as_str().to_owned());
    let blocks = regions
        .into_iter()
        .map(aphrody_ocr::PpOcr::block)
        .filter(|block| !block.text.is_empty())
        .collect::<Vec<_>>();
    let confidences = blocks.iter().filter_map(|block| block.confidence).collect::<Vec<_>>();
    let mean_confidence = (!confidences.is_empty())
        .then(|| confidences.iter().sum::<f32>() / confidences.len() as f32);
    let elapsed_ms = started.elapsed().as_millis();
    let status = if blocks.is_empty() { OcrStatus::NoText } else { OcrStatus::Text };
    let reasons = fallback_reason.into_iter().collect::<Vec<_>>();
    let result = OcrResult {
        schema: RESULT_SCHEMA_V2.into(),
        page_id: image.to_string_lossy().into_owned(),
        image: ImageIdentity::from_path(image.to_path_buf()),
        attempts: vec![Attempt {
            run: RunProvenance {
                model_id: model.into(),
                backend: "onnx-runtime".into(),
                provider,
                model_digest: None,
                prompt_digest: None,
            },
            status: AttemptStatus::Completed,
            elapsed_ms,
            quality: Quality { mean_confidence, reasons: reasons.clone() },
            error: None,
        }],
        status,
        markdown: None,
        blocks,
        quality: Quality { mean_confidence, reasons },
        raw: None,
    };
    let mut report =
        aphrody_models::Report::new(format!("PP-OCR {}", image.display()), &["FIELD", "VALUE"]);
    report.push(vec!["model".into(), model.into()]);
    report.push(vec!["elapsed".into(), format!("{elapsed_ms} ms")]);
    report.push(vec!["regions".into(), result.blocks.len().to_string()]);
    report.push(vec![
        "text".into(),
        result.blocks.iter().map(|block| block.text.as_str()).collect::<Vec<_>>().join("\\n"),
    ]);
    let json = serde_json::to_value(&result)
        .map_err(|e| miette::miette!("serialise PP-OCR result: {e}"))?;
    output.emit_report(&report, &json)
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

    let json =
        serde_json::to_value(&result).map_err(|e| miette::miette!("serialise result: {e}"))?;

    let mut report =
        aphrody_models::Report::new(format!("OCR {}", image.display()), &["FIELD", "VALUE"]);
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
        },
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
        },
        None => Box::new(std::io::stdout()),
    };

    eprintln!("reading {} with {model}", dir.display());
    if !done.is_empty() {
        eprintln!(
            "skipping {} page(s) already in {}",
            done.len(),
            out.unwrap_or(Path::new("-")).display()
        );
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
                    let rate = started.elapsed().as_secs_f64()
                        / f64::from(u32::try_from(read).unwrap_or(u32::MAX));
                    eprintln!(
                        "  {read}/{total} read, {with_text} with text, {failed} failed, \
                         {rate:.1}s/page"
                    );
                }
            },
            Err(e) => {
                // One unreadable plate must not cost the other 399.
                failed += 1;
                eprintln!("page failed ({}): {e}", image.display());
            },
        }
    }

    let _ = sink.flush();
    eprintln!(
        "\n{read}/{total} page(s) read in {:.1}s — {with_text} with text, {} textless, {failed} \
         failed",
        started.elapsed().as_secs_f64(),
        read.saturating_sub(with_text)
    );
    Ok(())
}

/// Audit a JSONL and refuse a batch that carries blocking defects.
fn audit(input: &Path, output: &OutputOpts) -> miette::Result<()> {
    let file =
        std::fs::File::open(input).map_err(|e| miette::miette!("read {}: {e}", input.display()))?;

    let mut pages: Vec<(PathBuf, Option<String>)> = Vec::new();
    for line in std::io::BufReader::new(file).lines() {
        let Ok(line) = line else { continue };
        if line.trim().is_empty() {
            continue;
        }
        let Ok(value) = serde_json::from_str::<serde_json::Value>(&line) else { continue };
        let image = value
            .get("image")
            .and_then(serde_json::Value::as_str)
            .map_or_else(PathBuf::new, PathBuf::from);
        let text = value
            .get("text")
            .and_then(|t| t.get("markdown"))
            .and_then(serde_json::Value::as_str)
            .map(ToOwned::to_owned);
        pages.push((image, text));
    }

    let report = aphrody_ocr::audit::audit_batch(
        pages.iter().map(|(image, text)| (image.clone(), text.as_deref())),
    );

    let mut table = aphrody_models::Report::new(format!("Audit {}", input.display()), &[
        "PAGE", "DEFECT", "DETAIL",
    ])
    .with_summary(format!(
        "{} with text, {} textless, {} finding(s) on {} page(s)",
        report.examined,
        report.textless,
        report.finding_count(),
        report.flagged.len()
    ))
    .with_footer(if report.has_blocking() {
        "blocking defects found — do not deposit this batch".to_owned()
    } else {
        "no blocking defect".to_owned()
    });

    for page in &report.flagged {
        let name = page
            .image
            .file_name()
            .map_or_else(|| page.image.display().to_string(), |n| n.to_string_lossy().into_owned());
        for finding in &page.findings {
            let (kind, detail) = describe(finding);
            table.push(vec![name.clone(), kind.to_owned(), detail]);
        }
    }

    let json =
        serde_json::to_value(&report).map_err(|e| miette::miette!("serialise audit: {e}"))?;
    output.emit_report(&table, &json)?;

    if report.has_blocking() {
        // Refusing here is the whole point: a deposit is hard to undo.
        return Err(miette::miette!(
            "{} page(s) carry blocking defects",
            report
                .flagged
                .iter()
                .filter(|p| p.findings.iter().any(aphrody_ocr::audit::Finding::is_blocking))
                .count()
        ));
    }
    Ok(())
}

/// Render one finding as `(kind, detail)`.
fn describe(finding: &aphrody_ocr::audit::Finding) -> (&'static str, String) {
    use aphrody_ocr::audit::Finding;
    match finding {
        Finding::ControlToken { token } => ("control-token", token.clone()),
        Finding::Loop { word, repeats } => ("loop", format!("{word} x{repeats}")),
        Finding::Markup { sample } => ("markup", sample.clone()),
        Finding::Watermark { line } => ("watermark", line.clone()),
        // `Finding` is `#[non_exhaustive]`: a new defect kind added upstream
        // must still be reported, not silently dropped from the table.
        other => ("other", format!("{other:?}")),
    }
}

/// Re-run the cleanup rules over an existing JSONL.
///
/// Only lines that kept their raw model output can be recleaned: everything
/// else has already lost the text the rules act on. Those lines pass through
/// untouched rather than being dropped, so the file stays complete and a
/// caller never silently loses pages to a maintenance command.
fn clean(input: &Path, out: Option<&Path>) -> miette::Result<()> {
    let file =
        std::fs::File::open(input).map_err(|e| miette::miette!("read {}: {e}", input.display()))?;

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
        "{recleaned} page(s) recleaned ({changed} changed), {passthrough} without raw output kept \
         as-is -> {}",
        target.display()
    );
    if passthrough > 0 && recleaned == 0 {
        eprintln!(
            "note: no line carried `raw`; re-run the batch with --raw to make cleaning possible"
        );
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
        },
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

/// Dit si `argv` désigne exactement `aphrody ocr batch --server`.
///
/// Ce chemin est entièrement synchrone : il bloque sur un processus fils et
/// sur des sockets loopback, sans toucher une seule API async. Le faire tourner
/// dans un runtime tokio ne serait pas seulement inutile — le runtime finit
/// par être détruit depuis un contexte où bloquer est interdit, ce qui panique.
/// Il est donc aiguillé AVANT qu'un runtime n'existe.
pub(crate) fn is_synchronous_server_batch(argv: &[std::ffi::OsString]) -> bool {
    let words: Vec<String> = argv.iter().map(|a| a.to_string_lossy().into_owned()).collect();
    let Some(ocr) = words.iter().position(|w| w == "ocr") else { return false };
    words.get(ocr + 1).is_some_and(|w| w == "batch") && words.iter().any(|w| w == "--server")
}

/// Exécute `ocr batch --server` sans aucun runtime async.
///
/// Renvoie le code de sortie du processus : les échecs sont imprimés plutôt que
/// propagés, puisque le rapport d'erreurs habituel de la CLI n'est pas encore
/// en place à ce stade.
pub(crate) fn run_sync_server_batch(argv: &[std::ffi::OsString]) -> i32 {
    use clap::Parser as _;

    #[derive(clap::Parser)]
    struct Shim {
        #[command(subcommand)]
        command: Wrapper,
    }
    #[derive(clap::Subcommand)]
    enum Wrapper {
        Ocr {
            #[command(subcommand)]
            action: OcrAction,
        },
    }

    // On ne parse que le sous-ensemble accepté ici ; le reste a déjà été écarté
    // par `is_synchronous_server_batch`.
    let parsed = match Shim::try_parse_from(argv) {
        Ok(parsed) => parsed,
        Err(e) => {
            let _ = e.print();
            return 2;
        },
    };
    let Wrapper::Ocr { action } = parsed.command;

    match run(action) {
        Ok(()) => 0,
        Err(report) => {
            eprintln!("aphrody: {report:?}");
            70 // EX_SOFTWARE (sysexits.h)
        },
    }
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
    fn databooks_preset_uses_the_japanese_ocr_model_contract() {
        let opts = options("dots-ocr", None, 2048, true);
        assert_eq!(opts.model_id, "dots-ocr");
        assert_eq!(opts.max_tokens, 2048);
        assert!(opts.keep_raw);
        assert_eq!(opts.prompt, "Extract all text from this image.");
    }

    #[test]
    fn resume_reads_back_the_images_already_recorded() {
        let dir = tempfile::tempdir().unwrap();
        let jsonl = dir.path().join("out.jsonl");
        std::fs::write(
            &jsonl,
            "{\"image\":\"a.jpg\",\"text\":{\"kind\":\"none\"}}\n{\"image\":\"b.jpg\",\"text\":{\"\
             kind\":\"text\",\"markdown\":\"hi\"}}\n",
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
            "{\"image\":\"a.jpg\",\"raw\":\"宿敵\\ncapsulecommentary.com\",\"text\":{\"kind\":\"\
             text\",\"markdown\":\"宿敵\\ncapsulecommentary.com\"}}\n",
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
            "{\"image\":\"a.jpg\",\"text\":{\"kind\":\"text\",\"markdown\":\"gardé\"}}\n{\"image\"\
             :\"b.jpg\",\"text\":{\"kind\":\"none\"}}\n",
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
