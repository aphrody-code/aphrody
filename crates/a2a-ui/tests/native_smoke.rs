// SPDX-License-Identifier: Apache-2.0
//! Smoke tests for the native TUI backend.
//!
//! These tests run only when the `native` feature is enabled and do not
//! require an interactive TTY.

#![cfg(feature = "native")]

use a2a_ui::Envelope;
use a2a_ui::native::mailbox::MailboxPair;
use a2a_ui::native::ui::{DrawState, Palette, draw};
use ratatui::{Terminal, backend::TestBackend, widgets::ListState};
use std::time::SystemTime;

// ---------------------------------------------------------------------------
// Sample envelopes used across tests
// ---------------------------------------------------------------------------

const SAMPLE_A: &str = r#"{"v":1,"ts":"2026-05-17T20:02:27Z","from":"aphrody","to":"winclean","kind":"fact","topic":"workspace-gate-exit","body":"exit 0"}"#;
const SAMPLE_B: &str = r#"{"v":1,"ts":"2026-05-17T21:00:00Z","from":"aphrody","to":"winclean","kind":"ask","topic":"confirm-build","body":"did it pass?"}"#;
const SAMPLE_C: &str = r#"{"ts":"2026-05-17T22:00:00Z","from":"winclean","type":"ack","subject":"build-ack","body":"yes"}"#;

fn sample_envelopes() -> Vec<Envelope> {
    Envelope::parse_jsonl(&format!("{SAMPLE_A}\n{SAMPLE_B}\n{SAMPLE_C}\n"))
        .expect("sample JSONL must parse cleanly")
}

// ---------------------------------------------------------------------------
// Test: parse 3 sample envelopes
// ---------------------------------------------------------------------------

#[test]
fn parse_three_sample_envelopes() {
    let envelopes = sample_envelopes();
    assert_eq!(envelopes.len(), 3, "expected exactly 3 envelopes");

    assert_eq!(envelopes[0].kind, "fact");
    assert_eq!(envelopes[0].from, "aphrody");
    assert_eq!(envelopes[0].topic, "workspace-gate-exit");

    assert_eq!(envelopes[1].kind, "ask");
    assert_eq!(envelopes[1].topic, "confirm-build");

    // Legacy alias: `type` -> `kind`, `subject` -> `topic`
    assert_eq!(envelopes[2].kind, "ack");
    assert_eq!(envelopes[2].topic, "build-ack");
}

// ---------------------------------------------------------------------------
// Test: one frame draw into a TestBackend — must not panic
// ---------------------------------------------------------------------------

#[test]
fn one_frame_draw_does_not_panic() {
    let backend = TestBackend::new(120, 30);
    let mut terminal = Terminal::new(backend).expect("TestBackend terminal creation");
    let palette = Palette::resolve();
    let envelopes = sample_envelopes();
    let matches: Vec<usize> = Vec::new();
    let mut list_state = ListState::default();
    list_state.select(Some(0));

    terminal
        .draw(|frame| {
            let mut state = DrawState {
                envelopes: &envelopes,
                list_state: &mut list_state,
                selected: Some(0),
                active_side: "aphrody",
                aphrody_hb: Some(SystemTime::now()),
                winclean_hb: None,
                search: None,
                search_matches: &matches,
                palette: &palette,
            };
            draw(frame, &mut state);
        })
        .expect("draw must not return an error");
}

// ---------------------------------------------------------------------------
// Test: mailbox refresh on a temp directory with a fixture JSONL file
// ---------------------------------------------------------------------------

#[test]
fn mailbox_refresh_reads_fixture_file() {
    let dir = tempfile::tempdir().expect("create tempdir");
    let inbox_path = dir.path().join("inbox-from-aphrody.jsonl");
    std::fs::write(
        &inbox_path,
        format!("{SAMPLE_A}\n{SAMPLE_B}\n"),
    )
    .expect("write fixture");

    let mut pair = MailboxPair::new(dir.path());
    let changed = pair.refresh().expect("refresh must succeed");

    assert!(changed, "first refresh should report changed=true");
    assert_eq!(pair.from_aphrody.envelopes.len(), 2);
    assert_eq!(pair.from_winclean.envelopes.len(), 0);

    // Second refresh without touching the file — should be unchanged.
    let changed2 = pair.refresh().expect("second refresh");
    assert!(!changed2, "second refresh with no mtime change should be false");
}
