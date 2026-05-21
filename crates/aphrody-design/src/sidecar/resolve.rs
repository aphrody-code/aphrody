// SPDX-License-Identifier: Apache-2.0
//! Cross-platform path resolution for sidecar runtime files.
//!
//! Mirrors the upstream sidecar's descriptor-driven contract
//! (`packages/sidecar/src/index.ts`): the host process provides a
//! [`SidecarContract`] with default tmp-dir / namespace / IPC-base values,
//! plus environment-variable names. The functions here resolve runtime
//! paths the same way the upstream Node module does, so a Rust sidecar
//! can drop into the same socket layout transparently.

use std::path::{Path, PathBuf};

use crate::sidecar::error::{SidecarError, SidecarResult};

/// Static defaults for a product's sidecar contract.
#[derive(Debug, Clone)]
pub struct SidecarDefaults {
    /// Bind host for TCP fallbacks (typically `127.0.0.1`).
    pub host: &'static str,
    /// On POSIX, the directory under which app sockets live.
    pub ipc_base: &'static str,
    /// Default namespace when neither caller-supplied nor env-supplied.
    pub namespace: &'static str,
    /// Directory name under the project root used to host the per-source
    /// runtime tree.
    pub project_tmp_dir_name: &'static str,
    /// Prefix injected into the Windows named-pipe name.
    pub windows_pipe_prefix: &'static str,
}

/// Environment-variable names the contract recognizes.
#[derive(Debug, Clone)]
pub struct SidecarEnvKeys {
    pub base: &'static str,
    pub ipc_base: &'static str,
    pub ipc_path: &'static str,
    pub namespace: &'static str,
    pub source: &'static str,
}

/// Full contract describing one host product's sidecar wiring.
#[derive(Debug, Clone)]
pub struct SidecarContract {
    pub defaults: SidecarDefaults,
    pub env: SidecarEnvKeys,
}

/// Resolve the project-tmp root: `<project_root>/<project_tmp_dir_name>`.
///
/// # Errors
///
/// [`SidecarError::Validation`] if `project_root` is empty.
pub fn resolve_project_tmp_root(contract: &SidecarContract, project_root: &Path) -> SidecarResult<PathBuf> {
    if project_root.as_os_str().is_empty() {
        return Err(SidecarError::Validation("project_root must not be empty".into()));
    }
    Ok(project_root.join(contract.defaults.project_tmp_dir_name))
}

/// Resolve the per-source runtime root under the project tmp dir.
///
/// # Errors
///
/// Propagates [`resolve_project_tmp_root`].
pub fn resolve_source_runtime_root(
    contract: &SidecarContract,
    project_root: &Path,
    source: &str,
) -> SidecarResult<PathBuf> {
    let base = resolve_project_tmp_root(contract, project_root)?;
    Ok(base.join(source))
}

/// Resolve the namespace root: `<base>/<namespace>`.
pub fn resolve_namespace_root(base: &Path, namespace: &str) -> PathBuf {
    base.join(namespace)
}

/// Resolve the app IPC path. On Windows: `\\.\pipe\<prefix>-<ns>-<app>`,
/// on POSIX: `<ipc_base>/<ns>/<app>.sock`.
pub fn resolve_app_ipc_path(
    contract: &SidecarContract,
    namespace: &str,
    app: &str,
) -> PathBuf {
    #[cfg(windows)]
    {
        let _ = contract.defaults.ipc_base;
        let pipe = format!(
            r"\\.\pipe\{}-{}-{}",
            contract.defaults.windows_pipe_prefix, namespace, app
        );
        PathBuf::from(pipe)
    }
    #[cfg(not(windows))]
    {
        let _ = contract.defaults.windows_pipe_prefix;
        PathBuf::from(contract.defaults.ipc_base)
            .join(namespace)
            .join(format!("{app}.sock"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fake_contract() -> SidecarContract {
        SidecarContract {
            defaults: SidecarDefaults {
                host: "127.0.0.1",
                ipc_base: "/tmp/fake/ipc",
                namespace: "default",
                project_tmp_dir_name: ".fake-tmp",
                windows_pipe_prefix: "fake",
            },
            env: SidecarEnvKeys {
                base: "FAKE_BASE",
                ipc_base: "FAKE_IPC_BASE",
                ipc_path: "FAKE_IPC_PATH",
                namespace: "FAKE_NAMESPACE",
                source: "FAKE_SOURCE",
            },
        }
    }

    #[test]
    fn project_tmp_root_appends_default_dir_name() {
        let contract = fake_contract();
        let p = resolve_project_tmp_root(&contract, Path::new("/repo/product")).unwrap();
        assert_eq!(p, PathBuf::from("/repo/product").join(".fake-tmp"));
    }

    #[test]
    fn ipc_path_is_platform_appropriate() {
        let contract = fake_contract();
        let p = resolve_app_ipc_path(&contract, "alpha", "ui");
        let s = p.to_string_lossy();
        #[cfg(windows)]
        assert_eq!(s, r"\\.\pipe\fake-alpha-ui");
        #[cfg(not(windows))]
        assert_eq!(s, "/tmp/fake/ipc/alpha/ui.sock");
    }
}
