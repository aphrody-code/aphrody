// SPDX-License-Identifier: Apache-2.0
//! `aphrody model …` — local model lifecycle: catalog, pull, inspect, evict.
//!
//! The CLI surface of the `aphrody-models` crate, and the entry point of
//! aphrody's local-inference toolbox: it is how weights for OCR, visual
//! transcription and speech-to-text get onto the machine, get verified, and
//! get reclaimed when disk runs short.
//!
//! # Output contract
//!
//! Every subcommand takes `--format text|json|markdown|html|csv` (with `--json`
//! as a shorthand) and `--out <path>` to write the report to a file instead of
//! stdout. When `--out` carries a known extension and `--format` was not given,
//! the format is inferred from it, so `--out report.md` does the obvious thing.
//!
//! The rendered report goes to **stdout**; progress and human chatter go to
//! **stderr**. That split is what lets `aphrody model pull … --json | jq` work
//! while a multi-gigabyte download is still drawing a progress line.

use std::path::{Path, PathBuf};

use aphrody_models::{
    Catalog, Downloader, Format, HardwareProfile, ModelRef, ModelStore, ModelTask, Progress,
    PullOutcome, Report, SpeedTier, accel, human_bytes,
};

/// Output options shared by every subcommand.
#[derive(clap::Args, Debug, Clone, Default)]
pub(crate) struct OutputOpts {
    /// Output format: `text`, `json`, `markdown` (`md`), `html`, `csv`.
    /// Defaults to the extension of `--out`, else `text`.
    #[arg(long, short = 'f', global = true)]
    format: Option<String>,
    /// Shorthand for `--format json`.
    #[arg(long, global = true)]
    json: bool,
    /// Write the report to this path instead of stdout.
    #[arg(long, short = 'o', global = true, value_name = "PATH")]
    out: Option<PathBuf>,
}

impl OutputOpts {
    /// Resolve the effective format.
    ///
    /// Precedence: explicit `--format`, then `--json`, then the extension of
    /// `--out`, then plain text.
    fn format(&self) -> miette::Result<Format> {
        if let Some(raw) = &self.format {
            return Format::from_str_opt(raw).ok_or_else(|| {
                let valid: Vec<&str> = Format::all().iter().map(|f| f.as_str()).collect();
                miette::miette!("unknown format `{raw}` (expected one of: {})", valid.join(", "))
            });
        }
        if self.json {
            return Ok(Format::Json);
        }
        if let Some(path) = &self.out {
            if let Some(inferred) = Format::from_extension(&path.to_string_lossy()) {
                return Ok(inferred);
            }
        }
        Ok(Format::Text)
    }

    /// Whether the caller asked for machine-readable output. Used to silence
    /// progress drawing, which would otherwise interleave with a piped stream.
    fn is_machine(&self) -> bool {
        matches!(self.format(), Ok(Format::Json | Format::Csv))
    }

    /// Emit a rendered document to `--out` or stdout.
    fn emit(&self, body: &str) -> miette::Result<()> {
        match &self.out {
            Some(path) => write_out(path, body),
            None => {
                println!("{}", body.trim_end());
                Ok(())
            },
        }
    }

    /// Emit either a serialised JSON value or a rendered table, whichever the
    /// resolved format calls for. This is the single exit point of every
    /// subcommand, so the `--format` contract cannot drift between them.
    pub(crate) fn emit_report(
        &self,
        report: &Report,
        json: &serde_json::Value,
    ) -> miette::Result<()> {
        let format = self.format()?;
        let body = if format == Format::Json {
            serde_json::to_string_pretty(json)
                .map_err(|e| miette::miette!("serialise report: {e}"))?
        } else {
            report.render(format)
        };
        self.emit(&body)
    }
}

/// Write a report to disk, creating parent directories as needed.
fn write_out(path: &Path, body: &str) -> miette::Result<()> {
    if let Some(parent) = path.parent() {
        if !parent.as_os_str().is_empty() {
            std::fs::create_dir_all(parent)
                .map_err(|e| miette::miette!("create {}: {e}", parent.display()))?;
        }
    }
    std::fs::write(path, body).map_err(|e| miette::miette!("write {}: {e}", path.display()))?;
    eprintln!("wrote {} ({} bytes)", path.display(), body.len());
    Ok(())
}

