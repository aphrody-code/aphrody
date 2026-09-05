// SPDX-License-Identifier: Apache-2.0
//
// Offline-safe integration smoke tests for the headline `aphrody` CLI
// commands, acting as a regression gate. Every test here is hermetic: it makes
// ZERO network calls and only exercises pure-CPU / filesystem subcommands
// (`version`, `re {triage,strings,sections}`, `scan tree`, `completions`).
//
// Run with:
//   cargo nextest run -p aphrody --test cli_smoke --locked
//   cargo test       -p aphrody --test cli_smoke --locked
//
// Design constraints (mirrors `tests/doctor.rs`):
//  - The binary under test is the freshly-built `aphrody` workspace binary,
//    invoked through `assert_cmd` so the full argument-parsing + dispatch path
//    is covered.
//  - No assumption is made about any host file-system layout: every input is
//    created in a `tempfile::tempdir()` or is the test binary itself
//    (`Command::cargo_bin` resolves `CARGO_BIN_EXE_aphrody`).
//  - The `aphrody` binary is itself a real PE (Windows) / ELF (Linux/macOS)
//    executable, so `re triage`/`re sections` against it detect a concrete
//    format and a non-empty section table on every supported platform.
//  - No subcommand here touches the network: `re`/`scan`/`completions`/
//    `version` are all local. The TOS/`--accept-tos` gate that the
//    orchestrator hypothesised for `re` does NOT exist in the codebase (see
//    the module-level note on `re_triage_self_detects_format`), so no
//    acceptance flag is required.

#![cfg(not(target_arch = "wasm32"))]

use std::path::Path;

use assert_cmd::Command;
use predicates::prelude::*;
use serde_json::Value;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// A `Command` already pointing at the compiled `aphrody` workspace binary.
fn aphrody() -> Command {
    Command::cargo_bin("aphrody").expect("aphrody binary must be present in cargo target dir")
}

/// Absolute path to the `aphrody` binary under test — a real native
/// executable, used as a known-good triage/sections input.
fn self_bin() -> String {
    // `CARGO_BIN_EXE_<name>` is injected by cargo for integration tests and
    // points at the very binary `Command::cargo_bin` runs. Using it avoids any
    // hard-coded, machine-specific path.
    env!("CARGO_BIN_EXE_aphrody").to_string()
}

// ===========================================================================
// 1. `aphrody version`
// ===========================================================================

/// `aphrody version --json` must exit 0 and emit a JSON object whose schema
/// matches `VersionCommand::execute` in `crates/cli/src/commands.rs` exactly:
///   { version, commit, built, target, profile, repo, license, a2a }
/// Note: there is NO `name` key — the literal `aphrody` only appears in the
/// text form (`aphrody <version>`), so we assert the keys that truly exist.
#[test]
fn version_json_has_expected_schema() {
    let output = aphrody()
        .args(["version", "--json"])
        .output()
        .expect("aphrody version --json must produce output");

    assert!(
        output.status.success(),
        "aphrody version --json must exit 0, got {:?}",
        output.status.code()
    );

    let value: Value = serde_json::from_slice(&output.stdout)
        .expect("aphrody version --json stdout must be valid JSON");

    // Every key declared in VersionCommand::execute must be present and a string.
    for key in &["version", "commit", "built", "target", "profile", "repo", "license", "a2a"] {
        let field = value
            .get(*key)
            .unwrap_or_else(|| panic!("version --json missing key `{key}`.\nJSON:\n{value:#}"));
        assert!(field.is_string(), "version --json key `{key}` must be a string, got {field:?}");
    }

    // Pin the load-bearing constant fields to their compiled-in values.
    assert_eq!(value["version"], env!("CARGO_PKG_VERSION"), "version must echo CARGO_PKG_VERSION");
    assert_eq!(value["license"], "Apache-2.0", "license field must be Apache-2.0");
    assert_eq!(
        value["repo"], "https://github.com/aphrody-code/aphrody",
        "repo field must be the canonical repository URL"
    );
}

/// `aphrody version` (text form) must exit 0 and print the `aphrody <version>`
/// banner plus the flat key:value lines (`license:`, `commit:`).
#[test]
fn version_text_exit_zero_and_banner() {
    aphrody()
        .arg("version")
        .assert()
        .success()
        .stdout(predicate::str::contains(format!("aphrody {}", env!("CARGO_PKG_VERSION"))))
        .stdout(predicate::str::contains("license:   Apache-2.0"))
        .stdout(predicate::str::contains("commit:"));
}

