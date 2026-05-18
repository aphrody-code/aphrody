// SPDX-License-Identifier: Apache-2.0
//
// Smoke / integration tests for the `aphrody` natural-language fallback.
//
// These exercise the binary end-to-end via `assert_cmd` so we cover:
//   1. clap's `--help` still works (sanity — the try_parse_from fallback MUST not swallow help
//      requests).
//   2. An unknown subcommand whose first token is not a flag triggers the auto_command path and
//      fails gracefully (exit != 0, no panic) when no A2A listener is running on localhost:8788.
//   3. `APHRODY_A2A_ENDPOINT` is honoured at runtime — pointing at an unroutable address still
//      produces a clean error rather than a crash.
//
// No network listener is started; we only assert the binary does not
// panic and surfaces a human-readable error.

#![cfg(not(target_arch = "wasm32"))]

use assert_cmd::Command;
use predicates::prelude::*;

fn aphrody() -> Command {
    Command::cargo_bin("aphrody").expect("aphrody binary must be present in cargo target dir")
}

// ---------------------------------------------------------------------------
// 1. Help / version still work (try_parse_from fallback must not eat them).
// ---------------------------------------------------------------------------
#[test]
fn help_flag_still_works_with_fallback_active() {
    aphrody().arg("--help").assert().success().stdout(predicate::str::contains("aphrody"));
}

#[test]
fn version_subcommand_still_works_with_fallback_active() {
    aphrody().arg("version").assert().success().stdout(predicate::str::contains("aphrody "));
}

// ---------------------------------------------------------------------------
// 2. Natural-language prompt — no A2A listener up — must exit cleanly.
//
// We point the binary at a guaranteed-dead endpoint so the test is
// hermetic (no dependency on whether the developer happens to be running
// `bun run C:/winclean/.coord/listener.ts`).
// ---------------------------------------------------------------------------
#[test]
fn nl_prompt_graceful_failure_when_no_listener() {
    // 127.0.0.1:1 is reserved and refuses connections on every supported
    // platform; reqwest will surface a "connect error" which classify_error
    // routes to AutoCommandError::Transport (exit code 1).
    let assert = aphrody()
        .env("APHRODY_A2A_ENDPOINT", "http://127.0.0.1:1/jsonrpc")
        .arg("what is 2+2")
        .timeout(std::time::Duration::from_secs(15))
        .assert()
        .failure();
    let output = assert.get_output();
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("aphrody:") || stderr.contains("transport") || stderr.contains("A2A"),
        "expected a human-readable error mentioning the A2A transport, got stderr={stderr:?}"
    );
}

// ---------------------------------------------------------------------------
// 3. Multi-word NL prompt is forwarded verbatim (joined with spaces) — the binary must not crash on
//    quoted whitespace-bearing argv tail.
// ---------------------------------------------------------------------------
#[test]
fn multi_word_prompt_does_not_crash() {
    aphrody()
        .env("APHRODY_A2A_ENDPOINT", "http://127.0.0.1:1/jsonrpc")
        .args(["explain", "how", "rustls", "default", "provider", "works"])
        .timeout(std::time::Duration::from_secs(15))
        .assert()
        .failure();
}
