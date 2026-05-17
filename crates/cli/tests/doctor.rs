// SPDX-License-Identifier: Apache-2.0
//
// Integration tests for `aphrody doctor` (text + JSON output paths).
//
// Run with:
//   cargo nextest run -p aphrody --test doctor --locked --offline
//
// The tests invoke the compiled `aphrody` binary via `assert_cmd` so they
// exercise the full CLI surface including argument parsing and the
// `DoctorCommand` execution path.
//
// Design constraints:
//  - The peer winclean may be offline (DEGRADED verdict is expected and
//    treated as a passing condition by the binary itself — exit code 0).
//  - Tests must not assume any specific file-system layout beyond what the
//    doctor subcommand itself discovers at runtime.
//  - No network calls are made by the test harness; HTTP reachability is
//    exercised only by the binary under test (with a 2-second timeout built
//    in to `check_http_listener`).

#![cfg(not(target_arch = "wasm32"))]

use assert_cmd::Command;
use predicates::prelude::*;

// ---------------------------------------------------------------------------
// Helper: build a Command already pointing at the `aphrody` workspace binary.
// ---------------------------------------------------------------------------
fn aphrody() -> Command {
    Command::cargo_bin("aphrody").expect("aphrody binary must be present in cargo target dir")
}

// ---------------------------------------------------------------------------
// 1. Text output — [runtime] section
// ---------------------------------------------------------------------------
#[test]
fn doctor_text_runtime_section_present() {
    aphrody()
        .arg("doctor")
        .assert()
        .stdout(predicate::str::contains("[runtime]"))
        .stdout(predicate::str::contains("binary version:"))
        .stdout(predicate::str::contains("rustls CryptoProvider:"));
}

// ---------------------------------------------------------------------------
// 2. Text output — [peer A2A coord] section
// ---------------------------------------------------------------------------
#[test]
fn doctor_text_peer_section_present() {
    aphrody()
        .arg("doctor")
        .assert()
        .stdout(predicate::str::contains("[peer A2A coord]"))
        .stdout(predicate::str::contains("peer winclean:"))
        .stdout(predicate::str::contains("HTTP listener:"));
}

// ---------------------------------------------------------------------------
// 3. Text output — [supply chain] section
// ---------------------------------------------------------------------------
#[test]
fn doctor_text_supply_chain_section_present() {
    aphrody()
        .arg("doctor")
        .assert()
        .stdout(predicate::str::contains("[supply chain]"))
        .stdout(predicate::str::contains("cargo-vet audits:"));
}

// ---------------------------------------------------------------------------
// 4. Text output — Verdict line (one of HEALTHY / DEGRADED / UNHEALTHY)
// ---------------------------------------------------------------------------
#[test]
fn doctor_text_verdict_present() {
    let output = aphrody()
        .arg("doctor")
        .output()
        .expect("aphrody doctor must produce output");

    let stdout = String::from_utf8_lossy(&output.stdout);
    let has_verdict = stdout.contains("Verdict: HEALTHY")
        || stdout.contains("Verdict: DEGRADED")
        || stdout.contains("Verdict: UNHEALTHY");

    assert!(
        has_verdict,
        "stdout did not contain any recognised Verdict line.\nstdout:\n{stdout}"
    );
}

// ---------------------------------------------------------------------------
// 5. JSON output — valid serde + required top-level keys
// ---------------------------------------------------------------------------
#[test]
fn doctor_json_valid_serde() {
    let output = aphrody()
        .args(["doctor", "--json"])
        .output()
        .expect("aphrody doctor --json must produce output");

    assert!(
        output.status.success() || output.status.code() == Some(1),
        "aphrody doctor --json exited with unexpected code {:?}",
        output.status.code()
    );

    let value: serde_json::Value = serde_json::from_slice(&output.stdout)
        .expect("aphrody doctor --json stdout must be valid JSON");

    for key in &["runtime", "peer_a2a", "supply_chain", "environment", "verdict"] {
        assert!(
            value.get(key).is_some(),
            "JSON output is missing top-level key `{key}`.\nJSON:\n{}",
            serde_json::to_string_pretty(&value).unwrap_or_default()
        );
    }

    // `verdict` must be a string with a recognisable value.
    let verdict = value["verdict"]
        .as_str()
        .expect("JSON `verdict` field must be a string");
    assert!(
        matches!(verdict, "HEALTHY" | "DEGRADED" | "UNHEALTHY"),
        "unexpected verdict value: {verdict:?}"
    );
}

// ---------------------------------------------------------------------------
// 6. Exit code — 0 even when DEGRADED (DEGRADED is non-fatal per spec)
// ---------------------------------------------------------------------------
#[test]
fn doctor_exit_code_is_zero() {
    let output = aphrody()
        .arg("doctor")
        .output()
        .expect("aphrody doctor must produce output");

    let stdout = String::from_utf8_lossy(&output.stdout);

    // UNHEALTHY triggers exit(1) by design; guard against that edge so we
    // only assert exit-0 when the binary itself considers the run non-fatal.
    let is_unhealthy = stdout.contains("Verdict: UNHEALTHY");
    if !is_unhealthy {
        assert!(
            output.status.success(),
            "aphrody doctor must exit 0 when verdict is HEALTHY or DEGRADED.\nstdout:\n{stdout}"
        );
    }
}
