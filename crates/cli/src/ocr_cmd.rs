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
        #[arg(long, default_value_t = aphrody_ocr::vlm::DEFAULT_MAX_TOKENS)]
        max_tokens: u32,
        /// Include the model's raw output.
        #[arg(long)]
        raw: bool,
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
        /// Also run the checks that need a Japanese dictionary — chiefly, is
        /// this page real Japanese or invented kana? No other check can tell:
        /// gibberish has the exact shape of a transcription.
        ///
        /// Needs a binary built with `--features ocr-japanese`.
        #[arg(long)]
        japonais: bool,
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
        /// Also apply the Japanese rules: rejoin lines cut mid-word and drop
        /// spaces the model inserted between two Japanese characters.
        ///
        /// Measured on 5762 databook plates: 50 725 line breaks fall between
        /// two Japanese characters, and Japanese has no spaces, so both are
        /// artefacts of a page read in columns. Needs a binary built with
        /// `--features ocr-japanese` — the IPADIC dictionary is compiled in.
        #[arg(long)]
        japonais: bool,
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
        ///
        /// A backstop rather than the usual stopping condition: a page now
        /// ends where the model ends it. What this guards against is a
        /// generation that never terminates.
        #[arg(long, default_value_t = aphrody_ocr::vlm::DEFAULT_MAX_TOKENS)]
        max_tokens: u32,
        /// Keep each page's raw model output in the JSONL. Costs disk, but it
        /// is what makes `aphrody ocr clean` able to re-apply a cleanup rule
        /// later without reading the images again.
        #[arg(long)]
        raw: bool,
        /// Keep the model resident behind a llama-server instead of spawning
        /// one process per page. Three times faster, and no less faithful.
        ///
        /// An earlier note here said the opposite — that the server "reads less
        /// of each page", 1384 characters against 1624 on plate 18-0249. That
        /// was a missing stop token, not a front end: dots.ocr closes its turn
        /// with a token llama.cpp did not have in its end-of-generation set, so
        /// half the pages ran to the token ceiling and the loop cutter trimmed
        /// them all back to the same prefix. Fixed on both backends.
        ///
        /// Re-measured after the fix, same six plates: 2.6 s per plate against
        /// 8.4 s, five transcriptions identical to the character. On the sixth
        /// it is the *default* backend that degenerates, repeating one sentence
        /// four times where the server reads distinct furigana.
        ///
        /// The per-page process still earns its place when a run must survive a
        /// crash: a server that dies takes the batch with it. Do not mix the two
        /// over one corpus — not for fidelity now, but because greedy decoding
        /// through two front ends is not bit-identical.
        #[arg(long)]
        server: bool,
        /// Loopback port for --server.
        #[arg(long, default_value_t = 8791)]
        server_port: u16,
        /// Pages the resident server reads at once, one slot each.
        ///
        /// This is where a batch's wall-clock actually comes from. Generating
        /// a page is bound by memory bandwidth, not by arithmetic: at one
        /// sequence the GPU re-reads four gigabytes of weights for every
        /// single token. Two sequences read those same weights once and emit
        /// two tokens, so the second page is very nearly free — the cost is
        /// one KV cache per slot, not a second copy of the model.
        ///
        /// How far that goes depends entirely on the plates. A light batch —
        /// 1600x1056, some 250 tokens generated — is dominated by the image
        /// prompt, and there two slots won: 35 s against 50 s at four. The
        /// databook corpus inverts it. Its plates are 1340x2048 and generate
        /// 1357 tokens, so the 2.3 s image prompt is a sixth of the page and
        /// decode is the rest; twelve of them took 196 s at two slots, 146 s
        /// at four, 118 s at eight. Measure on the plates you will read, not
        /// on a sample chosen for being quick to try.
        ///
        /// One earlier reading here was that six slots "shift the numerics",
        /// because a plate came back with 4918 characters of repeated sentence
        /// where two slots read 433 clean ones. That plate does degenerate —
        /// but so it does at two. There is no reproducible decode to protect:
        /// two runs with identical slots, `temperature 0` and a pinned seed
        /// diverge on seven plates out of twelve, because llama.cpp's batched
        /// attention is not invariant to batch composition and composition
        /// follows the order pages land in slots. Choose this number for
        /// throughput and for the KV cache the card can hold; it buys no
        /// stability either way.
        ///
        /// Ignored without --server, where each page is its own process and
        /// concurrency would mean loading the model twice.
        #[arg(long, default_value_t = aphrody_ocr::server::DEFAULT_SLOTS)]
        slots: u32,
        /// Context window per slot, in tokens.
        ///
        /// A vision model spends most of its context on the image: a
        /// high-resolution plate becomes thousands of visual tokens before the
        /// first word is generated. Too small a window truncates the
        /// transcription in silence, which reads exactly like a page that had
        /// nothing more to say.
        #[arg(long, default_value_t = aphrody_ocr::server::DEFAULT_CTX_PER_SLOT)]
        ctx: u32,
    },
    /// Re-read the plates a run found silent, one speech balloon at a time.
    ///
    /// A document model returns nothing for a comics page: the lettering is a
    /// detail inside what it reads as a drawing. Cropped out and enlarged, the
    /// very same balloon reads fine. This takes a JSONL from `batch`, finds
    /// the pages it recorded as textless, and gives each balloon its own read.
    ///
    /// Measured on twelve silent plates: 44 balloons recovered across nine.
    ///
    /// Audit before depositing — a cropped fragment of a hand-drawn sound
    /// effect makes the model invent characters rather than return nothing.
    ///
    /// Example: aphrody ocr bulles lot-028.jsonl --images lot-028/images --out lot-028-bulles.jsonl --server
    #[cfg(feature = "ocr-bulles")]
    Bulles {
        /// JSONL produced by `aphrody ocr batch`.
        input: PathBuf,
        /// Directory holding the plates. Without it, the paths recorded in the
        /// JSONL are used as they stand.
        #[arg(long)]
        images: Option<PathBuf>,
        /// JSONL to append the recomposed plates to.
        #[arg(long)]
        out: PathBuf,
        /// Skip plates already present in `--out`.
        #[arg(long)]
        skip_done: bool,
        /// Stop after this many plates.
        #[arg(long)]
        limit: Option<usize>,
        /// Keep the crops under this directory instead of deleting them.
        ///
        /// The only way to check a surprising reading against the pixels that
        /// produced it, which this command needs more than `batch` does: its
        /// input is a fragment of a page, and a fragment can mislead.
        #[arg(long)]
        decoupes: Option<PathBuf>,
        /// Longest side a crop is enlarged towards, in pixels.
        ///
        /// Enlargement is what makes the read work: a raw crop of a small
        /// balloon still returns nothing, the same crop tripled reads `クッ!!`.
        #[arg(long, default_value_t = 900)]
        cible: u32,
        /// Most balloons read per plate.
        ///
        /// Bounds the cost, since each is one model read and four fifths of
        /// the regions are bright drawing rather than lettering. Regions are
        /// tried largest first, so the cut falls on the least likely.
        #[arg(long, default_value_t = 30)]
        max_bulles: usize,
        /// Catalog id of the vision model.
        #[arg(long, default_value = "dots-ocr")]
        model: String,
        /// Override the prompt handed to the model.
        #[arg(long)]
        prompt: Option<String>,
        /// Token budget per balloon.
        ///
        /// Far below a page's: a balloon holds a line of dialogue, and a
        /// budget sized for a full plate only buys room for a degenerate loop.
        #[arg(long, default_value_t = 256)]
        max_tokens: u32,
        /// Keep the resident server between plates.
        #[arg(long)]
        server: bool,
        /// Loopback port for --server.
        #[arg(long, default_value_t = 8791)]
        server_port: u16,
        /// Balloons read at once by the resident server.
        #[arg(long, default_value_t = aphrody_ocr::server::DEFAULT_SLOTS)]
        slots: u32,
        /// Context window per slot, in tokens.
        #[arg(long, default_value_t = aphrody_ocr::server::DEFAULT_CTX_PER_SLOT)]
        ctx: u32,
    },
}

