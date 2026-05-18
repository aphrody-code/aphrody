// SPDX-License-Identifier: Apache-2.0
//! Parser for `aphrody-browser-*` OSC sequences.
//!
//! ## Wire format
//!
//! ```text
//! ESC ]  aphrody-browser-<op> ; <payload>  BEL
//!  \x1b]  aphrody-browser-<op>;<payload>  \x07
//! ```
//!
//! ST (`\x1b\\`) is also a valid terminator.  The leading `ESC ]` is stripped
//! by the VT event bus before the byte slice reaches this module; we receive
//! only the OSC *content* (everything between `]` and the terminator).
//!
//! ## Op table
//!
//! | op           | payload                                   |
//! |---|---|
//! | `nav`        | raw URL                                   |
//! | `eval`       | base64-encoded JS source                  |
//! | `dom`        | base64-encoded CSS selector               |
//! | `screenshot` | `viewport` / `element:<sel>` / `fullpage` |
//! | `intercept`  | base64-encoded JSON rule                  |
//! | `extract`    | base64-encoded JSON schema                |
//! | `record`     | `<id>;<start>` or `<id>;<stop>`           |

use base64::{Engine as _, engine::general_purpose::STANDARD as B64};

use crate::proto::{BrowserRequest, RecordState, ScreenshotArea};

// ---------------------------------------------------------------------------
// Public entry point
// ---------------------------------------------------------------------------

/// Parse an `aphrody-browser-*` OSC content byte slice into a [`BrowserRequest`].
///
/// `input` must be the raw OSC *content* — i.e. the bytes between the `\x1b]`
/// introducer and the BEL/ST terminator.  The VT event bus is responsible for
/// stripping both the introducer and the terminator before calling this function.
///
/// Returns `None` if the content does not begin with `aphrody-browser-` or if
/// parsing fails.
pub fn parse_aphrody_browser_osc(input: &[u8]) -> Option<BrowserRequest> {
    // Strip optional ESC ] prefix + any leading BEL/ST that a caller might
    // include accidentally — be tolerant of input from the VT bus.
    let input = strip_osc_framing(input);

    const PREFIX: &[u8] = b"aphrody-browser-";
    let rest = input.strip_prefix(PREFIX)?;

    // Split on the first `;` to separate the op from the payload.
    let sep = rest.iter().position(|&b| b == b';')?;
    let op = &rest[..sep];
    let payload = &rest[sep + 1..];

    match op {
        b"nav" => parse_nav(payload),
        b"eval" => parse_eval(payload),
        b"dom" => parse_dom(payload),
        b"screenshot" => parse_screenshot(payload),
        b"intercept" => parse_intercept(payload),
        b"extract" => parse_extract(payload),
        b"record" => parse_record(payload),
        _ => None,
    }
}

// ---------------------------------------------------------------------------
// Internal helpers
// ---------------------------------------------------------------------------

/// Remove any OSC framing characters a caller may have included.
fn strip_osc_framing(input: &[u8]) -> &[u8] {
    // Strip leading ESC ]
    let input = if input.starts_with(b"\x1b]") { &input[2..] } else { input };
    // Strip trailing BEL or ST
    if input.ends_with(b"\x07") {
        &input[..input.len() - 1]
    } else if input.ends_with(b"\x1b\\") {
        &input[..input.len() - 2]
    } else {
        input
    }
}

/// Decode base64 payload to a UTF-8 string.  Returns `None` on any error.
fn b64_to_string(payload: &[u8]) -> Option<String> {
    let decoded = B64.decode(payload).ok()?;
    String::from_utf8(decoded).ok()
}

/// Decode base64 payload to a `serde_json::Value`.  Returns `None` on error.
fn b64_to_json(payload: &[u8]) -> Option<serde_json::Value> {
    let text = b64_to_string(payload)?;
    serde_json::from_str(&text).ok()
}

// ---------------------------------------------------------------------------
// Per-op parsers
// ---------------------------------------------------------------------------

fn parse_nav(payload: &[u8]) -> Option<BrowserRequest> {
    let url = String::from_utf8(payload.to_vec()).ok()?;
    if url.is_empty() {
        return None;
    }
    Some(BrowserRequest::Nav { url })
}

fn parse_eval(payload: &[u8]) -> Option<BrowserRequest> {
    let src = b64_to_string(payload)?;
    Some(BrowserRequest::Eval { src })
}

fn parse_dom(payload: &[u8]) -> Option<BrowserRequest> {
    let selector = b64_to_string(payload)?;
    Some(BrowserRequest::Dom { selector })
}

fn parse_screenshot(payload: &[u8]) -> Option<BrowserRequest> {
    let text = std::str::from_utf8(payload).ok()?;
    let area = if text == "viewport" {
        ScreenshotArea::Viewport
    } else if text == "fullpage" {
        ScreenshotArea::Fullpage
    } else if let Some(sel) = text.strip_prefix("element:") {
        if sel.is_empty() {
            return None;
        }
        ScreenshotArea::Element { selector: sel.to_owned() }
    } else {
        return None;
    };
    Some(BrowserRequest::Screenshot { area })
}

fn parse_intercept(payload: &[u8]) -> Option<BrowserRequest> {
    let rule = b64_to_json(payload)?;
    Some(BrowserRequest::Intercept { rule })
}

fn parse_extract(payload: &[u8]) -> Option<BrowserRequest> {
    let schema = b64_to_json(payload)?;
    Some(BrowserRequest::Extract { schema })
}

fn parse_record(payload: &[u8]) -> Option<BrowserRequest> {
    // payload = `<id>;<start|stop>`
    let text = std::str::from_utf8(payload).ok()?;
    let sep = text.find(';')?;
    let id = text[..sep].to_owned();
    let state_str = &text[sep + 1..];
    let state = match state_str {
        "start" => RecordState::Start,
        "stop" => RecordState::Stop,
        _ => return None,
    };
    Some(BrowserRequest::Record { id, state })
}

// ---------------------------------------------------------------------------
// Unit tests (pure, no I/O)
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nav_plain_url() {
        let seq = b"aphrody-browser-nav;https://example.com";
        let req = parse_aphrody_browser_osc(seq).unwrap();
        assert!(matches!(req, BrowserRequest::Nav { url } if url == "https://example.com"));
    }

    #[test]
    fn screenshot_viewport() {
        let seq = b"aphrody-browser-screenshot;viewport";
        let req = parse_aphrody_browser_osc(seq).unwrap();
        assert!(matches!(req, BrowserRequest::Screenshot { area: ScreenshotArea::Viewport }));
    }

    #[test]
    fn screenshot_element() {
        let seq = b"aphrody-browser-screenshot;element:#main";
        let req = parse_aphrody_browser_osc(seq).unwrap();
        assert!(
            matches!(req, BrowserRequest::Screenshot { area: ScreenshotArea::Element { ref selector } } if selector == "#main")
        );
    }

    #[test]
    fn unknown_op_returns_none() {
        let seq = b"aphrody-browser-bogus;payload";
        assert!(parse_aphrody_browser_osc(seq).is_none());
    }

    #[test]
    fn wrong_prefix_returns_none() {
        let seq = b"aphrody-md;SGVsbG8=";
        assert!(parse_aphrody_browser_osc(seq).is_none());
    }
}
