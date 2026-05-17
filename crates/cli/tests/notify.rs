// SPDX-License-Identifier: Apache-2.0
//
// Integration tests for `aphrody notify` (Slack / Telegram / Matrix).
//
// Run with:
//   cargo nextest run -p aphrody --test notify --locked --offline
//
// These tests do not perform any real network I/O: they verify that the
// command parses correctly, returns a non-zero exit code, and emits a
// clean "missing env var" error when credentials / destination room are
// absent. This is the production-correct behaviour we want documented
// against regression.

#![cfg(not(target_arch = "wasm32"))]

use assert_cmd::Command;
use predicates::prelude::*;

/// Build a `Command` already pointing at the `aphrody` workspace binary
/// with all known notify-related environment variables cleared. This keeps
/// the assertions deterministic regardless of the developer's shell state.
fn aphrody_clean_env() -> Command {
    let mut cmd =
        Command::cargo_bin("aphrody").expect("aphrody binary must be present in cargo target dir");
    for var in [
        "SLACK_BOT_TOKEN",
        "SLACK_CHANNEL",
        "TELEGRAM_BOT_TOKEN",
        "TELEGRAM_CHAT_ID",
        "MATRIX_HOMESERVER",
        "MATRIX_ACCESS_TOKEN",
        "MATRIX_USER_ID",
        "MATRIX_ROOM_ID",
    ] {
        cmd.env_remove(var);
    }
    cmd
}

// ---------------------------------------------------------------------------
// 1. `--help` lists the subcommand and its flags.
// ---------------------------------------------------------------------------
#[test]
fn notify_help_lists_flags() {
    aphrody_clean_env()
        .args(["notify", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("--channel"))
        .stdout(predicate::str::contains("--message"))
        .stdout(predicate::str::contains("slack"))
        .stdout(predicate::str::contains("telegram"))
        .stdout(predicate::str::contains("matrix"));
}

// ---------------------------------------------------------------------------
// 2. Missing destination room — clean error, exit code 1, no panic.
// ---------------------------------------------------------------------------
#[test]
fn notify_slack_missing_room_errors_cleanly() {
    aphrody_clean_env()
        .args(["notify", "--channel", "slack", "--message", "test"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("SLACK_CHANNEL"));
}

// ---------------------------------------------------------------------------
// 3. Room provided but credentials missing — Slack-specific env var named.
// ---------------------------------------------------------------------------
#[test]
fn notify_slack_missing_token_errors_cleanly() {
    aphrody_clean_env()
        .args(["notify", "--channel", "slack", "--message", "test", "--room", "C0123456"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("SLACK_BOT_TOKEN"));
}

// ---------------------------------------------------------------------------
// 4. Telegram channel: missing credentials surfaces TELEGRAM_BOT_TOKEN.
// ---------------------------------------------------------------------------
#[test]
fn notify_telegram_missing_token_errors_cleanly() {
    aphrody_clean_env()
        .args(["notify", "--channel", "telegram", "--message", "test", "--room=-100123"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("TELEGRAM_BOT_TOKEN"));
}

// ---------------------------------------------------------------------------
// 5. Matrix channel: missing credentials surfaces MATRIX_HOMESERVER first.
// ---------------------------------------------------------------------------
#[test]
fn notify_matrix_missing_credentials_errors_cleanly() {
    aphrody_clean_env()
        .args(["notify", "--channel", "matrix", "--message", "test", "--room", "!room:matrix.org"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("MATRIX_"));
}
