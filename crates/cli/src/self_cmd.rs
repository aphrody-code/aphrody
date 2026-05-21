// SPDX-License-Identifier: Apache-2.0
// `aphrody self` — installer / bootstrap subcommands.
//
// Replaces the legacy `scripts/Install-AphrodyToPath.ps1` (Windows) and
// `scripts/install-aphrody-path.sh` (Unix). All logic is native Rust, gated
// per-OS so the same binary handles both platforms without external shells.
//
// Phase 3.1 of docs/PLAN_RUST_ONLY.md.

use std::path::{Path, PathBuf};

use async_trait::async_trait;
use miette::{IntoDiagnostic, WrapErr};

use crate::{
    commands::SubprocessExit,
    context::{GoogleContext, TerminalCommand},
};

/// `aphrody self install-path` — register the release binary on the user's
/// PATH (or symlink it into `$HOME/.local/bin`).
pub(crate) struct InstallPathCommand {
    /// Absolute path to the `aphrody` binary. When `None`, falls back to
    /// `<repo>/target/release/aphrody[.exe]` relative to the current working
    /// directory.
    pub bin: Option<PathBuf>,
    /// If the binary is missing, run `cargo build -p aphrody --release
    /// --locked` first.
    pub build_if_missing: bool,
    /// Plan-only — print the intended actions, never mutate registry/symlinks.
    pub dry_run: bool,
}

#[async_trait]
impl TerminalCommand for InstallPathCommand {
    async fn execute(&self, _ctx: &GoogleContext) -> miette::Result<()> {
        let bin = resolve_bin_path(self.bin.clone())?;

        if !bin.exists() {
            if self.build_if_missing {
                if self.dry_run {
                    println!(
                        "[dry-run] would run: cargo build -p aphrody --release --locked"
                    );
                } else {
                    println!(
                        "binary not found at {} — running cargo build -p aphrody --release \
                         --locked",
                        bin.display()
                    );
                    let status = std::process::Command::new("cargo")
                        .args(["build", "-p", "aphrody", "--release", "--locked"])
                        .status()
                        .into_diagnostic()
                        .wrap_err("failed to spawn `cargo build`")?;
                    if !status.success() {
                        return Err(miette::Report::new(SubprocessExit(
                            status.code().unwrap_or(1),
                        )));
                    }
                }
            } else if !self.dry_run {
                return Err(miette::miette!(
                    "binary not found at {}. Pass --build to compile it first.",
                    bin.display()
                ));
            } else {
                println!(
                    "[dry-run] note: binary not found at {} (pass --build to compile)",
                    bin.display()
                );
            }
        }

        platform_install_path(&bin, self.dry_run)
    }
}

/// Resolve the binary path, defaulting to `<cwd>/target/release/aphrody[.exe]`.
fn resolve_bin_path(explicit: Option<PathBuf>) -> miette::Result<PathBuf> {
    if let Some(p) = explicit {
        return Ok(p);
    }
    let cwd = std::env::current_dir()
        .into_diagnostic()
        .wrap_err("failed to read current working directory")?;
    let exe_name = if cfg!(windows) { "aphrody.exe" } else { "aphrody" };
    Ok(cwd.join("target").join("release").join(exe_name))
}

