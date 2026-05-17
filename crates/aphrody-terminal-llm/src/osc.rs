// SPDX-License-Identifier: Apache-2.0
//! Parser for `aphrody-*` OSC escape sequences.
//!
//! Recognises:
//! - `\x1b]aphrody-md;<b64>\x07` (or `\x1b\\` ST terminator)
//! - `\x1b]aphrody-json;<b64>\x07`
//! - `\x1b]aphrody-sub-agent;<id>;<status>;<text>\x07`
//! - `\x1b]aphrody-mcp;<server>;<state>[;<rpc>]\x07`
//! - `\x1b]aphrody-hook;<b64-json>\x07`
//! - `\x1b]aphrody-skill;<b64-json>\x07`
//! - `\x1b]aphrody-task;<id>;<status>;<subject>\x07`
//!
//! All failures (invalid base64, invalid JSON, wrong field counts) return
//! `None` after logging via `tracing::warn!`.

use base64::{Engine as _, engine::general_purpose::STANDARD as B64};

use crate::LlmEvent;

/// Attempt to parse one OSC sequence carrying an `aphrody-*` payload.
///
/// `input` must be the raw bytes of the full sequence including the leading
/// `\x1b]` and the trailing `\x07` (BEL) or `\x1b\\` (ST). Returns `None` if
/// the sequence is not an aphrody LLM sequence or if any field is malformed.
pub fn parse_aphrody_llm_osc(input: &[u8]) -> Option<LlmEvent> {
    // Require ESC ] prefix.
    let body = input.strip_prefix(b"\x1b]")?;

    // Strip BEL or ST terminator.
    let body = if let Some(b) = body.strip_suffix(b"\x07") {
        b
    } else if let Some(b) = body.strip_suffix(b"\x1b\\") {
        b
    } else {
        return None;
    };

    // Require "aphrody-" prefix.
    let body = body.strip_prefix(b"aphrody-")?;

    // Split op from payload at the first ';'.
    let sep = body.iter().position(|&b| b == b';')?;
    let op = &body[..sep];
    let payload = &body[sep + 1..];

    match op {
        b"md" => {
            let text = decode_b64_utf8(payload, "aphrody-md")?;
            Some(LlmEvent::Markdown { body: text })
        }
        b"json" => {
            let raw = decode_b64_bytes(payload, "aphrody-json")?;
            let value: serde_json::Value = match serde_json::from_slice(&raw) {
                Ok(v) => v,
                Err(e) => {
                    tracing::warn!(op = "aphrody-json", error = %e, "invalid JSON payload");
                    return None;
                }
            };
            Some(LlmEvent::JsonInspect { payload: value })
        }
        b"sub-agent" => {
            // <id>;<status>;<text>  — split on ';', max 3 parts
            let parts = split_fields(payload, 3);
            if parts.len() < 3 {
                tracing::warn!(
                    op = "aphrody-sub-agent",
                    "expected 3 semicolon-separated fields, got {}",
                    parts.len()
                );
                return None;
            }
            Some(LlmEvent::SubAgent {
                id: parts[0].clone(),
                status: parts[1].clone(),
                text: parts[2].clone(),
            })
        }
        b"mcp" => {
            // <server>;<state>[;<rpc>]
            let parts = split_fields(payload, 3);
            if parts.len() < 2 {
                tracing::warn!(
                    op = "aphrody-mcp",
                    "expected at least 2 semicolon-separated fields, got {}",
                    parts.len()
                );
                return None;
            }
            let rpc = if parts.len() >= 3 && !parts[2].is_empty() {
                Some(parts[2].clone())
            } else {
                None
            };
            Some(LlmEvent::Mcp {
                server: parts[0].clone(),
                state: parts[1].clone(),
                rpc,
            })
        }
        b"hook" => {
            let raw = decode_b64_bytes(payload, "aphrody-hook")?;
            let mut map: serde_json::Map<String, serde_json::Value> =
                match serde_json::from_slice(&raw) {
                    Ok(m) => m,
                    Err(e) => {
                        tracing::warn!(op = "aphrody-hook", error = %e, "invalid JSON payload");
                        return None;
                    }
                };
            let event = match map.remove("event") {
                Some(serde_json::Value::String(s)) => s,
                _ => {
                    tracing::warn!(op = "aphrody-hook", "missing string field 'event'");
                    return None;
                }
            };
            let hook_payload = map
                .remove("payload")
                .unwrap_or(serde_json::Value::Object(Default::default()));
            Some(LlmEvent::Hook {
                event,
                payload: hook_payload,
            })
        }
        b"skill" => {
            let raw = decode_b64_bytes(payload, "aphrody-skill")?;
            let mut map: serde_json::Map<String, serde_json::Value> =
                match serde_json::from_slice(&raw) {
                    Ok(m) => m,
                    Err(e) => {
                        tracing::warn!(op = "aphrody-skill", error = %e, "invalid JSON payload");
                        return None;
                    }
                };
            let name = match map.remove("name") {
                Some(serde_json::Value::String(s)) => s,
                _ => {
                    tracing::warn!(op = "aphrody-skill", "missing string field 'name'");
                    return None;
                }
            };
            let phase = match map.remove("phase") {
                Some(serde_json::Value::String(s)) => s,
                _ => {
                    tracing::warn!(op = "aphrody-skill", "missing string field 'phase'");
                    return None;
                }
            };
            let skill_payload = map
                .remove("payload")
                .unwrap_or(serde_json::Value::Object(Default::default()));
            Some(LlmEvent::Skill {
                name,
                phase,
                payload: skill_payload,
            })
        }
        b"task" => {
            // <id>;<status>;<subject>
            let parts = split_fields(payload, 3);
            if parts.len() < 3 {
                tracing::warn!(
                    op = "aphrody-task",
                    "expected 3 semicolon-separated fields, got {}",
                    parts.len()
                );
                return None;
            }
            Some(LlmEvent::Task {
                id: parts[0].clone(),
                status: parts[1].clone(),
                subject: parts[2].clone(),
            })
        }
        other => {
            // Could be aphrody-browser-* or unknown: not our concern here.
            tracing::trace!(
                op = ?String::from_utf8_lossy(other),
                "unhandled aphrody OSC op in llm parser"
            );
            None
        }
    }
}