/// Run an `ocr` action.
///
/// # Errors
///
/// Returns a `miette` report when the model or the llama.cpp runner is
/// missing, or when the output file cannot be written.
pub(crate) async fn run(action: OcrAction) -> miette::Result<()> {
    match action {
        OcrAction::Page { image, model, prompt, max_tokens, raw, output } => {
            page(&image, &model, prompt.as_deref(), max_tokens, raw, &output)
        }
        OcrAction::Audit { input, japonais, output } => audit(&input, japonais, &output),
        OcrAction::Clean { input, out, japonais } => clean(&input, out.as_deref(), japonais),
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
            slots,
            ctx,
        } => {
            batch(
                &dir,
                out.as_deref(),
                limit,
                skip_done,
                &model,
                prompt.as_deref(),
                max_tokens,
                raw,
                server.then(|| aphrody_ocr::server::ServerConfig {
                    port: server_port,
                    slots,
                    ctx_per_slot: ctx,
                }),
            )
            .await
        }
        #[cfg(feature = "ocr-bulles")]
        OcrAction::Bulles {
            input,
            images,
            out,
            skip_done,
            limit,
            decoupes,
            cible,
            max_bulles,
            model,
            prompt,
            max_tokens,
            server,
            server_port,
            slots,
            ctx,
        } => {
            let opts = options(&model, prompt.as_deref(), max_tokens, false);
            let resident = server.then(|| aphrody_ocr::server::ServerConfig {
                port: server_port,
                slots,
                ctx_per_slot: ctx,
            });
            // Comme `batch --server` : la boucle bloque sur des sockets et sur
            // un processus enfant, et faire ça sur un worker du runtime fait
            // paniquer tokio quand il vient à droper un runtime depuis un
            // contexte où il n'avait pas le droit de bloquer.
            tokio::task::spawn_blocking(move || {
                bulles(
                    &input,
                    images.as_deref(),
                    &out,
                    skip_done,
                    limit,
                    decoupes.as_deref(),
                    cible,
                    max_bulles,
                    opts,
                    resident,
                )
            })
            .await
            .map_err(|e| miette::miette!("ocr bulles task: {e}"))?
        }
    }
}

/// Whether an argv is `ocr batch … --server`.
///
/// Checked before the runtime is built, so this looks at raw arguments rather
/// than a parsed `OcrAction`: clap runs later, and by then a runtime exists.
/// Deliberately narrow — only this exact shape takes the synchronous path.
pub(crate) fn is_synchronous_server_batch(argv: &[std::ffi::OsString]) -> bool {
    let words: Vec<String> =
        argv.iter().map(|a| a.to_string_lossy().into_owned()).collect();
    let Some(ocr) = words.iter().position(|w| w == "ocr") else { return false };
    words.get(ocr + 1).is_some_and(|w| w == "batch") && words.iter().any(|w| w == "--server")
}