// ── Windows: HKCU\Environment\Path ──────────────────────────────────────────
#[cfg(target_os = "windows")]
fn platform_install_path(bin: &Path, dry_run: bool) -> miette::Result<()> {
    use std::ffi::OsString;
    use std::os::windows::ffi::OsStringExt;

    use windows_sys::Win32::Foundation::ERROR_SUCCESS;
    use windows_sys::Win32::System::Registry::{
        HKEY, HKEY_CURRENT_USER, KEY_READ, KEY_WRITE, REG_EXPAND_SZ, REG_SZ, RegCloseKey,
        RegOpenKeyExW, RegQueryValueExW, RegSetValueExW,
    };

    let bin_dir = bin
        .parent()
        .ok_or_else(|| miette::miette!("binary path has no parent directory: {}", bin.display()))?
        .to_path_buf();
    let bin_dir_str =
        bin_dir.to_str().ok_or_else(|| miette::miette!("non-UTF-8 binary directory"))?;

    // Open HKCU\Environment for read first to inspect current PATH.
    let subkey: Vec<u16> = "Environment\0".encode_utf16().collect();
    let path_name: Vec<u16> = "Path\0".encode_utf16().collect();

    let (current_path, current_type) = unsafe {
        let mut hkey: HKEY = std::ptr::null_mut();
        let rc = RegOpenKeyExW(
            HKEY_CURRENT_USER,
            subkey.as_ptr(),
            0,
            KEY_READ,
            &mut hkey,
        );
        if rc != ERROR_SUCCESS {
            return Err(miette::miette!(
                "RegOpenKeyExW(HKCU\\Environment) failed with code {rc}"
            ));
        }

        let mut buf_size: u32 = 0;
        let mut value_type: u32 = 0;
        // First call with NULL buffer to learn the size.
        let _ = RegQueryValueExW(
            hkey,
            path_name.as_ptr(),
            std::ptr::null_mut(),
            &mut value_type,
            std::ptr::null_mut(),
            &mut buf_size,
        );

        let (path, type_out) = if buf_size == 0 {
            (String::new(), REG_SZ)
        } else {
            let mut buf = vec![0u16; (buf_size as usize).div_ceil(2)];
            let mut size = buf_size;
            let rc2 = RegQueryValueExW(
                hkey,
                path_name.as_ptr(),
                std::ptr::null_mut(),
                &mut value_type,
                buf.as_mut_ptr() as *mut u8,
                &mut size,
            );
            if rc2 != ERROR_SUCCESS {
                let _ = RegCloseKey(hkey);
                return Err(miette::miette!(
                    "RegQueryValueExW(Path) failed with code {rc2}"
                ));
            }
            // Strip trailing NUL if present.
            let len = buf.iter().position(|&c| c == 0).unwrap_or(buf.len());
            let s = OsString::from_wide(&buf[..len])
                .into_string()
                .map_err(|_| miette::miette!("HKCU PATH is not valid UTF-8"))?;
            (s, value_type)
        };

        let _ = RegCloseKey(hkey);
        (path, type_out)
    };

    let entries: Vec<&str> =
        current_path.split(';').filter(|s| !s.is_empty()).collect();
    let already_present = entries.iter().any(|e| {
        // Case-insensitive comparison on Windows.
        e.to_lowercase().trim_end_matches('\\')
            == bin_dir_str.to_lowercase().trim_end_matches('\\')
    });

    if already_present {
        println!("{} is already on HKCU\\Environment\\Path", bin_dir.display());
        print_verify_hints();
        return Ok(());
    }

    if dry_run {
        println!(
            "[dry-run] would append {} to HKCU\\Environment\\Path (current length: {} entries)",
            bin_dir.display(),
            entries.len()
        );
        return Ok(());
    }

    // Build new PATH; preserve REG_EXPAND_SZ vs REG_SZ.
    let mut new_entries: Vec<String> = entries.iter().map(|s| s.to_string()).collect();
    new_entries.push(bin_dir_str.to_string());
    let new_path = new_entries.join(";");
    let mut wide: Vec<u16> = new_path.encode_utf16().collect();
    wide.push(0);

    let preserve_type = if current_type == REG_EXPAND_SZ { REG_EXPAND_SZ } else { REG_SZ };

    unsafe {
        let mut hkey: HKEY = std::ptr::null_mut();
        let rc = RegOpenKeyExW(
            HKEY_CURRENT_USER,
            subkey.as_ptr(),
            0,
            KEY_READ | KEY_WRITE,
            &mut hkey,
        );
        if rc != ERROR_SUCCESS {
            return Err(miette::miette!(
                "RegOpenKeyExW(HKCU\\Environment, RW) failed with code {rc}"
            ));
        }

        let rc2 = RegSetValueExW(
            hkey,
            path_name.as_ptr(),
            0,
            preserve_type,
            wide.as_ptr() as *const u8,
            (wide.len() * 2) as u32,
        );
        let _ = RegCloseKey(hkey);
        if rc2 != ERROR_SUCCESS {
            return Err(miette::miette!(
                "RegSetValueExW(Path) failed with code {rc2}"
            ));
        }
    }

    // Broadcast WM_SETTINGCHANGE so already-open processes (Explorer, new
    // shells launched from it) refresh their environment. This is best-effort;
    // failure here is non-fatal.
    broadcast_setting_change();

    println!("appended {} to HKCU\\Environment\\Path", bin_dir.display());
    println!(
        "open a NEW terminal (or run: $env:Path = [Environment]::GetEnvironmentVariable('Path','User')) to pick it up"
    );
    print_verify_hints();
    Ok(())
}