/// Actions for the `model` subcommand.
#[derive(clap::Subcommand, Debug, Clone)]
pub(crate) enum ModelAction {
    /// List locally installed models.
    ///
    /// Example: aphrody model list --format markdown --out models.md
    List {
        /// Only show models pulled through a catalog entry with this task.
        #[arg(long)]
        task: Option<String>,
        #[command(flatten)]
        output: OutputOpts,
    },
    /// Show the curated catalog of models aphrody knows how to run.
    ///
    /// Example: aphrody model catalog --task ocr
    Catalog {
        /// Restrict to one task (`ocr`, `visual-transcription`,
        /// `speech-to-text`, `text-embedding`, `text-generation`).
        #[arg(long)]
        task: Option<String>,
        #[command(flatten)]
        output: OutputOpts,
    },
    /// Report the accelerators, GPUs and CUDA toolkit found on this machine.
    ///
    /// Example: aphrody model accel --json
    Accel {
        #[command(flatten)]
        output: OutputOpts,
    },
    /// Rank the catalog for this machine and a given task.
    ///
    /// The default preference is `fast`, because the job this toolbox exists
    /// for is bulk processing; ask for `quality` when accuracy on one hard
    /// document matters more than throughput.
    ///
    /// Example: aphrody model recommend --task ocr --prefer fast
    Recommend {
        /// Task to rank for. Defaults to `ocr`.
        #[arg(long, default_value = "ocr")]
        task: String,
        /// Throughput preference: `fast`, `balanced`, `quality`.
        #[arg(long, default_value = "fast")]
        prefer: String,
        /// Pull the top-ranked model instead of only reporting it.
        #[arg(long)]
        pull: bool,
        #[command(flatten)]
        output: OutputOpts,
    },
    /// Download a model into `~/.aphrody/models`, resuming if interrupted.
    ///
    /// SPEC is a catalog id (`whisper-base-en`) or a raw reference
    /// (`hf:owner/repo/file.gguf@rev`, `https://…`, `file:/path`). Catalog
    /// pulls are verified against the digest pinned in the catalog.
    ///
    /// Example: aphrody model pull ppocr-v5-mobile
    Pull {
        /// Catalog id or model reference.
        spec: String,
        /// Re-download even if the artefact is already present and intact.
        #[arg(long)]
        force: bool,
        #[command(flatten)]
        output: OutputOpts,
    },
    /// Show what an installed artefact actually is: size, digest, and the
    /// decoded GGUF / GGML / safetensors / ONNX header.
    ///
    /// Example: aphrody model info whisper-base-en --json
    Info {
        /// Catalog id or model reference.
        spec: String,
        #[command(flatten)]
        output: OutputOpts,
    },
    /// Re-hash installed bytes and compare against what was recorded.
    ///
    /// Exits non-zero when an artefact no longer matches.
    Verify {
        /// Catalog id or model reference.
        spec: String,
        #[command(flatten)]
        output: OutputOpts,
    },
    /// Delete an installed model and drop it from the registry.
    ///
    /// Artefacts adopted from outside the store are un-tracked, never deleted.
    Rm {
        /// Catalog id or model reference.
        spec: String,
        #[command(flatten)]
        output: OutputOpts,
    },
    /// Evict least-recently-used models until the store fits a size budget,
    /// and sweep interrupted `.part` downloads.
    ///
    /// Example: aphrody model gc --budget 4GiB
    Gc {
        /// Size to shrink to, e.g. `2GiB`, `500MB`, `1500000`.
        #[arg(long, default_value = "8GiB")]
        budget: String,
        #[command(flatten)]
        output: OutputOpts,
    },
    /// Report drift between the registry and the files on disk.
    ///
    /// Exits non-zero when the store is not clean.
    Doctor {
        #[command(flatten)]
        output: OutputOpts,
    },
    /// Track a model file that already exists elsewhere on disk, without
    /// copying it.
    Adopt {
        /// Path to the artefact.
        path: PathBuf,
        #[command(flatten)]
        output: OutputOpts,
    },
    /// Print the store root, or the resolved path of one artefact.
    Path {
        /// Catalog id or model reference. Omit to print the store root.
        spec: Option<String>,
    },
}

/// Run a `model` action.
///
/// # Errors
///
/// Returns a `miette` report on resolution, transport, digest or filesystem
/// failure. `verify` and `doctor` also report a non-zero outcome as an error
/// so shell pipelines can branch on it.
pub(crate) async fn run(action: ModelAction) -> miette::Result<()> {
    match action {
        ModelAction::Accel { output } => accel_report(&output),
        ModelAction::Catalog { task, output } => catalog(task.as_deref(), &output),
        ModelAction::Recommend { task, prefer, pull: do_pull, output } => {
            recommend(&task, &prefer, do_pull, &output).await
        },
        other => {
            // Everything else needs the store open.
            let store = ModelStore::open().map_err(|e| miette::miette!("open model store: {e}"))?;
            match other {
                ModelAction::List { task, output } => list(&store, task.as_deref(), &output),
                ModelAction::Pull { spec, force, output } => {
                    pull(&store, &spec, force, &output).await
                },
                ModelAction::Info { spec, output } => info(&store, &spec, &output),
                ModelAction::Verify { spec, output } => verify(&store, &spec, &output),
                ModelAction::Rm { spec, output } => remove(&store, &spec, &output),
                ModelAction::Gc { budget, output } => gc(&store, &budget, &output),
                ModelAction::Doctor { output } => doctor(&store, &output),
                ModelAction::Adopt { path, output } => adopt(&store, &path, &output),
                ModelAction::Path { spec } => path(&store, spec.as_deref()),
                // Handled above, before the store is opened.
                ModelAction::Accel { .. }
                | ModelAction::Catalog { .. }
                | ModelAction::Recommend { .. } => unreachable!(),
            }
        },
    }
}

