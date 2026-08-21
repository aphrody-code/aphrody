// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 aphrody contributors
//
// Locating the ONNX Runtime shared library.
//
// aphrody deliberately does NOT let `ort` download its own runtime
// (`download-binaries` is off in the workspace manifest): that feature fetches
// a CPU-only build over native-tls, and this project is rustls-only and wants
// the CUDA build. Instead the runtime is installed once under
// `~/.aphrody/runtimes/` and discovered here.
//
// Why `load-dynamic` at all: aphrody forces `+crt-static` on MSVC while every
// prebuilt ONNX Runtime is `/MD`. Statically linking the two is irreconcilable
// (CLAUDE.md §7), so the library is loaded at runtime through `libloading`,
// which also makes the CUDA build swappable without a rebuild.
//
// Resolution order:
//   1. $APHRODY_ORT_DYLIB        — explicit override, used verbatim
//   2. $ORT_DYLIB_PATH           — the variable `ort` itself documents
//   3. ~/.aphrody/runtimes/onnxruntime-*/lib/<dylib>  — newest match wins
//   4. the loader's own search path (system install)

use std::path::{Path, PathBuf};

use crate::error::{InferError, Result};

/// Platform file name of the ONNX Runtime shared library.
pub const DYLIB_NAME: &str = if cfg!(windows) {
    "onnxruntime.dll"
} else if cfg!(target_vendor = "apple") {
    "libonnxruntime.dylib"
} else {
    "libonnxruntime.so"
};

/// Where a runtime was found, so a report can explain the choice.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "kebab-case", tag = "source", content = "path")]
pub enum RuntimeSource {
    /// `$APHRODY_ORT_DYLIB`.
    Override(PathBuf),
    /// `$ORT_DYLIB_PATH`.
    OrtEnv(PathBuf),
    /// Installed under `~/.aphrody/runtimes`.
    Managed(PathBuf),
    /// Left to the platform loader; no explicit path.
    SystemSearchPath,
}

impl RuntimeSource {
    /// The resolved path, when the choice names one.
    #[must_use]
    pub fn path(&self) -> Option<&Path> {
        match self {
            Self::Override(p) | Self::OrtEnv(p) | Self::Managed(p) => Some(p),
            Self::SystemSearchPath => None,
        }
    }

    /// Short label for reports.
    #[must_use]
    pub const fn label(&self) -> &'static str {
        match self {
            Self::Override(_) => "APHRODY_ORT_DYLIB",
            Self::OrtEnv(_) => "ORT_DYLIB_PATH",
            Self::Managed(_) => "managed (~/.aphrody/runtimes)",
            Self::SystemSearchPath => "system search path",
        }
    }
}

/// Root of the managed runtime directory: `~/.aphrody/runtimes`.
///
/// # Errors
///
/// [`InferError::NoStateDir`] when no home directory can be resolved.
pub fn runtimes_dir() -> Result<PathBuf> {
    if let Ok(explicit) = std::env::var("APHRODY_RUNTIMES_DIR") {
        if !explicit.is_empty() {
            return Ok(PathBuf::from(explicit));
        }
    }
    let state = if let Ok(home) = std::env::var("APHRODY_HOME").filter_non_empty() {
        PathBuf::from(home)
    } else if cfg!(windows) && let Ok(profile) = std::env::var("USERPROFILE").filter_non_empty() {
        PathBuf::from(profile).join(".aphrody")
    } else if let Ok(home) = std::env::var("HOME").filter_non_empty() {
        PathBuf::from(home).join(".aphrody")
    } else {
        dirs::home_dir().ok_or(InferError::NoStateDir)?.join(".aphrody")
    };
    Ok(state.join("runtimes"))
}

/// Small helper so the resolution chain above reads as one expression.
trait NonEmpty {
    fn filter_non_empty(self) -> Self;
}

impl NonEmpty for core::result::Result<String, std::env::VarError> {
    fn filter_non_empty(self) -> Self {
        match self {
            Ok(value) if value.is_empty() => Err(std::env::VarError::NotPresent),
            other => other,
        }
    }
}

/// Locate the ONNX Runtime shared library.
///
/// Never fails: falling through to [`RuntimeSource::SystemSearchPath`] is a
/// legitimate answer on a machine with a system-wide install, and letting the
/// loader try is more useful than refusing up front.
#[must_use]
pub fn discover() -> RuntimeSource {
    for (var, wrap) in [
        ("APHRODY_ORT_DYLIB", RuntimeSource::Override as fn(PathBuf) -> RuntimeSource),
        ("ORT_DYLIB_PATH", RuntimeSource::OrtEnv as fn(PathBuf) -> RuntimeSource),
    ] {
        if let Ok(raw) = std::env::var(var) {
            if !raw.is_empty() {
                return wrap(PathBuf::from(raw));
            }
        }
    }

    if let Some(found) = find_managed() {
        return RuntimeSource::Managed(found);
    }

    RuntimeSource::SystemSearchPath
}

/// Scan `~/.aphrody/runtimes/*/lib/<dylib>` and pick the best candidate.
///
/// Directory names carry the flavour and version
/// (`onnxruntime-win-x64-gpu_cuda13-1.29.0`), and a GPU build is preferred
/// over a CPU one because a CPU-only library silently makes every CUDA request
/// fall back — the exact failure this whole module exists to avoid. Ties break
/// on the directory name in reverse order, which puts the highest version
/// first for the `name-X.Y.Z` convention upstream uses.
fn find_managed() -> Option<PathBuf> {
    find_managed_in(&runtimes_dir().ok()?)
}