#[cfg(target_os = "windows")]
fn broadcast_setting_change() {
    use windows_sys::Win32::Foundation::{HWND, LPARAM, WPARAM};
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        HWND_BROADCAST, SMTO_ABORTIFHUNG, SendMessageTimeoutW, WM_SETTINGCHANGE,
    };

    let env: Vec<u16> = "Environment\0".encode_utf16().collect();
    let mut result: usize = 0;
    unsafe {
        let _ = SendMessageTimeoutW(
            HWND_BROADCAST as HWND,
            WM_SETTINGCHANGE,
            0 as WPARAM,
            env.as_ptr() as LPARAM,
            SMTO_ABORTIFHUNG,
            5000,
            &mut result,
        );
    }
}

// ── Unix: symlink into $HOME/.local/bin ─────────────────────────────────────
#[cfg(not(target_os = "windows"))]
fn platform_install_path(bin: &Path, dry_run: bool) -> miette::Result<()> {
    let home = std::env::var("HOME")
        .into_diagnostic()
        .wrap_err("HOME environment variable is not set")?;
    let link_dir = PathBuf::from(&home).join(".local").join("bin");
    let link = link_dir.join("aphrody");

    if dry_run {
        println!(
            "[dry-run] would create directory {} (if missing)",
            link_dir.display()
        );
        println!(
            "[dry-run] would symlink {} -> {}",
            link.display(),
            bin.display()
        );
    } else {
        std::fs::create_dir_all(&link_dir)
            .into_diagnostic()
            .wrap_err_with(|| format!("failed to create {}", link_dir.display()))?;

        // Remove an existing symlink/file at the target before re-linking so
        // `symlink` does not fail with EEXIST.
        if link.exists() || link.symlink_metadata().is_ok() {
            std::fs::remove_file(&link)
                .into_diagnostic()
                .wrap_err_with(|| format!("failed to remove existing {}", link.display()))?;
        }

        #[cfg(unix)]
        std::os::unix::fs::symlink(bin, &link)
            .into_diagnostic()
            .wrap_err_with(|| format!("failed to symlink {} -> {}", link.display(), bin.display()))?;

        println!("symlinked {} -> {}", link.display(), bin.display());
    }

    // Check PATH membership (informational only — modifying ~/.profile is a
    // user decision we surface as a hint rather than performing).
    let path_var = std::env::var("PATH").unwrap_or_default();
    let on_path = std::env::split_paths(&path_var).any(|p| p == link_dir);
    if on_path {
        println!("{} is already on PATH", link_dir.display());
    } else {
        println!();
        println!("{} is NOT on PATH.", link_dir.display());
        println!("Add this to your shell rc file (~/.bashrc, ~/.zshrc, ~/.profile):");
        println!();
        println!("  export PATH=\"$HOME/.local/bin:$PATH\"");
        println!();
        println!("Then reload: source ~/.bashrc   (or open a new terminal)");
    }

    print_verify_hints();
    Ok(())
}