// ===========================================================================
// 2. `aphrody re triage`
// ===========================================================================

/// `aphrody re triage <self>` triages the test binary itself — a real PE/ELF —
/// so the detected `format` is one of the known executable variants and the
/// section table is non-empty. Output is compact JSON on stdout.
///
/// IMPORTANT (orchestrator hypothesis refuted): the `re` dispatch in
/// `crates/cli/src/lib.rs` (the `ReAction::Triage` arm) reads the file and
/// calls `aphrody_re::triage` directly. There is NO `--accept-tos` flag and no
/// first-run TOS warning anywhere in the `re` code path, so none is passed.
#[test]
fn re_triage_self_detects_format() {
    let output = aphrody()
        .args(["re", "triage", &self_bin()])
        .output()
        .expect("aphrody re triage must produce output");

    assert!(
        output.status.success(),
        "aphrody re triage <self> must exit 0, got {:?}\nstderr:\n{}",
        output.status.code(),
        String::from_utf8_lossy(&output.stderr)
    );

    let report: Value =
        serde_json::from_slice(&output.stdout).expect("re triage stdout must be valid JSON");

    // `format` is serialised lowercase by aphrody-re::Format (#[serde(rename_all
    // = "lowercase")]): pe32/pe64/elf32/elf64. The test binary is always one of
    // these on a supported host — never "unknown".
    let format = report["format"].as_str().expect("triage `format` must be a string");
    assert!(
        matches!(format, "pe32" | "pe64" | "elf32" | "elf64"),
        "triage of the aphrody binary should detect a real executable format, got {format:?}"
    );

    // A real executable always carries an architecture and at least one section.
    assert!(
        report["arch"].is_string(),
        "triage of a real binary must report an `arch`, got {:?}",
        report["arch"]
    );
    let sections = report["sections"].as_array().expect("triage `sections` must be an array");
    assert!(!sections.is_empty(), "triage of a real binary must list at least one section");

    // SHA-256 is always populated (64 lowercase hex chars).
    let sha = report["sha256"].as_str().expect("triage `sha256` must be a string");
    assert_eq!(sha.len(), 64, "sha256 must be 64 hex chars, got {} chars", sha.len());
    assert!(
        sha.chars().all(|c| c.is_ascii_hexdigit()),
        "sha256 must be hex-only, got {sha:?}"
    );

    // `size` must be the byte length of the file we triaged.
    let on_disk = std::fs::metadata(self_bin()).expect("binary must be stat-able").len();
    assert_eq!(
        report["size"].as_u64(),
        Some(on_disk),
        "triage `size` must equal the on-disk byte length"
    );
}

/// `aphrody re triage --pretty` must still exit 0 and emit a multi-line
/// (indented) JSON object that parses back to the same `format` value.
#[test]
fn re_triage_pretty_is_valid_json() {
    let output = aphrody()
        .args(["re", "triage", &self_bin(), "--pretty"])
        .output()
        .expect("aphrody re triage --pretty must produce output");

    assert!(output.status.success(), "re triage --pretty must exit 0");

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains('\n'), "--pretty output should span multiple lines");

    let report: Value =
        serde_json::from_str(&stdout).expect("re triage --pretty stdout must be valid JSON");
    assert!(report["format"].is_string(), "pretty triage must still carry a `format`");
}

// ===========================================================================
// 3. `aphrody re strings` / `aphrody re sections`
// ===========================================================================

