// SPDX-License-Identifier: Apache-2.0
//! `aphrody infer …` — local ONNX inference backend: runtime, providers,
//! session probing.
//!
//! Built only with `--features infer`, which links ONNX Runtime through
//! `aphrody-infer`. The subcommands here answer the questions a pipeline needs
//! settled before it starts a batch: is a runtime installed, which execution
//! provider will a model actually get, and does the graph load at all.

use std::path::PathBuf;

use aphrody_infer::{RuntimeSource, SessionConfig, runtime};
use aphrody_models::{Accelerator, Catalog, Report, accel, human_bytes};

use crate::model_cmd::OutputOpts;

/// Actions for the `infer` subcommand.
#[derive(clap::Subcommand, Debug, Clone)]
pub(crate) enum InferAction {
    /// Report which ONNX Runtime will be loaded and from where.
    ///
    /// Example: aphrody infer runtime --json
    Runtime {
        #[command(flatten)]
        output: OutputOpts,
    },
    /// Report which llama.cpp binaries are available for the GGUF entries in
    /// the catalog (dots.ocr, granite-docling, SmolVLM).
    ///
    /// aphrody drives the upstream llama.cpp release binaries rather than
    /// linking them, so this reports what was found and where.
    ///
    /// Example: aphrody infer llama --json
    Llama {
        #[command(flatten)]
        output: OutputOpts,
    },
    /// Load a model and report the execution provider it actually got, plus
    /// the graph's inputs and outputs.
    ///
    /// This is the check that distinguishes "running on the GPU" from "fell
    /// back to CPU and nobody noticed". Exits non-zero if `--require` names a
    /// provider and the session did not get it.
    ///
    /// Example: aphrody infer probe ppocr-v5-mobile --role detector --require cuda
    Probe {
        /// Catalog id (with `--role`) or a path to an `.onnx` file.
        target: String,
        /// Artefact role inside the catalog entry. Defaults to the entry's
        /// primary artefact.
        #[arg(long)]
        role: Option<String>,
        /// Fail unless the session lands on this provider (`cuda`,
        /// `directml`, `cpu`).
        #[arg(long)]
        require: Option<String>,
        /// Force a single provider instead of the probed fallback chain.
        #[arg(long)]
        provider: Option<String>,
        #[command(flatten)]
        output: OutputOpts,
    },
}

/// Run an `infer` action.
///
/// # Errors
///
/// Returns a `miette` report when the runtime cannot load, the model is not
/// installed, or `--require` is not satisfied.
pub(crate) fn run(action: InferAction) -> miette::Result<()> {
    match action {
        InferAction::Runtime { output } => runtime_report(&output),
        InferAction::Llama { output } => llama_report(&output),
        InferAction::Probe { target, role, require, provider, output } => {
            probe(&target, role.as_deref(), require.as_deref(), provider.as_deref(), &output)
        },
    }
}

fn parse_provider(raw: &str) -> miette::Result<Accelerator> {
    Accelerator::from_str_opt(raw).ok_or_else(|| {
        miette::miette!("unknown provider `{raw}` (expected one of: cuda, directml, cpu)")
    })
}

fn runtime_report(output: &OutputOpts) -> miette::Result<()> {
    let source = runtime::discover();
    let dir = runtime::runtimes_dir().ok();

    // Loading it is the only honest way to answer "will this work".
    let load_result = aphrody_infer::init_runtime();

    let mut report = Report::new("ONNX Runtime", &["FIELD", "VALUE"]).with_footer(
        "install a runtime under ~/.aphrody/runtimes, or point $APHRODY_ORT_DYLIB at one",
    );
    report.push(vec!["discovered via".into(), source.label().to_owned()]);
    report.push(vec![
        "library".into(),
        source.path().map_or_else(|| "(platform loader)".to_owned(), |p| p.display().to_string()),
    ]);
    report.push(vec![
        "runtimes dir".into(),
        dir.as_ref().map_or_else(|| "-".to_owned(), |p| p.display().to_string()),
    ]);
    report.push(vec![
        "gpu build".into(),
        source.path().map_or_else(
            || "unknown".to_owned(),
            |p| runtime::is_gpu_build(&p.to_string_lossy()).to_string(),
        ),
    ]);
    report.push(vec!["loads".into(), match &load_result {
        Ok(_) => "yes".to_owned(),
        Err(e) => format!("no — {e}"),
    }]);

    let json = serde_json::json!({
        "source": source,
        "runtimes_dir": dir,
        "gpu_build": source.path().map(|p| runtime::is_gpu_build(&p.to_string_lossy())),
        "loads": load_result.is_ok(),
        "error": load_result.as_ref().err().map(ToString::to_string),
    });
    output.emit_report(&report, &json)?;

    load_result.map(|_: RuntimeSource| ()).map_err(|e| miette::miette!("{e}"))
}

