// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 aphrody contributors
//
// ONNX Runtime session loading with execution-provider fallback.
//
// The contract this module enforces: a caller asks for the best available
// accelerator and ALWAYS gets a working session, but is told which provider it
// actually got. Silent CPU fallback is the classic way a "GPU pipeline" ends
// up ten times slower than expected with nobody noticing, so the provider that
// won is part of the returned value, not a log line.
//
// Provider order is derived from the hardware probe in `aphrody-models`, not
// hard-coded: on this machine that yields CUDA -> DirectML -> CPU, on a Linux
// box without a GPU it collapses to CPU alone.
//
// This module is compiled only with the `onnx` feature.

use std::path::{Path, PathBuf};

use aphrody_models::{Accelerator, HardwareProfile};
use ort::session::Session;
use ort::session::builder::GraphOptimizationLevel;

use crate::error::{InferError, Result};
use crate::runtime::{self, RuntimeSource};

/// How to build a session.
#[derive(Debug, Clone)]
pub struct SessionConfig {
    /// Providers to try, in order. The first that builds a session wins.
    pub providers: Vec<Accelerator>,
    /// Intra-op thread count. `None` lets ONNX Runtime choose.
    pub intra_threads: Option<usize>,
    /// Graph optimisation level. `Level3` is the default and is what a batch
    /// job wants: the extra load-time cost is amortised over every image.
    pub optimize: bool,
}

impl Default for SessionConfig {
    fn default() -> Self {
        Self { providers: vec![Accelerator::Cpu], intra_threads: None, optimize: true }
    }
}

impl SessionConfig {
    /// Build a configuration from a probed hardware profile.
    ///
    /// The profile already ranks accelerators best-first, and CPU is appended
    /// as the floor so a session can always be built.
    #[must_use]
    pub fn from_profile(profile: &HardwareProfile) -> Self {
        let mut providers = profile.accelerators.clone();
        if !providers.contains(&Accelerator::Cpu) {
            providers.push(Accelerator::Cpu);
        }
        Self {
            providers,
            // Leave intra-op threads to ONNX Runtime: it reads the actual core
            // topology, and over-subscribing hurts a batch pipeline more than
            // under-subscribing.
            intra_threads: None,
            optimize: true,
        }
    }

    /// Force a single provider, skipping the fallback chain.
    #[must_use]
    pub fn with_only(provider: Accelerator) -> Self {
        Self { providers: vec![provider], ..Self::default() }
    }
}

/// A loaded model, plus what it actually runs on.
pub struct LoadedModel {
    /// The ONNX Runtime session.
    pub session: Session,
    /// The provider that successfully built this session.
    pub provider: Accelerator,
    /// Providers that were tried and rejected, with the reason. Empty when the
    /// first choice worked.
    pub fallbacks: Vec<(Accelerator, String)>,
    /// The model file this session was built from.
    pub path: PathBuf,
}

impl LoadedModel {
    /// Input names and their types, as ONNX Runtime reports them.
    #[must_use]
    pub fn inputs(&self) -> Vec<(String, String)> {
        self.session
            .inputs()
            .iter()
            .map(|outlet| (outlet.name().to_owned(), format!("{:?}", outlet.dtype())))
            .collect()
    }

    /// Output names and their types.
    #[must_use]
    pub fn outputs(&self) -> Vec<(String, String)> {
        self.session
            .outputs()
            .iter()
            .map(|outlet| (outlet.name().to_owned(), format!("{:?}", outlet.dtype())))
            .collect()
    }

    /// Whether the session ended up on something other than the CPU.
    #[must_use]
    pub fn is_accelerated(&self) -> bool {
        self.provider != Accelerator::Cpu
    }
}

impl core::fmt::Debug for LoadedModel {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("LoadedModel")
            .field("provider", &self.provider)
            .field("fallbacks", &self.fallbacks)
            .field("path", &self.path)
            .finish_non_exhaustive()
    }
}

/// Initialise ONNX Runtime once for the process, loading the discovered
/// shared library.
///
/// Calling this more than once is harmless: the second call sees the runtime
/// already loaded and returns the same source.
///
/// # Errors
///
/// [`InferError::RuntimeLoad`] when the library cannot be loaded.
pub fn init_runtime() -> Result<RuntimeSource> {
    use std::sync::OnceLock;
    static INIT: OnceLock<core::result::Result<RuntimeSource, String>> = OnceLock::new();

    INIT.get_or_init(|| {
        let source = runtime::discover();
        let outcome = if let Some(path) = source.path() {
            ort::init_from(path).map(|builder| {
                builder.commit();
            })
        } else {
            // No explicit path: let `ort` use the platform loader.
            ort::init().commit();
            Ok(())
        };
        match outcome {
            Ok(()) => Ok(source),
            Err(e) => Err(e.to_string()),
        }
    })
    .clone()
    .map_err(|reason| InferError::RuntimeLoad { path: runtime::discover().path().map(Path::to_path_buf), reason })
}