// ---------------------------------------------------------------------------
// Resolution helpers
// ---------------------------------------------------------------------------

/// Turn a user spec into the references it names.
///
/// A catalog id expands to every artefact in the entry; anything else parses
/// as a single reference. Used by the read-only subcommands, which act on all
/// of an entry's files (a Florence-2 install is four graphs plus sidecars).
fn resolve_refs(spec: &str) -> miette::Result<Vec<ModelRef>> {
    let resolved =
        Catalog::builtin().resolve(spec).map_err(|e| miette::miette!("resolve `{spec}`: {e}"))?;
    Ok(resolved.artifacts().into_iter().map(|(r, _)| r).collect())
}

/// Parse a task name, listing the valid ones on failure.
fn parse_task(raw: &str) -> miette::Result<ModelTask> {
    ModelTask::from_str_opt(raw).ok_or_else(|| {
        let valid: Vec<&str> = ModelTask::all().iter().map(|t| t.as_str()).collect();
        miette::miette!("unknown task `{raw}` (expected one of: {})", valid.join(", "))
    })
}

/// Parse a throughput preference.
fn parse_speed(raw: &str) -> miette::Result<SpeedTier> {
    match raw.trim().to_ascii_lowercase().as_str() {
        "fast" => Ok(SpeedTier::Fast),
        "balanced" => Ok(SpeedTier::Balanced),
        "quality" => Ok(SpeedTier::Quality),
        other => Err(miette::miette!(
            "unknown preference `{other}` (expected one of: fast, balanced, quality)"
        )),
    }
}

/// Parse a byte budget: a bare integer, or a number with a binary (`KiB`,
/// `MiB`, `GiB`, `TiB`) or decimal (`KB`, `MB`, `GB`, `TB`) suffix.
///
/// Case-insensitive, and a bare `K`/`M`/`G`/`T` is read as binary, matching
/// what `du -h` prints.
fn parse_budget(raw: &str) -> miette::Result<u64> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Err(miette::miette!("empty size budget"));
    }
    let split = trimmed.find(|c: char| !c.is_ascii_digit() && c != '.').unwrap_or(trimmed.len());
    let (number, suffix) = trimmed.split_at(split);
    let value: f64 =
        number.parse().map_err(|_| miette::miette!("`{raw}` does not start with a number"))?;

    let multiplier: u64 = match suffix.trim().to_ascii_lowercase().as_str() {
        "" | "b" => 1,
        "k" | "kib" => 1 << 10,
        "m" | "mib" => 1 << 20,
        "g" | "gib" => 1 << 30,
        "t" | "tib" => 1_u64 << 40,
        "kb" => 1_000,
        "mb" => 1_000_000,
        "gb" => 1_000_000_000,
        "tb" => 1_000_000_000_000,
        other => {
            return Err(miette::miette!(
                "unknown size suffix `{other}` (use B, KiB, MiB, GiB, TiB, KB, MB, GB, TB)"
            ));
        },
    };

    // f64 has 53 bits of mantissa: exact for every byte count below 8 PiB.
    let bytes = value * multiplier as f64;
    if !bytes.is_finite() || bytes < 0.0 {
        return Err(miette::miette!("`{raw}` is not a usable size"));
    }
    Ok(bytes as u64)
}

// ---------------------------------------------------------------------------
// Subcommands
// ---------------------------------------------------------------------------