fn print_verify_hints() {
    println!();
    println!("Verify:");
    println!("  aphrody --version");
    println!("  aphrody doctor");
}

// ────────────────────────────────────────────────────────────────────────────
// `aphrody self bootstrap` — inventory dev toolchain (rustup, cargo, git,
// zigbuild, wasm-bindgen, …). `--check` only prints status; without `--check`
// it attempts `rustup target add` / `rustup component add` to fill the gaps.

pub(crate) struct BootstrapCommand {
    /// Print the inventory and exit; do not install anything.
    pub check: bool,
}

#[async_trait]
impl TerminalCommand for BootstrapCommand {
    async fn execute(&self, _ctx: &GoogleContext) -> miette::Result<()> {
        // Required executables (cross-platform names — `which` honours PATHEXT
        // on Windows so `rustup` resolves to `rustup.exe`).
        let bins = ["rustup", "cargo", "rustc", "git"];
        // Optional but recommended.
        let optional = ["cargo-zigbuild", "wasm-bindgen", "cargo-deny", "cargo-vet"];

        println!("aphrody self bootstrap{}", if self.check { " --check" } else { "" });
        println!();
        println!("Required tools:");
        let mut missing_required: Vec<&str> = Vec::new();
        for name in bins {
            match which::which(name) {
                Ok(p) => println!("  [ok] {name:<18} {}", p.display()),
                Err(_) => {
                    println!("  [--] {name:<18} (missing)");
                    missing_required.push(name);
                },
            }
        }
        println!();
        println!("Optional tools:");
        for name in optional {
            match which::which(name) {
                Ok(p) => println!("  [ok] {name:<18} {}", p.display()),
                Err(_) => println!("  [--] {name:<18} (missing — optional)"),
            }
        }

        // Target inventory (best-effort — needs rustup).
        let targets = [
            "x86_64-unknown-linux-gnu",
            "x86_64-pc-windows-msvc",
            "wasm32-unknown-unknown",
        ];
        println!();
        println!("Rust targets (priority order: Linux > Windows > wasm):");
        let rustup_present = which::which("rustup").is_ok();
        for t in targets {
            if rustup_present {
                let out = std::process::Command::new("rustup")
                    .args(["target", "list", "--installed"])
                    .output();
                match out {
                    Ok(o) => {
                        let installed = String::from_utf8_lossy(&o.stdout);
                        let ok = installed.lines().any(|l| l.trim() == t);
                        println!("  [{}] {t}", if ok { "ok" } else { "--" });
                    },
                    Err(_) => println!("  [??] {t} (rustup query failed)"),
                }
            } else {
                println!("  [??] {t} (rustup missing)");
            }
        }

        if self.check {
            // Non-zero exit if a required tool is missing — useful for CI gates.
            if !missing_required.is_empty() {
                return Err(miette::miette!(
                    "missing required tools: {}",
                    missing_required.join(", ")
                ));
            }
            return Ok(());
        }

        // Install mode — only attempt to fix what rustup can fix; defer
        // OS-level installs (MSVC, NASM, GTK) to platform package managers
        // documented in the README.
        if !rustup_present {
            return Err(miette::miette!(
                "rustup not found on PATH — install from https://rustup.rs and re-run \
                 `aphrody self bootstrap` (use --check to inspect without installing)"
            ));
        }

        println!();
        println!("Installing missing rust targets via rustup...");
        for t in targets {
            let status = std::process::Command::new("rustup")
                .args(["target", "add", t])
                .status()
                .into_diagnostic()
                .wrap_err_with(|| format!("failed to spawn rustup target add {t}"))?;
            if !status.success() {
                eprintln!("  warning: rustup target add {t} exited with code {:?}", status.code());
            }
        }

        println!();
        println!("Installing rust-src component (needed for build-std on wasm/embedded)...");
        let _ = std::process::Command::new("rustup")
            .args(["component", "add", "rust-src"])
            .status();

        Ok(())
    }
}