/// Build a session for `model_path`, walking the provider chain.
///
/// # Errors
///
/// [`InferError::SessionBuild`] when every provider in the chain failed. The
/// message carries each provider's own failure, because "it fell back to CPU"
/// and "it could not load at all" need different fixes.
pub fn load(model_path: &Path, config: &SessionConfig) -> Result<LoadedModel> {
    init_runtime()?;

    let mut fallbacks: Vec<(Accelerator, String)> = Vec::new();
    for provider in &config.providers {
        match try_build(model_path, *provider, config) {
            Ok(session) => {
                return Ok(LoadedModel {
                    session,
                    provider: *provider,
                    fallbacks,
                    path: model_path.to_path_buf(),
                });
            }
            Err(reason) => {
                tracing::debug!(provider = provider.as_str(), %reason, "execution provider rejected");
                fallbacks.push((*provider, reason));
            }
        }
    }

    let detail = fallbacks
        .iter()
        .map(|(provider, reason)| format!("{provider}: {reason}"))
        .collect::<Vec<_>>()
        .join("; ");
    Err(InferError::SessionBuild {
        path: model_path.to_path_buf(),
        reason: if detail.is_empty() {
            "no execution provider was configured".to_owned()
        } else {
            detail
        },
    })
}

/// Attempt one provider. Returns the provider's own error message on failure.
fn try_build(
    model_path: &Path,
    provider: Accelerator,
    config: &SessionConfig,
) -> core::result::Result<Session, String> {
    let mut builder = Session::builder().map_err(|e| e.to_string())?;

    // Registering an EP that ONNX Runtime was not built with is a hard error
    // here rather than a warning, which is exactly what makes the fallback
    // chain observable instead of silent.
    let dispatch = match provider {
        Accelerator::Cuda => vec![ort::ep::CUDA::default().build()],
        Accelerator::DirectMl => vec![ort::ep::DirectML::default().build()],
        Accelerator::Cpu => vec![ort::ep::CPU::default().build()],
        // No ONNX Runtime EP maps onto these on the platforms aphrody targets.
        // `Accelerator` is `#[non_exhaustive]`, so a wildcard also keeps this
        // compiling when a new variant lands upstream — it just falls to the
        // next provider in the chain instead of failing the build.
        _ => {
            return Err(format!("no ONNX Runtime execution provider for {provider}"));
        }
    };
    builder = builder.with_execution_providers(dispatch).map_err(|e| e.to_string())?;

    if let Some(threads) = config.intra_threads {
        builder = builder.with_intra_threads(threads).map_err(|e| e.to_string())?;
    }
    if config.optimize {
        builder = builder
            .with_optimization_level(GraphOptimizationLevel::Level3)
            .map_err(|e| e.to_string())?;
    }

    builder.commit_from_file(model_path).map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn profile(accelerators: Vec<Accelerator>) -> HardwareProfile {
        HardwareProfile {
            accelerators,
            gpus: Vec::new(),
            cuda_toolkit: None,
            cpu_threads: 8,
        }
    }

    #[test]
    fn config_from_profile_keeps_the_probe_order() {
        let config = SessionConfig::from_profile(&profile(vec![
            Accelerator::Cuda,
            Accelerator::DirectMl,
            Accelerator::Cpu,
        ]));
        assert_eq!(config.providers, vec![Accelerator::Cuda, Accelerator::DirectMl, Accelerator::Cpu]);
    }

    #[test]
    fn cpu_is_always_the_last_resort() {
        // A profile that somehow omits CPU must still yield a buildable chain.
        let config = SessionConfig::from_profile(&profile(vec![Accelerator::Cuda]));
        assert_eq!(config.providers.last(), Some(&Accelerator::Cpu));
    }

    #[test]
    fn forcing_one_provider_disables_the_chain() {
        let config = SessionConfig::with_only(Accelerator::Cuda);
        assert_eq!(config.providers, vec![Accelerator::Cuda]);
    }

    #[test]
    fn loading_a_missing_file_names_the_file_not_the_provider() {
        let missing = Path::new("does-not-exist-anywhere.onnx");
        let err = load(missing, &SessionConfig::default()).unwrap_err();
        let rendered = err.to_string();
        assert!(rendered.contains("does-not-exist-anywhere.onnx"), "{rendered}");
    }

    #[test]
    fn an_empty_provider_chain_is_reported_plainly() {
        let config = SessionConfig { providers: Vec::new(), ..SessionConfig::default() };
        let err = load(Path::new("whatever.onnx"), &config).unwrap_err();
        assert!(err.to_string().contains("no execution provider was configured"), "{err}");
    }
}