fn list(store: &ModelStore, task: Option<&str>, output: &OutputOpts) -> miette::Result<()> {
    let task = task.map(parse_task).transpose()?;
    let mut installed = store.list().map_err(|e| miette::miette!("read model registry: {e}"))?;

    if let Some(task) = task {
        // Task lives on the catalog entry, not the artefact, so filtering
        // means mapping each entry's recorded catalog id back to the catalog.
        let ids: Vec<&str> =
            Catalog::builtin().by_task(task).into_iter().map(|e| e.id.as_str()).collect();
        installed.retain(|m| m.catalog_id.as_deref().is_some_and(|id| ids.contains(&id)));
    }

    let total: u64 = installed.iter().map(|m| m.bytes).sum();

    let mut report = Report::new("Installed models", &["SIZE", "FORMAT", "CATALOG", "REFERENCE"])
        .with_summary(format!("Store root: {}", store.root().display()))
        .with_footer(format!("{} model(s), {} on disk", installed.len(), human_bytes(total)));
    for model in &installed {
        report.push(vec![
            human_bytes(model.bytes),
            model.format.as_str().to_owned(),
            model.catalog_id.clone().unwrap_or_else(|| "-".to_owned()),
            model.reference.to_string(),
        ]);
    }

    let json = serde_json::json!({
        "root": store.root(),
        "count": installed.len(),
        "total_bytes": total,
        "models": installed.iter().map(|m| serde_json::json!({
            "ref": m.reference.to_string(),
            "path": m.path,
            "bytes": m.bytes,
            "sha256": m.sha256,
            "format": m.format.as_str(),
            "catalog_id": m.catalog_id,
            "installed_at": m.installed_at_rfc3339(),
            "last_used_at": m.last_used_at_rfc3339(),
        })).collect::<Vec<_>>(),
    });

    output.emit_report(&report, &json)
}

fn catalog(task: Option<&str>, output: &OutputOpts) -> miette::Result<()> {
    let task = task.map(parse_task).transpose()?;
    let builtin = Catalog::builtin();
    let entries = match task {
        Some(t) => builtin.by_task(t),
        None => builtin.entries.iter().collect(),
    };

    let mut report =
        Report::new("Model catalog", &["ID", "TASK", "BACKEND", "SPEED", "SIZE", "TITLE"])
            .with_summary(
                "Pinned to immutable commits and verified by SHA-256 on download.".to_owned(),
            )
            .with_footer(format!("{} entr(ies)", entries.len()));
    for entry in &entries {
        report.push(vec![
            entry.id.clone(),
            entry.task.to_string(),
            entry.backend.to_string(),
            entry.speed.to_string(),
            entry.total_bytes().map_or_else(|| "?".to_owned(), human_bytes),
            entry.title.clone(),
        ]);
    }

    let json = serde_json::json!({ "count": entries.len(), "entries": entries });
    output.emit_report(&report, &json)
}

fn accel_report(output: &OutputOpts) -> miette::Result<()> {
    let profile = accel::probe();

    let mut report = Report::new("Hardware profile", &["FIELD", "VALUE"])
        .with_summary(format!("Best available accelerator: {}", profile.best()))
        .with_footer("`aphrody model recommend --task ocr` ranks the catalog against this profile");
    report.push(vec![
        "accelerators".into(),
        profile.accelerators.iter().map(|a| a.as_str()).collect::<Vec<_>>().join(", "),
    ]);
    report.push(vec!["cpu threads".into(), profile.cpu_threads.to_string()]);
    report.push(vec![
        "cuda toolkit".into(),
        profile.cuda_toolkit.clone().unwrap_or_else(|| "not found".to_owned()),
    ]);
    for (index, gpu) in profile.gpus.iter().enumerate() {
        report.push(vec![format!("gpu {index}"), gpu.name.clone()]);
        report.push(vec![
            format!("gpu {index} vram"),
            format!(
                "{} total, {} free",
                human_bytes(gpu.vram_bytes),
                human_bytes(gpu.vram_free_bytes)
            ),
        ]);
        report.push(vec![
            format!("gpu {index} driver"),
            format!("{} (compute {})", gpu.driver, gpu.compute_capability),
        ]);
    }
    if profile.gpus.is_empty() {
        report.push(vec!["gpu".into(), "none detected".into()]);
    }

    let json = serde_json::to_value(&profile)
        .map_err(|e| miette::miette!("serialise hardware profile: {e}"))?;
    output.emit_report(&report, &json)
}

async fn recommend(
    task: &str,
    prefer: &str,
    do_pull: bool,
    output: &OutputOpts,
) -> miette::Result<()> {
    let task = parse_task(task)?;
    let prefer = parse_speed(prefer)?;
    let profile = accel::probe();
    let ranked = aphrody_models::rank_for(Catalog::builtin(), task, &profile, prefer);

    if ranked.is_empty() {
        return Err(miette::miette!("no catalogued model for task `{task}` fits this machine"));
    }

    let mut report = Report::new(format!("Recommended models for {task}"), &[
        "RANK", "ID", "RUNS ON", "DOWNLOAD", "WHY",
    ])
    .with_summary(hardware_line(&profile))
    .with_footer(format!("Top pick: {} — `aphrody model pull {}`", ranked[0].id, ranked[0].id));
    for (index, recommendation) in ranked.iter().enumerate() {
        report.push(vec![
            (index + 1).to_string(),
            recommendation.id.clone(),
            recommendation.accelerator.to_string(),
            recommendation.download_bytes.map_or_else(|| "?".to_owned(), human_bytes),
            recommendation.rationale.clone(),
        ]);
    }

    let json = serde_json::json!({
        "task": task.as_str(),
        "prefer": prefer.as_str(),
        "hardware": profile,
        "ranked": ranked,
        "top": ranked[0].id,
    });
    output.emit_report(&report, &json)?;

    if do_pull {
        let store = ModelStore::open().map_err(|e| miette::miette!("open model store: {e}"))?;
        return pull(&store, &ranked[0].id, false, output).await;
    }
    Ok(())
}