/// `aphrody re strings <blob>` extracts contiguous printable runs. We feed a
/// synthetic blob with two known ASCII markers and assert both are returned
/// (JSON array), proving the extractor reflects the real input. Using a small
/// in-tempdir blob keeps the test fast and deterministic.
#[test]
fn re_strings_extracts_known_markers() {
    let dir = tempfile::tempdir().expect("tempdir");
    let blob = dir.path().join("blob.bin");

    // Two readable markers (>= 6 chars) separated by non-printable bytes plus a
    // sub-min-length run (`AB`) that must be filtered out at --min-len 6.
    let mut bytes = Vec::new();
    bytes.extend_from_slice(b"FIRST_MARKER_STRING");
    bytes.extend_from_slice(&[0x00, 0x01, 0x02, 0x03]);
    bytes.extend_from_slice(b"SECOND_MARKER_STRING");
    bytes.extend_from_slice(&[0xff, 0xfe]);
    bytes.extend_from_slice(b"AB"); // shorter than min-len 6 -> excluded
    std::fs::write(&blob, &bytes).expect("write blob");

    let output = aphrody()
        .args(["re", "strings", blob.to_str().unwrap(), "--min-len", "6"])
        .output()
        .expect("aphrody re strings must produce output");

    assert!(
        output.status.success(),
        "aphrody re strings must exit 0, got {:?}",
        output.status.code()
    );

    let strings: Vec<String> =
        serde_json::from_slice(&output.stdout).expect("re strings stdout must be a JSON array");

    assert!(
        strings.iter().any(|s| s == "FIRST_MARKER_STRING"),
        "expected FIRST_MARKER_STRING in {strings:?}"
    );
    assert!(
        strings.iter().any(|s| s == "SECOND_MARKER_STRING"),
        "expected SECOND_MARKER_STRING in {strings:?}"
    );
    assert!(
        !strings.iter().any(|s| s == "AB"),
        "sub-min-length run `AB` must be filtered out, got {strings:?}"
    );
}

/// `aphrody re strings --limit N` must cap the number of returned strings.
#[test]
fn re_strings_honours_limit() {
    let dir = tempfile::tempdir().expect("tempdir");
    let blob = dir.path().join("many.bin");

    // Ten distinct markers separated by NULs.
    let mut bytes = Vec::new();
    for i in 0..10u8 {
        bytes.extend_from_slice(format!("MARKER_NUMBER_{i:02}").as_bytes());
        bytes.push(0x00);
    }
    std::fs::write(&blob, &bytes).expect("write blob");

    let output = aphrody()
        .args(["re", "strings", blob.to_str().unwrap(), "--min-len", "6", "--limit", "3"])
        .output()
        .expect("aphrody re strings --limit must produce output");

    assert!(output.status.success(), "re strings --limit must exit 0");

    let strings: Vec<String> =
        serde_json::from_slice(&output.stdout).expect("re strings stdout must be a JSON array");
    assert!(
        strings.len() <= 3,
        "--limit 3 must cap output to at most 3 strings, got {} ({strings:?})",
        strings.len()
    );
}

/// `aphrody re sections <self>` parses the test binary and emits a non-empty
/// JSON array of section descriptors, each with `name`/`vaddr`/`size` fields.
#[test]
fn re_sections_self_lists_sections() {
    let output = aphrody()
        .args(["re", "sections", &self_bin()])
        .output()
        .expect("aphrody re sections must produce output");

    assert!(
        output.status.success(),
        "aphrody re sections <self> must exit 0, got {:?}",
        output.status.code()
    );

    let sections: Value =
        serde_json::from_slice(&output.stdout).expect("re sections stdout must be valid JSON");
    let arr = sections.as_array().expect("re sections output must be a JSON array");
    assert!(!arr.is_empty(), "a real binary must expose at least one section");

    // Each row must carry the documented Section fields.
    let first = &arr[0];
    assert!(first["name"].is_string(), "section row must have a string `name`");
    assert!(first["vaddr"].is_u64(), "section row must have an unsigned `vaddr`");
    assert!(first["size"].is_u64(), "section row must have an unsigned `size`");
}

// ===========================================================================
// 4. `aphrody scan tree`
// ===========================================================================