/// Decode base64-encoded bytes from a slice, logging on failure.
fn decode_b64_bytes(raw: &[u8], op: &str) -> Option<Vec<u8>> {
    match B64.decode(raw) {
        Ok(v) => Some(v),
        Err(e) => {
            tracing::warn!(op, error = %e, "invalid base64 in OSC payload");
            None
        }
    }
}

/// Decode base64-encoded UTF-8 from a slice, logging on failure.
fn decode_b64_utf8(raw: &[u8], op: &str) -> Option<String> {
    let bytes = decode_b64_bytes(raw, op)?;
    match String::from_utf8(bytes) {
        Ok(s) => Some(s),
        Err(e) => {
            tracing::warn!(op, error = %e, "OSC payload is not valid UTF-8 after base64 decode");
            None
        }
    }
}

/// Split `src` on `;`, yielding at most `max` parts as owned `String`s.
/// The last part absorbs any remaining semicolons.
fn split_fields(src: &[u8], max: usize) -> Vec<String> {
    let s = String::from_utf8_lossy(src);
    let mut out = Vec::with_capacity(max);
    let mut remaining = s.as_ref();
    for _ in 0..max - 1 {
        if remaining.is_empty() {
            break;
        }
        match remaining.find(';') {
            Some(pos) => {
                out.push(remaining[..pos].to_owned());
                remaining = &remaining[pos + 1..];
            }
            None => {
                out.push(remaining.to_owned());
                remaining = "";
                break;
            }
        }
    }
    if !remaining.is_empty() {
        out.push(remaining.to_owned());
    }
    out
}