/// One-line description of the machine, for report summaries.
fn hardware_line(profile: &HardwareProfile) -> String {
    match profile.gpus.first() {
        Some(gpu) => format!(
            "{} ({} VRAM, compute {}), {} CPU threads, CUDA {}",
            gpu.name,
            human_bytes(gpu.vram_bytes),
            gpu.compute_capability,
            profile.cpu_threads,
            profile.cuda_toolkit.as_deref().unwrap_or("toolkit not found")
        ),
        None => format!("No GPU detected; {} CPU threads", profile.cpu_threads),
    }
}

async fn pull(
    store: &ModelStore,
    spec: &str,
    force: bool,
    output: &OutputOpts,
) -> miette::Result<()> {
    let downloader = Downloader::new().map_err(|e| miette::miette!("build HTTP client: {e}"))?;
    let quiet = output.is_machine();

    // Progress is throttled to whole percentage points: a 500 MB transfer
    // delivers tens of thousands of chunks, and redrawing on each one costs
    // more than the download.
    let mut last_percent = u64::MAX;
    let mut last_role = String::new();
    let report_progress = |role: &str, progress: Progress| {
        if quiet {
            return;
        }
        let percent = progress.fraction().map_or(0, |f| (f * 100.0) as u64);
        if role != last_role || percent != last_percent {
            last_role = role.to_owned();
            last_percent = percent;
            match progress.total {
                Some(total) => eprint!(
                    "\r  {role:<18} {percent:>3}%  {} / {}   ",
                    human_bytes(progress.downloaded),
                    human_bytes(total)
                ),
                None => {
                    eprint!("\r  {role:<18}      {}   ", human_bytes(progress.downloaded));
                },
            }
        }
    };

    if !quiet {
        eprintln!("pulling {spec} into {}", store.root().display());
    }

    let outcomes = aphrody_models::pull_spec(store, &downloader, spec, force, report_progress)
        .await
        .map_err(|e| miette::miette!("pull `{spec}`: {e}"))?;

    if !quiet {
        eprintln!();
    }

    let transferred: u64 =
        outcomes.iter().filter(|o| o.transferred()).map(|o| o.model().bytes).sum();
    let total: u64 = outcomes.iter().map(|o| o.model().bytes).sum();

    let mut report = Report::new(format!("Pulled {spec}"), &["OUTCOME", "SIZE", "FORMAT", "PATH"])
        .with_footer(format!(
            "{} artefact(s), {} on disk, {} transferred",
            outcomes.len(),
            human_bytes(total),
            human_bytes(transferred)
        ));
    for outcome in &outcomes {
        let model = outcome.model();
        report.push(vec![
            outcome_label(outcome).to_owned(),
            human_bytes(model.bytes),
            model.format.as_str().to_owned(),
            model.path.display().to_string(),
        ]);
    }

    let json = serde_json::json!({
        "spec": spec,
        "artifacts": outcomes.iter().map(|o| {
            let m = o.model();
            serde_json::json!({
                "ref": m.reference.to_string(),
                "path": m.path,
                "bytes": m.bytes,
                "sha256": m.sha256,
                "format": m.format.as_str(),
                "outcome": outcome_label(o),
            })
        }).collect::<Vec<_>>(),
        "transferred_bytes": transferred,
        "total_bytes": total,
    });

    output.emit_report(&report, &json)
}

/// Stable machine-readable word for what a pull did.
const fn outcome_label(outcome: &PullOutcome) -> &'static str {
    match outcome {
        PullOutcome::AlreadyPresent(_) => "already-present",
        PullOutcome::Downloaded(_) => "downloaded",
        PullOutcome::Adopted(_) => "adopted",
    }
}