/// Run `ocr batch --server` with no async runtime at all.
///
/// # Errors
///
/// Returns the process exit code; failures are printed rather than propagated,
/// because this runs before the CLI's normal error reporting is set up.
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

    // Parse only the subset this path accepts; anything else has already been
    // ruled out by `is_synchronous_server_batch`.
    let parsed = match Shim::try_parse_from(argv) {
        Ok(parsed) => parsed,
        Err(e) => {
            let _ = e.print();
            return 2;
        }
    };
    let Wrapper::Ocr { action } = parsed.command;
    let OcrAction::Batch {
        dir,
        out,
        limit,
        skip_done,
        model,
        prompt,
        max_tokens,
        raw,
        server_port,
        slots,
        ctx,
        ..
    } = action
    else {
        eprintln!("aphrody: internal error — synchronous path reached with a non-batch action");
        return 70;
    };

    let opts = options(&model, prompt.as_deref(), max_tokens, raw);
    let config = aphrody_ocr::server::ServerConfig { port: server_port, slots, ctx_per_slot: ctx };
    eprintln!(
        "starting llama-server on 127.0.0.1:{server_port} (loading {model}, {slots} slot(s) x {ctx} tokens)…"
    );
    let outcome = ServerRunner::start(opts, config).and_then(|resident| {
        let workers = resident.slots() as usize;
        batch_loop(&dir, out.as_deref(), limit, skip_done, workers, &|image| {
            resident.read(image)
        })
    });

    match outcome {
        Ok(()) => 0,
        Err(e) => {
            eprintln!("aphrody: {e}");
            1
        }
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

async fn batch(
    dir: &Path,
    out: Option<&Path>,
    limit: Option<usize>,
    skip_done: bool,
    model: &str,
    prompt: Option<&str>,
    max_tokens: u32,
    keep_raw: bool,
    resident: Option<aphrody_ocr::server::ServerConfig>,
) -> miette::Result<()> {
    let opts = options(model, prompt, max_tokens, keep_raw);

    // `reqwest::blocking` cannot run inside the CLI's tokio runtime: every
    // request stalls silently. The resident backend therefore owns its own OS
    // thread, and everything it needs is moved onto it.
    if let Some(config) = resident {
        let dir = dir.to_path_buf();
        let out = out.map(Path::to_path_buf);
        let model = model.to_owned();
        // `spawn_blocking` is the only correct way to run this from the CLI's
        // async context: the loop blocks on sockets and on a child process, and
        // doing that on a runtime worker makes tokio panic when it later drops
        // a runtime it was not allowed to block in.
        return tokio::task::spawn_blocking(move || {
            eprintln!(
                "starting llama-server on 127.0.0.1:{} (loading {model}, {} slot(s) x {} tokens)…",
                config.port, config.slots, config.ctx_per_slot
            );
            let resident = ServerRunner::start(opts, config)?;
            let workers = resident.slots() as usize;
            batch_loop(&dir, out.as_deref(), limit, skip_done, workers, &|image| {
                resident.read(image)
            })
        })
        .await
        .map_err(|e| miette::miette!("ocr server task: {e}"))?
        .map_err(|e| miette::miette!("{e}"));
    }

    let runner = VlmRunner::new(opts).map_err(|e| miette::miette!("{e}"))?;
    // One worker: this backend spawns a process per page, and running two at
    // once would mean two copies of the model resident at the same time — more
    // memory for less throughput than the resident backend gives for free.
    batch_loop(dir, out, limit, skip_done, 1, &|image| runner.read(image))
        .map_err(|e| miette::miette!("{e}"))
}

/// The batch loop itself, independent of which backend reads a page.
///
/// Both backends must see the same file selection, or a resumed run would skip
/// different pages than it recorded. They do not have to see the same *order*:
/// `--skip-done` matches on paths, not on an offset, so a file written out of
/// order still resumes exactly.
///
/// `workers` is how many pages are read at once. Above one, `read_page` is
/// called from several threads at the same time, so a backend that cannot bear
/// that must ask for one — which is what the per-process backend does.
fn batch_loop(
    dir: &Path,
    out: Option<&Path>,
    limit: Option<usize>,
    skip_done: bool,
    workers: usize,
    read_page: &(dyn Fn(&Path) -> aphrody_ocr::Result<PageResult> + Sync),
) -> aphrody_ocr::Result<()> {
    let done = if skip_done {
        out.map(already_done).unwrap_or_default()
    } else {
        std::collections::BTreeSet::new()
    };

    // Append, never truncate: resuming must not destroy the earlier pages.
    let mut sink: Box<dyn std::io::Write> = match out {
        Some(path) => {
            if let Some(parent) = path.parent() {
                if !parent.as_os_str().is_empty() {
                    std::fs::create_dir_all(parent).map_err(|source| {
                        aphrody_ocr::OcrError::Io { path: parent.to_path_buf(), source }
                    })?;
                }
            }
            Box::new(
                std::fs::OpenOptions::new()
                    .create(true)
                    .append(true)
                    .open(path)
                    .map_err(|source| aphrody_ocr::OcrError::Io {
                        path: path.to_path_buf(),
                        source,
                    })?,
            )
        }
        None => Box::new(std::io::stdout()),
    };

    eprintln!("reading {}", dir.display());
    if !done.is_empty() {
        eprintln!(
            "skipping {} page(s) already in {}",
            done.len(),
            out.unwrap_or(Path::new("-")).display()
        );
    }

    let images = aphrody_ocr::list_images_sorted(dir)?;
    let pending: Vec<PathBuf> = images.into_iter().filter(|p| !done.contains(p)).collect();
    let total = pending.len();
    let queue: Vec<PathBuf> = pending.into_iter().take(limit.unwrap_or(usize::MAX)).collect();

    let mut tally = Tally::default();
    let started = std::time::Instant::now();

    if workers <= 1 {
        for image in &queue {
            tally.record(read_page(image), image, &mut sink, total, started);
        }
    } else {
        read_concurrently(&queue, workers, read_page, &mut tally, &mut sink, total, started);
    }

    let _ = sink.flush();
    eprintln!(
        "\n{}/{total} page(s) read in {:.1}s — {} with text, {} textless, {} failed",
        tally.done,
        started.elapsed().as_secs_f64(),
        tally.with_text,
        tally.done.saturating_sub(tally.with_text),
        tally.failed
    );
    Ok(())
}

/// Read `queue` with `workers` pages in flight, writing results as they land.
///
/// Work is taken by whoever is free rather than dealt out in advance: plates
/// differ by several seconds each, and a fixed split would leave one thread
/// finishing alone while the others idle.
///
/// Only the workers are parallel. Writing and counting stay on this thread —
/// a JSONL line torn in half by two concurrent writes would corrupt the very
/// file that makes a run resumable, and no amount of throughput is worth that.
fn read_concurrently(
    queue: &[PathBuf],
    workers: usize,
    read_page: &(dyn Fn(&Path) -> aphrody_ocr::Result<PageResult> + Sync),
    tally: &mut Tally,
    sink: &mut Box<dyn std::io::Write>,
    total: usize,
    started: std::time::Instant,
) {
    use std::sync::atomic::{AtomicUsize, Ordering};

    let next = AtomicUsize::new(0);
    let (tx, rx) = std::sync::mpsc::channel();

    std::thread::scope(|scope| {
        for _ in 0..workers {
            let tx = tx.clone();
            let next = &next;
            scope.spawn(move || {
                loop {
                    let index = next.fetch_add(1, Ordering::Relaxed);
                    let Some(image) = queue.get(index) else { break };
                    // A closed channel means the collector is gone, which only
                    // happens once the batch is over: stop rather than read a
                    // page whose result nobody will write down.
                    if tx.send((image.clone(), read_page(image))).is_err() {
                        break;
                    }
                }
            });
        }
        // The collector below ends when every sender is dropped; this one is
        // held by the loop above and would keep it waiting forever.
        drop(tx);

        for (image, outcome) in rx {
            tally.record(outcome, &image, sink, total, started);
        }
    });
}

/// Running counts for a batch, and the one place a result is written down.
#[derive(Debug, Default)]
struct Tally {
    done: usize,
    with_text: usize,
    failed: usize,
}

impl Tally {
    /// Write one page's outcome and fold it into the counts.
    fn record(
        &mut self,
        outcome: aphrody_ocr::Result<PageResult>,
        image: &Path,
        sink: &mut Box<dyn std::io::Write>,
        total: usize,
        started: std::time::Instant,
    ) {
        match outcome {
            Ok(result) => {
                self.done += 1;
                if result.text.has_text() {
                    self.with_text += 1;
                }
                if let Err(e) = write_line(sink, &result) {
                    eprintln!("write failed: {e}");
                }
                if self.done % 10 == 0 {
                    #[allow(clippy::cast_precision_loss)]
                    let rate = started.elapsed().as_secs_f64() / self.done as f64;
                    eprintln!(
                        "  {}/{total} read, {} with text, {} failed, {rate:.1}s/page",
                        self.done, self.with_text, self.failed
                    );
                }
            }
            Err(e) => {
                // One unreadable plate must not cost the other 399.
                self.failed += 1;
                eprintln!("page failed ({}): {e}", image.display());
            }
        }
    }
}

/// Audit a JSONL and refuse a batch that carries blocking defects.
fn audit(input: &Path, japonais: bool, output: &OutputOpts) -> miette::Result<()> {
    let japonais = japonais_optionnel(japonais)?;
    let file = std::fs::File::open(input)
        .map_err(|e| miette::miette!("read {}: {e}", input.display()))?;

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

    let mut report = aphrody_ocr::audit::audit_batch(
        pages.iter().map(|(image, text)| (image.clone(), text.as_deref())),
    );
    ajoute_findings_japonais(&mut report, &pages, japonais.as_ref());

    let mut table = aphrody_models::Report::new(
        format!("Audit {}", input.display()),
        &["PAGE", "DEFECT", "DETAIL"],
    )
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

    let json = serde_json::to_value(&report)
        .map_err(|e| miette::miette!("serialise audit: {e}"))?;
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
        Finding::RawJson { sample } => ("json-brut", sample.clone()),
        Finding::Charabia { caracteres, part_inconnue } => (
            "charabia",
            format!("{:.0}% de {caracteres} caractères hors dictionnaire", part_inconnue * 100.0),
        ),
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
/// Loads the Japanese analyser, or explains why it is not there.
///
/// A missing cargo feature must say its own name. The trap it replaces cost an
/// afternoon once already: an argv the binary cannot serve fell through to the
/// A2A fallback and complained about an unreachable server.
#[cfg(feature = "ocr-japanese")]
fn japonais_optionnel(demande: bool) -> miette::Result<Option<aphrody_ocr::japonais::Analyseur>> {
    if !demande {
        return Ok(None);
    }
    aphrody_ocr::japonais::Analyseur::nouveau()
        .map(Some)
        .map_err(|e| miette::miette!("{e}"))
}

#[cfg(not(feature = "ocr-japanese"))]
fn japonais_optionnel(demande: bool) -> miette::Result<Option<std::convert::Infallible>> {
    if demande {
        return Err(miette::miette!(
            "--japonais needs a binary built with `--features ocr-japanese`; this one was not"
        ));
    }
    Ok(None)
}

/// Applies the Japanese rules to one page, tallying what they changed.
///
/// Order matters: spaces first, then the rejoin. A space sitting between the
/// two halves of a cut word would otherwise hide the cut from the analyser,
/// and the pass would leave the word broken.
#[cfg(feature = "ocr-japanese")]
fn remet_en_forme(
    markdown: &str,
    japonais: Option<&aphrody_ocr::japonais::Analyseur>,
    lexique: &aphrody_ocr::lexique::Lexique,
    bilan: &mut Bilan,
) -> String {
    let texte = sans_dictionnaire(markdown, bilan);

    let Some(analyseur) = japonais else {
        return texte;
    };

    // L'ordre compte. Les espaces d'abord : une espace posée entre les deux
    // moitiés d'un mot coupé cacherait la coupure à l'analyseur, qui laisserait
    // le mot en deux.
    let (texte, espaces) = aphrody_ocr::japonais::espaces_parasites(&texte);
    bilan.espaces += espaces;
    let (texte, recolles) = analyseur.recolle_lignes(&texte);
    bilan.recolles += recolles;
    // Les sosies ensuite : un mot coupé en deux par la mise en page n'est
    // reconnaissable qu'une fois recollé, et c'est le dictionnaire qui
    // autorise ou refuse chaque substitution.
    let (texte, corrections) = analyseur.corrige_sosies(&texte);
    bilan.sosies += corrections.len();
    // Le lexique en dernier, parce que c'est la seule passe qui n'a pas besoin
    // du dictionnaire pour trancher — elle sait déjà. Un nom propre est
    // absent d'IPADIC par construction, donc les passes précédentes ne
    // pouvaient rien pour `プロリー` ; celle-ci le corrige de mémoire.
    let (texte, noms) = lexique.applique(&texte);
    bilan.noms += noms.len();
    // Et enfin la distance : un nom que la table ne connaît pas encore, mais
    // qui n'est qu'à un kana d'un terme attesté — et d'un seul.
    let (texte, voisins) = lexique.corrige_par_distance(&texte);
    bilan.voisins += voisins.len();
    texte
}

#[cfg(not(feature = "ocr-japanese"))]
fn remet_en_forme(
    markdown: &str,
    _japonais: Option<&std::convert::Infallible>,
    lexique: &aphrody_ocr::lexique::Lexique,
    bilan: &mut Bilan,
) -> String {
    let texte = sans_dictionnaire(markdown, bilan);
    let (texte, noms) = lexique.applique(&texte);
    bilan.noms += noms.len();
    // Et enfin la distance : un nom que la table ne connaît pas encore, mais
    // qui n'est qu'à un kana d'un terme attesté — et d'un seul.
    let (texte, voisins) = lexique.corrige_par_distance(&texte);
    bilan.voisins += voisins.len();
    texte
}

/// Les passes japonaises qui ne demandent aucun dictionnaire.
///
/// Elles tournent quelle que soit la feature : ce sont des règles d'Unicode et
/// de typographie, pas de morphologie. Une planche qui n'est pas en japonais
/// les traverse sans une seule modification — elles ne touchent que des plages
/// de caractères que rien d'autre n'occupe.
fn sans_dictionnaire(markdown: &str, bilan: &mut Bilan) -> String {
    // La ponctuation d'abord, et après la coupure de boucles qu'a déjà faite
    // `Document::parse` : une planche du corpus porte 2 034 points médians
    // d'affilée, qui relèvent de la génération bloquée. Les convertir en
    // ellipses remplacerait un défaut par un autre.
    let (texte, ponctuation) = aphrody_ocr::kana::normalise_ponctuation(markdown);
    bilan.ponctuation += ponctuation;
    // Puis la demi-chasse : les katakana étroits n'existent pas dans un livre
    // imprimé, donc `ｶﾞ` ne peut vouloir dire que `ガ`. Après la ponctuation,
    // pour qu'un `･` isolé — un vrai séparateur — devienne `・` sans avoir été
    // pris pour le reste d'une ellipse.
    let (texte, demi_chasse) = aphrody_ocr::kana::normalise_demi_chasse(&texte);
    bilan.demi_chasse += demi_chasse;
    texte
}

/// Ce que les passes japonaises ont changé sur tout un fichier.
#[derive(Debug, Default)]
struct Bilan {
    /// Points de secours ramenés à une ellipse.
    ponctuation: usize,
    /// Caractères demi-chasse ramenés en pleine chasse.
    demi_chasse: usize,
    /// Espaces retirées d'entre deux caractères japonais.
    espaces: usize,
    /// Lignes recollées au milieu d'un mot.
    recolles: usize,
    /// Sosies typographiques remplacés, dictionnaire à l'appui.
    sosies: usize,
    /// Noms propres remis dans leur graphie, lexique à l'appui.
    noms: usize,
    /// Noms rétablis parce qu'un seul terme du lexique était à un kana près.
    voisins: usize,
}

impl Bilan {
    /// Rien n'a bougé.
    const fn vide(&self) -> bool {
        self.ponctuation == 0
            && self.demi_chasse == 0
            && self.espaces == 0
            && self.recolles == 0
            && self.sosies == 0
            && self.noms == 0
            && self.voisins == 0
    }
}

/// Ajoute au rapport les défauts que seul un dictionnaire japonais voit.
#[cfg(feature = "ocr-japanese")]
fn ajoute_findings_japonais(
    report: &mut aphrody_ocr::audit::AuditReport,
    pages: &[(PathBuf, Option<String>)],
    japonais: Option<&aphrody_ocr::japonais::Analyseur>,
) {
    let Some(analyseur) = japonais else { return };
    for (image, text) in pages {
        let Some(text) = text else { continue };
        let findings = aphrody_ocr::audit::audit_japonais(text, analyseur);
        if findings.is_empty() {
            continue;
        }
        // Une planche déjà signalée garde ses défauts : c'en est un de plus,
        // pas une entrée concurrente.
        if let Some(page) = report.flagged.iter_mut().find(|p| &p.image == image) {
            page.findings.extend(findings);
        } else {
            report.flagged.push(aphrody_ocr::audit::PageFindings {
                image: image.clone(),
                findings,
            });
        }
    }
}

#[cfg(not(feature = "ocr-japanese"))]
#[allow(clippy::needless_pass_by_value)]
fn ajoute_findings_japonais(
    _report: &mut aphrody_ocr::audit::AuditReport,
    _pages: &[(PathBuf, Option<String>)],
    _japonais: Option<&std::convert::Infallible>,
) {
}

fn clean(input: &Path, out: Option<&Path>, japonais: bool) -> miette::Result<()> {
    let japonais = japonais_optionnel(japonais)?;
    // Le lexique tourne toujours : ses entrées sont des fautes mesurées sur le
    // corpus, pas des devinettes, et chacune porte sa garde contre le mot
    // japonais légitime qui lui ressemble.
    let lexique = aphrody_ocr::lexique::Lexique::databooks_dragon_ball();
    let file = std::fs::File::open(input)
        .map_err(|e| miette::miette!("read {}: {e}", input.display()))?;

    let mut lines = Vec::new();
    let mut recleaned = 0_usize;
    let mut changed = 0_usize;
    let mut passthrough = 0_usize;
    let mut bilan = Bilan::default();

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

        let before = value
            .get("text")
            .and_then(|t| t.get("markdown"))
            .and_then(serde_json::Value::as_str)
            .unwrap_or_default()
            .to_owned();

        // `raw` is the better input — it still holds the markup a rule may
        // want to act on. But a batch run without `--raw` is not beyond help:
        // the rules that matter most in practice (a loop, a watermark) act on
        // the text itself, and re-parsing the markdown recovers them. That is
        // the difference between salvaging four hundred plates and re-reading
        // them on the GPU for an hour.
        let source = value
            .get("raw")
            .and_then(serde_json::Value::as_str)
            .map(ToOwned::to_owned)
            .unwrap_or_else(|| {
                passthrough += 1;
                before.clone()
            });
        if source.is_empty() {
            lines.push(value);
            continue;
        }

        let document = aphrody_ocr::Document::parse(&source);
        let after = document
            .to_markdown()
            .map(|m| remet_en_forme(&m, japonais.as_ref(), &lexique, &mut bilan));
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
        "{recleaned} page(s) recleaned ({changed} changed), {passthrough} re-parsed from their text for want of `raw` -> {}",
        target.display()
    );
    if !bilan.vide() {
        eprintln!(
            "japonais: {} ligne(s) recollée(s), {} espace(s) parasite(s) retirée(s), {} demi-chasse normalisée(s), {} ponctuation(s) remise(s) en forme, {} sosie(s) corrigé(s), {} nom(s) propre(s) rétabli(s), {} par voisinage du lexique",
            bilan.recolles,
            bilan.espaces,
            bilan.demi_chasse,
            bilan.ponctuation,
            bilan.sosies,
            bilan.noms,
            bilan.voisins
        );
    }
    if passthrough > 0 && recleaned == passthrough {
        eprintln!("note: no line carried `raw`, so only rules that act on the text itself could run; --raw keeps the markup a future rule may need");
    }
    Ok(())
}

/// Image paths already recorded in a JSONL file.
///
/// Infaillible : un fichier de reprise illisible signifie « rien de fait »,
/// jamais un échec de lot. Au pire, des planches sont relues — coûteux, pas
/// destructeur.
fn already_done(path: &Path) -> std::collections::BTreeSet<PathBuf> {
    // No file yet — or one that cannot be read — simply means nothing is done.
    let Ok(file) = std::fs::File::open(path) else {
        return std::collections::BTreeSet::new();
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
    done
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

        let done = already_done(&jsonl);
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
        let done = already_done(&jsonl);
        assert_eq!(done, [PathBuf::from("a.jpg")].into_iter().collect());
    }

    #[test]
    fn an_absent_resume_file_means_nothing_is_done() {
        let dir = tempfile::tempdir().unwrap();
        assert!(already_done(&dir.path().join("absent.jsonl")).is_empty());
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
        clean(&jsonl, Some(&out), false).unwrap();

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
        clean(&jsonl, Some(&out), false).unwrap();

        // A maintenance command must never silently lose pages.
        let written = std::fs::read_to_string(&out).unwrap();
        assert_eq!(written.lines().count(), 2, "{written}");
        assert!(written.contains("gardé"), "{written}");
    }

    #[test]
    fn cleaning_salvages_a_looping_page_that_never_kept_its_raw_output() {
        // The four lots refused at audit were read without `--raw`. Re-reading
        // them on the GPU costs an hour; re-parsing their text costs nothing,
        // and the rule that matters here acts on the text anyway.
        let dir = tempfile::tempdir().unwrap();
        let jsonl = dir.path().join("in.jsonl");
        let looped = format!("Chapitre 26 {}", "DRAGONBALL ".repeat(129));
        std::fs::write(
            &jsonl,
            format!(
                "{{\"image\":\"26-0021.jpg\",\"text\":{{\"kind\":\"text\",\"markdown\":\"{}\"}}}}\n",
                looped.trim()
            ),
        )
        .unwrap();

        let out = dir.path().join("out.jsonl");
        clean(&jsonl, Some(&out), false).unwrap();

        let written = std::fs::read_to_string(&out).unwrap();
        assert!(written.contains("Chapitre 26"), "the good prefix survives: {written}");
        assert_eq!(written.matches("DRAGONBALL").count(), 1, "the loop is cut: {written}");
    }

    #[test]
    fn cleaning_defaults_to_rewriting_the_input() {
        let dir = tempfile::tempdir().unwrap();
        let jsonl = dir.path().join("in.jsonl");
        std::fs::write(&jsonl, "{\"image\":\"a.jpg\",\"raw\":\"texte réel\"}\n").unwrap();
        clean(&jsonl, None, false).unwrap();
        let written = std::fs::read_to_string(&jsonl).unwrap();
        assert!(written.contains("texte réel"), "{written}");
    }
}

/// `aphrody ocr bulles` — read the plates a page-level run found silent.
///
/// # Why a separate command
///
/// It asks a different question than `batch`. `batch` asks what a page says;
/// on a comics page a document model answers nothing, because the lettering is
/// a detail inside what it reads as a drawing. This asks what the *balloons*
/// say, which the same model answers well once each balloon is its own image.
///
/// Measured on twelve plates the page-level pipeline reported silent: 44
/// balloons recovered across nine of them, carrying real dialogue. The cost is
/// roughly one page-read per plate, because four fifths of the detected
/// regions are bright areas of drawing that read as nothing.
///
/// # Why its output is not ready to deposit
///
/// A region holding a fragment of a hand-drawn sound effect makes the model
/// produce plausible wrong characters rather than nothing — one came back as
/// `禁 幸`. Run `aphrody ocr audit --japonais` over this output before it goes
/// anywhere near a corpus.
#[cfg(feature = "ocr-bulles")]
#[allow(clippy::too_many_arguments)]
fn bulles(
    input: &Path,
    images: Option<&Path>,
    out: &Path,
    skip_done: bool,
    limit: Option<usize>,
    decoupes: Option<&Path>,
    cible: u32,
    max_bulles: usize,
    opts: OcrOptions,
    resident: Option<aphrody_ocr::server::ServerConfig>,
) -> miette::Result<()> {
    let muettes = planches_sans_texte(input, images).map_err(|e| miette::miette!("{e}"))?;
    // La reprise se decide sur le NOM de la planche, pas sur son chemin.
    // `batch` peut comparer des chemins parce qu'il relit son propre
    // repertoire ; ici l'entree est un JSONL et les planches sont re-enracinees
    // sous `--images`, donc lancer la reprise avec un chemin absolu la ou la
    // premiere passe en avait un relatif suffit a ne rien reconnaitre — et a
    // tout relire en silence. Mesure : 448 planches relues pour rien.
    let deja: std::collections::BTreeSet<std::ffi::OsString> = if skip_done {
        already_done(out)
            .iter()
            .filter_map(|p| p.file_name().map(std::ffi::OsStr::to_os_string))
            .collect()
    } else {
        std::collections::BTreeSet::new()
    };
    let queue: Vec<PathBuf> = muettes
        .into_iter()
        .filter(|p| !p.file_name().is_some_and(|n| deja.contains(n)))
        .take(limit.unwrap_or(usize::MAX))
        .collect();

    eprintln!(
        "{} silent plate(s) to re-read{}",
        queue.len(),
        if deja.is_empty() { String::new() } else { format!(", {} already done", deja.len()) }
    );
    if queue.is_empty() {
        return Ok(());
    }

    let reglages =
        aphrody_ocr::bulles::ReglagesBulles { maximum: max_bulles, ..Default::default() };

    if let Some(config) = resident {
        eprintln!(
            "starting llama-server on 127.0.0.1:{} ({} slot(s) x {} tokens)…",
            config.port, config.slots, config.ctx_per_slot
        );
        let runner = ServerRunner::start(opts, config).map_err(|e| miette::miette!("{e}"))?;
        let workers = runner.slots() as usize;
        return boucle_bulles(&queue, out, &reglages, cible, decoupes, workers, &|image| {
            runner.read(image)
        })
        .map_err(|e| miette::miette!("{e}"));
    }

    let runner = VlmRunner::new(opts).map_err(|e| miette::miette!("{e}"))?;
    boucle_bulles(&queue, out, &reglages, cible, decoupes, 1, &|image| runner.read(image))
        .map_err(|e| miette::miette!("{e}"))
}

/// The plates a JSONL records as carrying no text.
///
/// Paths are taken from the file, then re-rooted under `images` when given: a
/// JSONL written on one machine names plates relative to where that run was
/// launched, which is rarely where it is re-read.
#[cfg(feature = "ocr-bulles")]
fn planches_sans_texte(input: &Path, images: Option<&Path>) -> std::io::Result<Vec<PathBuf>> {
    let file = std::fs::File::open(input)?;
    let mut out = Vec::new();
    for ligne in std::io::BufReader::new(file).lines() {
        let ligne = ligne?;
        let Ok(v) = serde_json::from_str::<serde_json::Value>(&ligne) else { continue };
        if v.get("text").and_then(|t| t.get("kind")).and_then(serde_json::Value::as_str)
            != Some("none")
        {
            continue;
        }
        let Some(image) = v.get("image").and_then(serde_json::Value::as_str) else { continue };
        // Un JSONL écrit sous Windows porte des antislashs. Le relire ailleurs
        // ne doit pas dépendre du séparateur de la machine qui l'a produit.
        let normalise = image.replace('\\', "/");
        let chemin = match images {
            Some(racine) => racine.join(normalise.rsplit('/').next().unwrap_or(&normalise)),
            None => PathBuf::from(&normalise),
        };
        out.push(chemin);
    }
    Ok(out)
}

/// Read every plate in `queue` balloon by balloon, one JSONL line each.
///
/// Crops go to disk because both backends take a path — the resident server
/// wants a file to encode, the per-process one wants an argv. They are deleted
/// with their directory unless `decoupes` asks to keep them, which is how a
/// surprising reading gets checked against the pixels that produced it.
#[cfg(feature = "ocr-bulles")]
fn boucle_bulles(
    queue: &[PathBuf],
    out: &Path,
    reglages: &aphrody_ocr::bulles::ReglagesBulles,
    cible: u32,
    decoupes: Option<&Path>,
    workers: usize,
    read_page: &(dyn Fn(&Path) -> aphrody_ocr::Result<PageResult> + Sync),
) -> aphrody_ocr::Result<()> {
    let mut sink = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(out)
        .map_err(|source| aphrody_ocr::OcrError::Io { path: out.to_path_buf(), source })?;

    let started = std::time::Instant::now();
    let mut avec_texte = 0_usize;
    let mut bulles_lues = 0_usize;
    let mut echouees = 0_usize;

    for (rang, planche) in queue.iter().enumerate() {
        let debut = std::time::Instant::now();
        let (texte, lues) =
            match lit_une_planche(planche, reglages, cible, decoupes, workers, read_page) {
                Ok(pair) => pair,
                Err(e) => {
                    eprintln!("plate failed ({}): {e}", planche.display());
                    echouees += 1;
                    continue;
                }
            };
        bulles_lues += lues;
        let page = PageResult {
            image: planche.clone(),
            text: texte,
            elapsed_ms: debut.elapsed().as_millis(),
            raw: None,
        };
        if page.text.has_text() {
            avec_texte += 1;
        }
        let ligne = serde_json::to_string(&page).unwrap_or_default();
        writeln!(sink, "{ligne}")
            .map_err(|source| aphrody_ocr::OcrError::Io { path: out.to_path_buf(), source })?;
        let _ = sink.flush();

        if (rang + 1) % 10 == 0 || rang + 1 == queue.len() {
            let par_planche = started.elapsed().as_secs_f64() / (rang + 1) as f64;
            eprintln!(
                "  {}/{} plate(s), {avec_texte} recovered, {bulles_lues} balloon(s) read, {par_planche:.1}s/plate",
                rang + 1,
                queue.len(),
            );
        }
    }

    let relues = queue.len() - echouees;
    eprintln!(
        "\n{relues}/{} plate(s) re-read in {:.1}s — {avec_texte} recovered text, {} still silent, {echouees} failed",
        queue.len(),
        started.elapsed().as_secs_f64(),
        relues - avec_texte
    );
    Ok(())
}

/// Detect, crop, read and recompose one plate.
///
/// Returns the recomposed text and how many balloons were read, so a caller
/// can tell "no balloon found" from "balloons found, none of them readable".
///
/// The crops live in a directory rather than in memory because both backends
/// take a path: the resident server wants a file to encode, the per-process
/// one wants an argv. Unless `--decoupes` asks to keep them, that directory is
/// under the system temp root and is removed as soon as the plate is read.
#[cfg(feature = "ocr-bulles")]
fn lit_une_planche(
    planche: &Path,
    reglages: &aphrody_ocr::bulles::ReglagesBulles,
    cible: u32,
    decoupes: Option<&Path>,
    workers: usize,
    read_page: &(dyn Fn(&Path) -> aphrody_ocr::Result<PageResult> + Sync),
) -> aphrody_ocr::Result<(aphrody_ocr::PageText, usize)> {
    let tige = planche.file_stem().unwrap_or_default().to_string_lossy().into_owned();
    let (dossier, ephemere) = match decoupes {
        Some(racine) => (racine.join(&tige), false),
        // Le pid dans le nom : deux lots lancés en parallèle sur la même
        // machine liraient sinon les découpes l'un de l'autre.
        None => (
            std::env::temp_dir().join(format!("aphrody-bulles-{}-{tige}", std::process::id())),
            true,
        ),
    };
    std::fs::create_dir_all(&dossier)
        .map_err(|source| aphrody_ocr::OcrError::Io { path: dossier.clone(), source })?;

    let resultat = (|| {
        let chemins =
            aphrody_ocr::bulles::decoupe_planche(planche, &dossier, reglages, cible)?;
        if chemins.is_empty() {
            return Ok((aphrody_ocr::PageText::None, 0));
        }
        let lues = chemins.len();
        let assemble: Vec<String> =
            lit_en_parallele(&chemins, workers, read_page).into_iter().flatten().collect();
        if assemble.is_empty() {
            return Ok((aphrody_ocr::PageText::None, lues));
        }
        // Une ligne vide entre deux bulles : chacune est une réplique
        // distincte, et les recoller sans séparation en ferait une phrase que
        // personne n'a dite.
        Ok((aphrody_ocr::PageText::Text { markdown: assemble.join("

") }, lues))
    })();

    // Nettoyer même quand la lecture a échoué : sur onze mille planches, des
    // découpes oubliées à chaque échec finissent par remplir le disque.
    if ephemere {
        let _ = std::fs::remove_dir_all(&dossier);
    }
    resultat
}

/// Read `chemins` with `workers` in flight, keeping the input order.
///
/// Order matters here in a way it does not for a page batch: the crops arrive
/// from `ordonne_lecture` already in reading order, and shuffling them would
/// scramble the dialogue.
#[cfg(feature = "ocr-bulles")]
fn lit_en_parallele(
    chemins: &[PathBuf],
    workers: usize,
    read_page: &(dyn Fn(&Path) -> aphrody_ocr::Result<PageResult> + Sync),
) -> Vec<Option<String>> {
    use std::sync::Mutex;
    use std::sync::atomic::{AtomicUsize, Ordering};

    let resultats: Vec<Mutex<Option<String>>> =
        (0..chemins.len()).map(|_| Mutex::new(None)).collect();
    let suivant = AtomicUsize::new(0);

    std::thread::scope(|scope| {
        for _ in 0..workers.max(1) {
            scope.spawn(|| {
                loop {
                    let i = suivant.fetch_add(1, Ordering::Relaxed);
                    let Some(chemin) = chemins.get(i) else { break };
                    let texte = read_page(chemin)
                        .ok()
                        .and_then(|p| p.text.markdown().map(str::to_owned))
                        .filter(|t| !t.trim().is_empty());
                    if let Ok(mut slot) = resultats[i].lock() {
                        *slot = texte;
                    }
                }
            });
        }
    });

    resultats.into_iter().map(|m| m.into_inner().unwrap_or(None)).collect()
}