/// `aphrody scan tree --root <tmp> --groups crates -o <file>` walks a known
/// tree and must produce a JSON report that reflects the created layout: two
/// top-level directories under `crates/` totalling three files. We write the
/// report to a tempfile (stdout also carries a human summary, so the file is
/// the clean JSON channel) and parse it back.
#[test]
fn scan_tree_reflects_known_layout() {
    let dir = tempfile::tempdir().expect("tempdir");
    let root = dir.path();

    // crates/alpha: 2 files (a.rs + b.txt); crates/beta/sub: 1 file (c.rs).
    let alpha = root.join("crates").join("alpha");
    let beta_sub = root.join("crates").join("beta").join("sub");
    std::fs::create_dir_all(&alpha).expect("mkdir alpha");
    std::fs::create_dir_all(&beta_sub).expect("mkdir beta/sub");
    std::fs::write(alpha.join("a.rs"), b"aaaa").expect("write a.rs");
    std::fs::write(alpha.join("b.txt"), b"bbbbbb").expect("write b.txt");
    std::fs::write(beta_sub.join("c.rs"), b"cccccccc").expect("write c.rs");

    let report_path = root.join("report.json");

    aphrody()
        .args([
            "scan",
            "tree",
            "--root",
            root.to_str().unwrap(),
            "--groups",
            "crates",
            "-o",
            report_path.to_str().unwrap(),
        ])
        .assert()
        .success();

    let raw = std::fs::read_to_string(&report_path).expect("scan tree must write the report file");
    let report: Value = serde_json::from_str(&raw).expect("scan tree report must be valid JSON");

    // byGroup must contain exactly the `crates` group with 2 dirs / 3 files.
    let by_group = report["byGroup"].as_array().expect("`byGroup` must be an array");
    let crates = by_group
        .iter()
        .find(|g| g["group"] == "crates")
        .expect("byGroup must contain the `crates` group");
    assert_eq!(crates["dirs"].as_u64(), Some(2), "crates group must report 2 top-level dirs");
    assert_eq!(crates["files"].as_u64(), Some(3), "crates group must report 3 files total");
    // bytes = 4 + 6 + 8 = 18.
    assert_eq!(crates["bytes"].as_u64(), Some(18), "crates group total bytes must be 18");

    // directories[] must name both alpha and beta.
    let dirs = report["directories"].as_array().expect("`directories` must be an array");
    let names: Vec<&str> = dirs.iter().filter_map(|d| d["name"].as_str()).collect();
    assert!(names.contains(&"alpha"), "directories must include `alpha`, got {names:?}");
    assert!(names.contains(&"beta"), "directories must include `beta`, got {names:?}");

    // alpha holds 2 files; beta (recursively, incl. sub/) holds 1 file.
    let alpha_entry = dirs.iter().find(|d| d["name"] == "alpha").unwrap();
    let beta_entry = dirs.iter().find(|d| d["name"] == "beta").unwrap();
    assert_eq!(alpha_entry["files"].as_u64(), Some(2), "alpha must report 2 files");
    assert_eq!(beta_entry["files"].as_u64(), Some(1), "beta must report 1 file (incl. sub/)");

    // The root recorded in the report must point at our tempdir.
    let reported_root = report["root"].as_str().expect("report must carry a `root` string");
    assert!(
        Path::new(reported_root).ends_with(root.file_name().unwrap()),
        "report root {reported_root:?} should reference the tempdir {root:?}"
    );
}

// ===========================================================================
// 5. `aphrody completions <shell>`
// ===========================================================================

/// `aphrody completions bash` must exit 0 and emit a bash completion script
/// carrying the `_aphrody` function and a `complete` registration.
#[test]
fn completions_bash_emits_script() {
    aphrody()
        .args(["completions", "bash"])
        .assert()
        .success()
        .stdout(predicate::str::contains("_aphrody"))
        .stdout(predicate::str::contains("complete"));
}

/// `aphrody completions zsh` must exit 0 and emit a zsh completion script
/// (the `#compdef aphrody` header is the canonical zsh marker).
#[test]
fn completions_zsh_emits_script() {
    aphrody()
        .args(["completions", "zsh"])
        .assert()
        .success()
        .stdout(predicate::str::contains("#compdef aphrody"))
        .stdout(predicate::str::contains("_aphrody"));
}

/// `aphrody completions pwsh` (alias of `powershell`) must exit 0 and emit a
/// PowerShell completion script with the `Register-ArgumentCompleter` call.
#[test]
fn completions_pwsh_emits_script() {
    aphrody()
        .args(["completions", "pwsh"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Register-ArgumentCompleter"))
        .stdout(predicate::str::contains("aphrody"));
}

/// An invalid shell value must be rejected by clap with a non-zero exit (the
/// `value_enum` only accepts bash/elvish/fish/powershell|pwsh/zsh).
#[test]
fn completions_rejects_unknown_shell() {
    aphrody().args(["completions", "not-a-shell"]).assert().failure();
}