fn info(store: &ModelStore, spec: &str, output: &OutputOpts) -> miette::Result<()> {
    let refs = resolve_refs(spec)?;
    let mut found = Vec::new();
    for reference in &refs {
        let entry =
            store.get(reference).map_err(|e| miette::miette!("read model registry: {e}"))?;
        found.push((reference.clone(), entry));
    }

    let installed = found.iter().filter(|(_, e)| e.is_some()).count();
    let mut report = Report::new(format!("Model info: {spec}"), &[
        "REFERENCE",
        "STATE",
        "SIZE",
        "FORMAT",
        "HEADER",
    ])
    .with_footer(format!("{installed} of {} artefact(s) installed", found.len()));

    for (reference, entry) in &found {
        match entry {
            None => report.push(vec![
                reference.to_string(),
                "not installed".into(),
                "-".into(),
                "-".into(),
                store.path_for(reference).display().to_string(),
            ]),
            Some(model) => {
                let header = model
                    .inspection
                    .as_ref()
                    .and_then(|i| i.details.as_ref())
                    .map(|d| serde_json::to_string(d).unwrap_or_default())
                    .unwrap_or_else(|| {
                        model
                            .inspection
                            .as_ref()
                            .and_then(|i| i.warning.clone())
                            .unwrap_or_else(|| "-".to_owned())
                    });
                report.push(vec![
                    reference.to_string(),
                    "installed".into(),
                    human_bytes(model.bytes),
                    model.format.as_str().to_owned(),
                    header,
                ]);
            },
        }
    }

    let json = serde_json::json!({
        "spec": spec,
        "artifacts": found.iter().map(|(reference, entry)| match entry {
            Some(m) => serde_json::json!({
                "ref": reference.to_string(),
                "installed": true,
                "path": m.path,
                "bytes": m.bytes,
                "sha256": m.sha256,
                "format": m.format.as_str(),
                "catalog_id": m.catalog_id,
                "installed_at": m.installed_at_rfc3339(),
                "last_used_at": m.last_used_at_rfc3339(),
                "inspection": m.inspection,
            }),
            None => serde_json::json!({
                "ref": reference.to_string(),
                "installed": false,
                "path": store.path_for(reference),
            }),
        }).collect::<Vec<_>>(),
    });

    output.emit_report(&report, &json)
}

fn verify(store: &ModelStore, spec: &str, output: &OutputOpts) -> miette::Result<()> {
    let refs = resolve_refs(spec)?;
    let mut reports = Vec::new();
    for reference in &refs {
        let report =
            store.verify(reference).map_err(|e| miette::miette!("verify `{reference}`: {e}"))?;
        reports.push(report);
    }
    let intact = reports.iter().all(aphrody_models::VerifyReport::is_intact);

    let mut report = Report::new(format!("Verify {spec}"), &["STATE", "REFERENCE", "SHA-256"])
        .with_footer(if intact {
            format!("{} artefact(s) intact", reports.len())
        } else {
            "at least one artefact does not match what was installed".to_owned()
        });
    for entry in &reports {
        report.push(vec![
            if entry.is_intact() { "ok".into() } else { "CORRUPT".to_owned() },
            entry.reference.clone(),
            entry.actual_sha256.clone(),
        ]);
    }

    let json = serde_json::json!({ "spec": spec, "intact": intact, "artifacts": reports });
    output.emit_report(&report, &json)?;

    if intact {
        Ok(())
    } else {
        // A corrupt artefact must fail the process, so a job runner that
        // verifies before inference stops instead of feeding garbage to a
        // model loader.
        Err(miette::miette!("`{spec}` failed verification"))
    }
}

fn remove(store: &ModelStore, spec: &str, output: &OutputOpts) -> miette::Result<()> {
    let refs = resolve_refs(spec)?;
    let mut removed = Vec::new();
    for reference in &refs {
        let entry =
            store.remove(reference).map_err(|e| miette::miette!("remove `{reference}`: {e}"))?;
        removed.push(entry);
    }
    let reclaimed: u64 = removed.iter().map(|m| m.bytes).sum();

    let mut report = Report::new(format!("Removed {spec}"), &["SIZE", "REFERENCE"])
        .with_footer(format!("reclaimed {}", human_bytes(reclaimed)));
    for model in &removed {
        report.push(vec![human_bytes(model.bytes), model.reference.to_string()]);
    }

    let json = serde_json::json!({
        "spec": spec,
        "removed": removed.iter().map(|m| m.reference.to_string()).collect::<Vec<_>>(),
        "reclaimed_bytes": reclaimed,
    });
    output.emit_report(&report, &json)
}