fn llama_report(output: &OutputOpts) -> miette::Result<()> {
    let found = aphrody_infer::llama::available();
    let gguf_entries: Vec<&str> = Catalog::builtin()
        .by_backend(aphrody_models::Backend::LlamaCpp)
        .into_iter()
        .map(|e| e.id.as_str())
        .collect();

    let mut report = Report::new("llama.cpp backend", &["TOOL", "SOURCE", "PATH"])
        .with_summary(format!(
            "Catalog entries needing this backend: {}",
            if gguf_entries.is_empty() { "none".to_owned() } else { gguf_entries.join(", ") }
        ))
        .with_footer(if found.is_empty() {
            "not installed — unpack a llama.cpp release under ~/.aphrody/runtimes/llama-<build>/"
                .to_owned()
        } else {
            format!("{} tool(s) available", found.len())
        });
    for (tool, source) in &found {
        report.push(vec![
            tool.to_string(),
            source.label().to_owned(),
            source.path().display().to_string(),
        ]);
    }

    let json = serde_json::json!({
        "available": found.iter().map(|(tool, source)| serde_json::json!({
            "tool": tool,
            "source": source,
        })).collect::<Vec<_>>(),
        "gguf_catalog_entries": gguf_entries,
        "installed": !found.is_empty(),
    });
    output.emit_report(&report, &json)
}

fn probe(
    target: &str,
    role: Option<&str>,
    require: Option<&str>,
    provider: Option<&str>,
    output: &OutputOpts,
) -> miette::Result<()> {
    let required = require.map(parse_provider).transpose()?;

    let profile = accel::probe();
    let config = match provider {
        Some(raw) => SessionConfig::with_only(parse_provider(raw)?),
        None => SessionConfig::from_profile(&profile),
    };

    // A catalog id resolves through the store; anything else is a path.
    let (label, model) = if let Ok(entry) = Catalog::builtin().get(target) {
        let role = role
            .map(ToOwned::to_owned)
            .or_else(|| entry.primary().map(|f| f.role.clone()))
            .ok_or_else(|| miette::miette!("catalog entry `{target}` has no artefacts"))?;
        let loaded = aphrody_infer::load_catalog_role(target, &role, &config)
            .map_err(|e| miette::miette!("{e}"))?;
        (format!("{target}/{role}"), loaded)
    } else {
        let path = PathBuf::from(target);
        if !path.is_file() {
            return Err(miette::miette!("`{target}` is neither a catalog id nor an existing file"));
        }
        let loaded = aphrody_infer::load(&path, &config).map_err(|e| miette::miette!("{e}"))?;
        (path.display().to_string(), loaded)
    };

    let size = std::fs::metadata(&model.path).map(|m| m.len()).unwrap_or(0);

    let mut report =
        Report::new(format!("Probe {label}"), &["FIELD", "VALUE"]).with_summary(format!(
            "Provider chain: {}",
            config.providers.iter().map(|p| p.as_str()).collect::<Vec<_>>().join(" -> ")
        ));
    report.push(vec!["provider".into(), model.provider.to_string()]);
    report.push(vec!["accelerated".into(), model.is_accelerated().to_string()]);
    report.push(vec!["model".into(), model.path.display().to_string()]);
    report.push(vec!["size".into(), human_bytes(size)]);
    for (name, dtype) in model.inputs() {
        report.push(vec!["input".into(), format!("{name}: {dtype}")]);
    }
    for (name, dtype) in model.outputs() {
        report.push(vec!["output".into(), format!("{name}: {dtype}")]);
    }
    for (provider, reason) in &model.fallbacks {
        report.push(vec!["rejected".into(), format!("{provider}: {reason}")]);
    }

    let json = serde_json::json!({
        "target": label,
        "provider": model.provider.as_str(),
        "accelerated": model.is_accelerated(),
        "path": model.path,
        "bytes": size,
        "chain": config.providers.iter().map(|p| p.as_str()).collect::<Vec<_>>(),
        "inputs": model.inputs().into_iter().map(|(n, t)| serde_json::json!({"name": n, "type": t})).collect::<Vec<_>>(),
        "outputs": model.outputs().into_iter().map(|(n, t)| serde_json::json!({"name": n, "type": t})).collect::<Vec<_>>(),
        "rejected": model.fallbacks.iter().map(|(p, r)| serde_json::json!({"provider": p.as_str(), "reason": r})).collect::<Vec<_>>(),
    });
    output.emit_report(&report, &json)?;

    if let Some(required) = required {
        if model.provider != required {
            // A batch job that silently ran on CPU is worse than one that
            // refused to start, so this is an error, not a warning.
            return Err(miette::miette!(
                "required provider `{required}` but the session landed on `{}`",
                model.provider
            ));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn provider_names_parse_and_bad_ones_explain_themselves() {
        assert_eq!(parse_provider("cuda").unwrap(), Accelerator::Cuda);
        assert_eq!(parse_provider("cpu").unwrap(), Accelerator::Cpu);
        assert_eq!(parse_provider("directml").unwrap(), Accelerator::DirectMl);
        let err = parse_provider("rocm").unwrap_err().to_string();
        assert!(err.contains("cuda"), "{err}");
    }

    #[test]
    fn a_probed_profile_always_ends_on_cpu() {
        // The probe drives the chain, so whatever this machine is, the last
        // resort must be buildable.
        let config = SessionConfig::from_profile(&accel::probe());
        assert_eq!(config.providers.last(), Some(&Accelerator::Cpu));
    }

    #[test]
    fn the_output_format_surface_is_shared_with_model_cmd() {
        // `infer` reuses `OutputOpts` so `--format markdown` means the same
        // thing on both subcommands.
        assert!(aphrody_models::Format::from_str_opt("markdown").is_some());
    }
}