/// The scan itself, against an explicit root.
///
/// Split out from [`find_managed`] so it is testable without writing to the
/// process environment — `set_var` is `unsafe` since edition 2024, and this
/// crate forbids `unsafe`.
#[must_use]
pub fn find_managed_in(root: &Path) -> Option<PathBuf> {
    let mut candidates: Vec<PathBuf> = std::fs::read_dir(root)
        .ok()?
        .filter_map(core::result::Result::ok)
        .map(|entry| entry.path())
        .filter(|path| path.is_dir())
        .collect();

    candidates.sort_by_key(|path| {
        let name = path.file_name().unwrap_or_default().to_string_lossy().into_owned();
        // `false` sorts first, so negate the GPU test to rank GPU builds ahead.
        (!is_gpu_build(&name), core::cmp::Reverse(name))
    });

    candidates.into_iter().find_map(|dir| {
        let lib = dir.join("lib").join(DYLIB_NAME);
        if lib.is_file() {
            return Some(lib);
        }
        // Some distributions drop the library at the top level.
        let flat = dir.join(DYLIB_NAME);
        flat.is_file().then_some(flat)
    })
}

/// Whether a runtime directory name denotes a GPU-capable build.
#[must_use]
pub fn is_gpu_build(dir_name: &str) -> bool {
    let lower = dir_name.to_ascii_lowercase();
    lower.contains("gpu") || lower.contains("cuda") || lower.contains("tensorrt")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dylib_name_matches_the_platform() {
        if cfg!(windows) {
            assert_eq!(DYLIB_NAME, "onnxruntime.dll");
        } else if cfg!(target_vendor = "apple") {
            assert_eq!(DYLIB_NAME, "libonnxruntime.dylib");
        } else {
            assert_eq!(DYLIB_NAME, "libonnxruntime.so");
        }
    }

    #[test]
    fn gpu_builds_are_recognised_by_directory_name() {
        assert!(is_gpu_build("onnxruntime-win-x64-gpu_cuda13-1.29.0"));
        assert!(is_gpu_build("onnxruntime-linux-x64-cuda-1.22.0"));
        assert!(is_gpu_build("onnxruntime-win-x64-TensorRT-1.20.0"));
        assert!(!is_gpu_build("onnxruntime-win-x64-1.29.0"));
        assert!(!is_gpu_build("onnxruntime-linux-aarch64-1.22.0"));
    }

    #[test]
    fn source_labels_and_paths_line_up() {
        let managed = RuntimeSource::Managed(PathBuf::from("/opt/ort/lib/libonnxruntime.so"));
        assert_eq!(managed.path(), Some(Path::new("/opt/ort/lib/libonnxruntime.so")));
        assert!(managed.label().contains("managed"));
        assert_eq!(RuntimeSource::SystemSearchPath.path(), None);
        assert_eq!(RuntimeSource::SystemSearchPath.label(), "system search path");
    }

    #[test]
    fn discovery_never_panics_and_always_answers() {
        // Whatever this machine holds, discovery must produce a usable answer
        // rather than an error: the loader gets the last word.
        let source = discover();
        if let Some(path) = source.path() {
            assert!(!path.as_os_str().is_empty());
        }
    }

    #[test]
    fn managed_scan_prefers_a_gpu_build_over_a_cpu_one() {
        let dir = tempfile::tempdir().unwrap();
        for name in ["onnxruntime-win-x64-1.29.0", "onnxruntime-win-x64-gpu_cuda13-1.29.0"] {
            let lib = dir.path().join(name).join("lib");
            std::fs::create_dir_all(&lib).unwrap();
            std::fs::write(lib.join(DYLIB_NAME), b"stub").unwrap();
        }

        let found = find_managed_in(dir.path()).expect("a runtime should be found");
        assert!(
            is_gpu_build(&found.to_string_lossy()),
            "picked the CPU build over the GPU one: {}",
            found.display()
        );
    }

    #[test]
    fn managed_scan_picks_the_highest_version_among_equals() {
        let dir = tempfile::tempdir().unwrap();
        for name in [
            "onnxruntime-win-x64-gpu_cuda13-1.27.1",
            "onnxruntime-win-x64-gpu_cuda13-1.29.0",
        ] {
            let lib = dir.path().join(name).join("lib");
            std::fs::create_dir_all(&lib).unwrap();
            std::fs::write(lib.join(DYLIB_NAME), b"stub").unwrap();
        }

        let found = find_managed_in(dir.path()).unwrap();
        assert!(found.to_string_lossy().contains("1.29.0"), "{}", found.display());
    }

    #[test]
    fn a_library_at_the_top_level_is_accepted_too() {
        let dir = tempfile::tempdir().unwrap();
        let flat = dir.path().join("onnxruntime-flat-1.29.0");
        std::fs::create_dir_all(&flat).unwrap();
        std::fs::write(flat.join(DYLIB_NAME), b"stub").unwrap();
        assert_eq!(find_managed_in(dir.path()), Some(flat.join(DYLIB_NAME)));
    }

    #[test]
    fn managed_scan_returns_none_when_nothing_is_installed() {
        let dir = tempfile::tempdir().unwrap();
        assert!(find_managed_in(dir.path()).is_none());
        // A directory that does not exist is the same answer, not a panic.
        assert!(find_managed_in(&dir.path().join("absent")).is_none());
    }
}