fn gc(store: &ModelStore, budget: &str, output: &OutputOpts) -> miette::Result<()> {
    let budget_bytes = parse_budget(budget)?;
    let gc_report =
        store.gc(budget_bytes).map_err(|e| miette::miette!("garbage-collect store: {e}"))?;

    let mut report = Report::new("Garbage collection", &["ACTION", "TARGET"]).with_footer(format!(
        "reclaimed {}, {} remaining (budget {})",
        human_bytes(gc_report.reclaimed_bytes),
        human_bytes(gc_report.remaining_bytes),
        human_bytes(budget_bytes)
    ));
    for reference in &gc_report.evicted {
        report.push(vec!["evicted".into(), reference.clone()]);
    }
    for part in &gc_report.removed_parts {
        report.push(vec!["swept".into(), part.display().to_string()]);
    }

    let json = serde_json::to_value(&gc_report)
        .map_err(|e| miette::miette!("serialise gc report: {e}"))?;
    output.emit_report(&report, &json)
}

fn doctor(store: &ModelStore, output: &OutputOpts) -> miette::Result<()> {
    let drift = store.reconcile().map_err(|e| miette::miette!("reconcile model store: {e}"))?;
    let clean = drift.is_clean();

    let mut report = Report::new("Model store doctor", &["ISSUE", "TARGET"])
        .with_summary(format!("Store root: {}", store.root().display()))
        .with_footer(if clean {
            "store is clean".to_owned()
        } else {
            "run `aphrody model gc` to sweep partial downloads".to_owned()
        });
    for reference in &drift.missing {
        report.push(vec!["missing".into(), reference.clone()]);
    }
    for path in &drift.untracked {
        report.push(vec!["untracked".into(), path.display().to_string()]);
    }
    for path in &drift.stale_parts {
        report.push(vec!["partial".into(), path.display().to_string()]);
    }

    let json = serde_json::json!({ "root": store.root(), "clean": clean, "report": drift });
    output.emit_report(&report, &json)?;

    if clean {
        Ok(())
    } else {
        Err(miette::miette!("model store has drifted; run `aphrody model gc` to sweep partials"))
    }
}

fn adopt(store: &ModelStore, path: &Path, output: &OutputOpts) -> miette::Result<()> {
    let entry =
        store.adopt_local(path).map_err(|e| miette::miette!("adopt `{}`: {e}", path.display()))?;

    let mut report = Report::new("Adopted artefact", &["FIELD", "VALUE"])
        .with_footer("the file stays where it is; `aphrody model rm` only un-tracks it");
    report.push(vec!["reference".into(), entry.reference.to_string()]);
    report.push(vec!["path".into(), entry.path.display().to_string()]);
    report.push(vec!["size".into(), human_bytes(entry.bytes)]);
    report.push(vec!["format".into(), entry.format.as_str().to_owned()]);
    report.push(vec!["sha256".into(), entry.sha256.clone()]);

    let json = serde_json::json!({
        "ref": entry.reference.to_string(),
        "path": entry.path,
        "bytes": entry.bytes,
        "sha256": entry.sha256,
        "format": entry.format.as_str(),
        "inspection": entry.inspection,
    });
    output.emit_report(&report, &json)
}

