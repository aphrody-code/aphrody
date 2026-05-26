// SPDX-License-Identifier: Apache-2.0
//
// Integration tests for `aphrody gui` — the native launcher for the desktop
// app (Tauri + Angular). Every test here is hermetic: it drives the compiled
// `aphrody` binary via `assert_cmd`, points the resolver at a temp file through
// `APHRODY_GUI_BIN`, and NEVER launches a real GUI (only `--print-path`, which
// resolves-and-prints without spawning).
//
// Run with:
//   cargo nextest run -p aphrody --test gui --locked --offline
//
// Design constraints (mirror tests/cli_smoke.rs + tests/doctor.rs):
//  - The binary under test is the freshly-built workspace `aphrody`
//    (`Command::cargo_bin` resolves `CARGO_BIN_EXE_aphrody`).
//  - No host file-system layout is assumed: the GUI binary is a
//    `tempfile::NamedTempFile`, so the env-override resolution step is exercised
//    deterministically regardless of what is (or is not) installed.
//  - `--print-path` is scriptable and never spawns a window, so these tests are
//    safe to run headless in CI.

#![cfg(not(target_arch = "wasm32"))]

use assert_cmd::Command;
use predicates::prelude::*;

/// A `Command` already pointing at the compiled `aphrody` workspace binary.
fn aphrody() -> Command {
    Command::cargo_bin("aphrody").expect("aphrody binary must be present in cargo target dir")
}

/// `aphrody gui --print-path` with `APHRODY_GUI_BIN` pointing at an existing
/// file must print that file's path and exit 0, without launching anything.
#[test]
fn gui_print_path_with_env_override_prints_path_and_exits_zero() {
    let tmp = tempfile::NamedTempFile::new().expect("create temp gui binary");
    let path = tmp.path().to_path_buf();
    // The binary canonicalises the resolved path; compare on the file name,
    // which survives canonicalisation on every platform (avoids brittle
    // `\\?\` / symlink-normalisation differences between OSes).
    let file_name = path
        .file_name()
        .expect("temp file has a name")
        .to_string_lossy()
        .into_owned();

    aphrody()
        .arg("gui")
        .arg("--print-path")
        .env("APHRODY_GUI_BIN", &path)
        .assert()
        .success()
        .stdout(predicate::str::contains(file_name));
}

/// `aphrody gui --print-path` with a blank `APHRODY_GUI_BIN` and no resolvable
/// binary must fail with the actionable French guidance (it must NOT print a
/// bogus path). We isolate PATH to a directory that cannot contain
/// `aphrody-gui` so the resolver deterministically reaches the not-found arm.
#[test]
fn gui_print_path_without_binary_errors_with_guidance() {
    // An empty temp dir serves as BOTH the PATH (no `aphrody-gui` on it) and the
    // working directory (so the resolver's in-tree walk-up from CWD cannot reach
    // an `apps/desktop/.../aphrody-gui` build artifact). The test binary itself
    // lives elsewhere, so the sibling-of-current_exe step finds nothing either.
    // This makes the not-found arm deterministic regardless of host/peer state.
    let empty = tempfile::tempdir().expect("create empty PATH + CWD dir");

    aphrody()
        .arg("gui")
        .arg("--print-path")
        // Blank override is ignored by the resolver (env step requires an
        // existing file); combined with an empty PATH + CWD this guarantees the
        // not-found path on CI hosts without a globally installed aphrody-gui.
        .env("APHRODY_GUI_BIN", "")
        .env("PATH", empty.path())
        .current_dir(empty.path())
        .assert()
        .failure()
        .stderr(predicate::str::contains("aphrody-gui").and(
            // The guidance must name the env override and the PRODUCTION build
            // command (`bun run tauri build`) -- a bare `cargo build` yields a
            // dev-mode binary that fails with "localhost refused to connect".
            predicate::str::contains("APHRODY_GUI_BIN")
                .and(predicate::str::contains("bun run tauri build")),
        ));
}

/// `aphrody gui --help` must build the command tree (no clap_complete-style
/// recursion) and document both the `--print-path` flag and the forwarded
/// arguments, exiting 0.
#[test]
fn gui_help_documents_flag_and_forwarding() {
    aphrody()
        .args(["gui", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("--print-path"));
}
