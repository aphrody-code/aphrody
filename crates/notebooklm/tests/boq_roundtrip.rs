// SPDX-License-Identifier: Apache-2.0
//! End-to-end fixture tests for the Boq encoder + parser pair.
//!
//! No network: every test feeds a synthetic envelope through the pure parser
//! and asserts the recovered inner values.

use notebooklm::boq;
use serde_json::json;

fn make_response(rpc_id: &str, inner: &serde_json::Value) -> String {
    let inner_str = inner.to_string();
    let envelope = json!([["wrb.fr", rpc_id, inner_str, null, null, null, "generic"]]);
    let envelope_str = envelope.to_string();
    format!(")]}}'\n{}\n{}", envelope_str.len(), envelope_str)
}

#[test]
fn encode_f_req_produces_expected_outer_shape() {
    let payload = json!(["nb-1", null, [2]]);
    let encoded = boq::encode_f_req(notebooklm::rpc_ids::CREATE_NOTEBOOK, &payload).unwrap();
    // Outer is [[[<id>,<inner>,null,"generic"]]]
    assert!(encoded.starts_with("[[["));
    assert!(encoded.ends_with("]]]"));
    assert!(encoded.contains("CCqFvf"));
    assert!(encoded.contains("\\\"nb-1\\\""));
    assert!(encoded.contains("generic"));
}

#[test]
fn parse_envelopes_recovers_inner_payload() {
    let inner = json!(["nb-7", null, ["thread-7"]]);
    let raw = make_response(notebooklm::rpc_ids::CREATE_NOTEBOOK, &inner);
    let envelopes = boq::parse_envelopes(&raw);
    assert_eq!(envelopes.len(), 1);
    assert_eq!(envelopes[0][0].as_str(), Some("nb-7"));
    assert!(envelopes[0][1].is_null());
    assert_eq!(envelopes[0][2][0].as_str(), Some("thread-7"));
}

#[test]
fn parse_envelopes_handles_arrays_of_arrays() {
    let inner = json!([[["src-1", "doc title"], ["src-2", "second"]]]);
    let raw = make_response(notebooklm::rpc_ids::ADD_SOURCE, &inner);
    let envelopes = boq::parse_envelopes(&raw);
    assert_eq!(envelopes.len(), 1);
    assert_eq!(envelopes[0][0][0][0].as_str(), Some("src-1"));
    assert_eq!(envelopes[0][0][1][0].as_str(), Some("src-2"));
}

#[test]
fn parse_envelopes_returns_empty_for_truncated_input() {
    assert!(boq::parse_envelopes("").is_empty());
    assert!(boq::parse_envelopes(")]}'\n").is_empty());
    assert!(boq::parse_envelopes(")]}'\n42\n[broken").is_empty());
}

#[test]
fn parse_envelopes_skips_non_wrb_envelopes() {
    let mixed = json!([
        ["di", 9000, "some unrelated payload"],
        ["wrb.fr", "wXbhsf", "[[\"nb-only\"]]", null, null, null, "generic"]
    ]);
    let inner_str = mixed.to_string();
    let raw = format!(")]}}'\n{}\n{}", inner_str.len(), inner_str);
    let envelopes = boq::parse_envelopes(&raw);
    assert_eq!(envelopes.len(), 1);
    assert_eq!(envelopes[0][0][0].as_str(), Some("nb-only"));
}

#[test]
fn first_envelope_errors_when_response_is_empty() {
    let err = boq::first_envelope(")]}'\n").unwrap_err();
    match err {
        notebooklm::NotebookError::Parse(msg) => {
            assert!(msg.contains("no wrb.fr envelope"));
        },
        other => panic!("unexpected error: {other:?}"),
    }
}

#[test]
fn strip_xssi_is_a_no_op_on_clean_input() {
    assert_eq!(boq::strip_xssi("[[1]]"), "[[1]]");
    assert_eq!(boq::strip_xssi(")]}'\n[[1]]"), "[[1]]");
    assert_eq!(boq::strip_xssi(")]}'[[1]]"), "[[1]]");
}