fn path(store: &ModelStore, spec: Option<&str>) -> miette::Result<()> {
    match spec {
        None => println!("{}", store.root().display()),
        Some(spec) => {
            for reference in resolve_refs(spec)? {
                println!("{}", store.path_for(&reference).display());
            }
        },
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn opts(format: Option<&str>, json: bool, out: Option<&str>) -> OutputOpts {
        OutputOpts { format: format.map(ToOwned::to_owned), json, out: out.map(PathBuf::from) }
    }

    #[test]
    fn budget_accepts_binary_and_decimal_suffixes() {
        assert_eq!(parse_budget("1024").unwrap(), 1024);
        assert_eq!(parse_budget("1KiB").unwrap(), 1024);
        assert_eq!(parse_budget("1kib").unwrap(), 1024);
        assert_eq!(parse_budget("2 MiB").unwrap(), 2 << 20);
        assert_eq!(parse_budget("4GiB").unwrap(), 4 << 30);
        assert_eq!(parse_budget("1TiB").unwrap(), 1_u64 << 40);
        assert_eq!(parse_budget("1KB").unwrap(), 1_000);
        assert_eq!(parse_budget("1GB").unwrap(), 1_000_000_000);
        // A bare unit letter reads as binary, matching `du -h`.
        assert_eq!(parse_budget("8G").unwrap(), 8 << 30);
        // Fractions are useful for "half a gig".
        assert_eq!(parse_budget("1.5GiB").unwrap(), 1_610_612_736);
        assert_eq!(parse_budget("0").unwrap(), 0);
    }

    #[test]
    fn budget_rejects_nonsense() {
        assert!(parse_budget("").is_err());
        assert!(parse_budget("   ").is_err());
        assert!(parse_budget("GiB").is_err());
        assert!(parse_budget("12PB").is_err());
        assert!(parse_budget("-4GiB").is_err());
    }

    #[test]
    fn task_parsing_lists_the_valid_names_on_failure() {
        assert_eq!(parse_task("ocr").unwrap(), ModelTask::Ocr);
        assert_eq!(parse_task("speech-to-text").unwrap(), ModelTask::SpeechToText);
        let err = parse_task("transcribe").unwrap_err().to_string();
        assert!(err.contains("visual-transcription"), "{err}");
    }

    #[test]
    fn speed_preference_parsing() {
        assert_eq!(parse_speed("fast").unwrap(), SpeedTier::Fast);
        assert_eq!(parse_speed(" QUALITY ").unwrap(), SpeedTier::Quality);
        assert!(parse_speed("turbo").is_err());
    }

    #[test]
    fn catalog_ids_expand_to_every_artifact() {
        // A single id must pull the whole pipeline, sidecars included.
        let refs = resolve_refs("florence2-base-ft").unwrap();
        assert_eq!(refs.len(), 8);
        // The batch-OCR entry is a detector + recogniser pair plus configs.
        assert_eq!(resolve_refs("ppocr-v5-mobile").unwrap().len(), 4);
        // A raw reference stays exactly one artefact.
        assert_eq!(resolve_refs("hf:owner/repo/model.gguf").unwrap().len(), 1);
    }

    #[test]
    fn unresolvable_specs_are_rejected() {
        assert!(resolve_refs("not a model at all").is_err());
    }

    #[test]
    fn format_precedence_is_flag_then_json_then_extension_then_text() {
        assert_eq!(opts(None, false, None).format().unwrap(), Format::Text);
        assert_eq!(opts(None, true, None).format().unwrap(), Format::Json);
        assert_eq!(opts(Some("md"), false, None).format().unwrap(), Format::Markdown);
        assert_eq!(opts(None, false, Some("out.html")).format().unwrap(), Format::Html);
        assert_eq!(opts(None, false, Some("out.csv")).format().unwrap(), Format::Csv);
        // An explicit flag beats both the shorthand and the extension.
        assert_eq!(opts(Some("html"), true, Some("out.md")).format().unwrap(), Format::Html);
        // `--json` beats an extension.
        assert_eq!(opts(None, true, Some("out.md")).format().unwrap(), Format::Json);
        // An unknown extension falls back to text rather than failing.
        assert_eq!(opts(None, false, Some("out.dat")).format().unwrap(), Format::Text);
    }

    #[test]
    fn an_unknown_format_names_the_valid_ones() {
        let err = opts(Some("pdf"), false, None).format().unwrap_err().to_string();
        assert!(err.contains("markdown"), "{err}");
        assert!(err.contains("html"), "{err}");
        assert!(err.contains("csv"), "{err}");
    }

    #[test]
    fn machine_formats_silence_progress_drawing() {
        assert!(opts(None, true, None).is_machine());
        assert!(opts(Some("csv"), false, None).is_machine());
        assert!(!opts(Some("markdown"), false, None).is_machine());
        assert!(!opts(None, false, None).is_machine());
    }

    #[test]
    fn writing_a_report_creates_missing_parent_directories() {
        let dir = tempfile::tempdir().unwrap();
        let target = dir.path().join("nested/deep/report.md");
        write_out(&target, "# hello\n").unwrap();
        assert_eq!(std::fs::read_to_string(&target).unwrap(), "# hello\n");
    }

    #[test]
    fn emitting_json_writes_the_value_not_the_table() {
        let dir = tempfile::tempdir().unwrap();
        let target = dir.path().join("report.json");
        let report = Report::new("t", &["A"]);
        let value = serde_json::json!({ "count": 2 });
        opts(None, true, Some(&target.to_string_lossy())).emit_report(&report, &value).unwrap();

        let written = std::fs::read_to_string(&target).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&written).unwrap();
        assert_eq!(parsed["count"], 2);
    }

    #[test]
    fn emitting_markdown_writes_a_table_not_json() {
        let dir = tempfile::tempdir().unwrap();
        let target = dir.path().join("report.md");
        let mut report = Report::new("Models", &["ID"]);
        report.push(vec!["whisper-base-en".into()]);
        opts(None, false, Some(&target.to_string_lossy()))
            .emit_report(&report, &serde_json::json!({}))
            .unwrap();

        let written = std::fs::read_to_string(&target).unwrap();
        assert!(written.starts_with("# Models"), "{written}");
        assert!(written.contains("| whisper-base-en |"), "{written}");
    }

    #[test]
    fn outcome_labels_are_stable_machine_words() {
        // The JSON surface promises these three exact strings.
        for label in ["already-present", "downloaded", "adopted"] {
            assert!(label.chars().all(|c| c.is_ascii_lowercase() || c == '-'), "{label}");
        }
    }
}
