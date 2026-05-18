// SPDX-License-Identifier: Apache-2.0
//
// OSC envelope codec — direct port of `src/osc.ts`. Frames follow the
// terminal-spec layout:
//
//   ESC ] aphrody-jsx-<opcode> ; <fields...> BEL
//
// where ESC = 0x1b and BEL = 0x07. Field payloads are base64-encoded JSON
// (or raw decimal strings for `window-size`). The codec is symmetric:
// `encode_frame` produces a string ready to write to stdout, and
// `decode_frame` recovers the opcode + fields from a single envelope.

//! `aphrody-jsx-*` OSC frame codec.

use alloc::string::{String, ToString};
use alloc::vec::Vec;

use base64::Engine as _;
use base64::engine::general_purpose::STANDARD as B64;
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

use crate::element::{JsonNode, JsonPatchOp};

const ESC: char = '\x1b';
const BEL: char = '\x07';
const PREFIX: &str = "\x1b]aphrody-jsx-";

/// Opcode tag — mirrors `OscOpcode` in `types.ts`.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum OscOpcode {
    /// First commit: ship the full subtree.
    Mount,
    /// Patch list for subsequent commits.
    Update,
    /// Root teardown.
    Unmount,
    /// Input event from the terminal back to the reconciler.
    Input,
    /// Window resize event.
    WindowSize,
    /// Focus change event.
    Focus,
}

impl OscOpcode {
    const fn as_str(self) -> &'static str {
        match self {
            Self::Mount => "mount",
            Self::Update => "update",
            Self::Unmount => "unmount",
            Self::Input => "input",
            Self::WindowSize => "window-size",
            Self::Focus => "focus",
        }
    }

    fn from_str(s: &str) -> Option<Self> {
        Some(match s {
            "mount" => Self::Mount,
            "update" => Self::Update,
            "unmount" => Self::Unmount,
            "input" => Self::Input,
            "window-size" => Self::WindowSize,
            "focus" => Self::Focus,
            _ => return None,
        })
    }
}

/// `mount` payload.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct MountFields {
    /// Root id.
    pub id: String,
    /// Initial subtree.
    pub tree: JsonNode,
}

/// `update` payload.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct UpdateFields {
    /// Root id.
    pub id: String,
    /// Ordered patch list.
    pub patch: Vec<JsonPatchOp>,
}

/// `unmount` payload.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct UnmountFields {
    /// Root id.
    pub id: String,
}

/// `input` payload.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct InputFields {
    /// Focused node id.
    pub id: String,
    /// Literal input string.
    pub input: String,
    /// Active key modifiers.
    pub key: Map<String, Value>,
}

/// `window-size` payload.
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq)]
pub struct WindowSizeFields {
    /// Width in columns.
    pub columns: u32,
    /// Height in rows.
    pub rows: u32,
}

/// `focus` payload.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct FocusFields {
    /// Node id.
    pub id: String,
    /// New focus state.
    pub focused: bool,
}

/// Typed OSC frame — mirrors `OscFrame` union in `osc.ts`.
#[derive(Clone, Debug, PartialEq)]
pub enum OscFrame {
    /// `mount` envelope.
    Mount(MountFields),
    /// `update` envelope.
    Update(UpdateFields),
    /// `unmount` envelope.
    Unmount(UnmountFields),
    /// `input` envelope.
    Input(InputFields),
    /// `window-size` envelope.
    WindowSize(WindowSizeFields),
    /// `focus` envelope.
    Focus(FocusFields),
}

impl OscFrame {
    /// Returns the opcode for this frame.
    #[must_use]
    pub const fn opcode(&self) -> OscOpcode {
        match self {
            Self::Mount(_) => OscOpcode::Mount,
            Self::Update(_) => OscOpcode::Update,
            Self::Unmount(_) => OscOpcode::Unmount,
            Self::Input(_) => OscOpcode::Input,
            Self::WindowSize(_) => OscOpcode::WindowSize,
            Self::Focus(_) => OscOpcode::Focus,
        }
    }
}

/// Encodes an OSC frame to its terminal-wire string form. Mirrors `encode()`.
#[must_use]
pub fn encode_frame(frame: &OscFrame) -> String {
    let opcode = frame.opcode().as_str();
    match frame {
        OscFrame::Mount(MountFields { id, tree }) => {
            let payload = B64.encode(serde_json::to_string(tree).unwrap_or_default());
            alloc::format!("{PREFIX}{opcode};{id};{payload}{BEL}")
        }
        OscFrame::Update(UpdateFields { id, patch }) => {
            let payload = B64.encode(serde_json::to_string(patch).unwrap_or_default());
            alloc::format!("{PREFIX}{opcode};{id};{payload}{BEL}")
        }
        OscFrame::Unmount(UnmountFields { id }) => {
            alloc::format!("{PREFIX}{opcode};{id}{BEL}")
        }
        OscFrame::Input(InputFields { id, input, key }) => {
            let mut body = Map::with_capacity(2);
            body.insert("input".to_string(), Value::String(input.clone()));
            body.insert("key".to_string(), Value::Object(key.clone()));
            let payload = B64.encode(serde_json::to_string(&Value::Object(body)).unwrap_or_default());
            alloc::format!("{PREFIX}{opcode};{id};{payload}{BEL}")
        }
        OscFrame::WindowSize(WindowSizeFields { columns, rows }) => {
            alloc::format!("{PREFIX}{opcode};{columns};{rows}{BEL}")
        }
        OscFrame::Focus(FocusFields { id, focused }) => {
            let flag = if *focused { "true" } else { "false" };
            alloc::format!("{PREFIX}{opcode};{id};{flag}{BEL}")
        }
    }
}

/// Decodes a single envelope. Returns `None` on malformed input. Mirrors
/// `parseFrame()` from `osc.ts`.
#[must_use]
pub fn decode_frame(envelope: &str) -> Option<OscFrame> {
    if !envelope.starts_with(PREFIX) || !envelope.ends_with(BEL) {
        return None;
    }
    let body = &envelope[PREFIX.len()..envelope.len() - BEL.len_utf8()];
    let (op_str, rest) = match body.find(';') {
        Some(idx) => (&body[..idx], &body[idx + 1..]),
        None => (body, ""),
    };
    let opcode = OscOpcode::from_str(op_str)?;
    match opcode {
        OscOpcode::Mount => {
            let (id, payload) = split_once(rest, ';')?;
            let json = B64.decode(payload).ok()?;
            let json_str = core::str::from_utf8(&json).ok()?;
            let tree: JsonNode = serde_json::from_str(json_str).ok()?;
            Some(OscFrame::Mount(MountFields {
                id: id.to_string(),
                tree,
            }))
        }
        OscOpcode::Update => {
            let (id, payload) = split_once(rest, ';')?;
            let json = B64.decode(payload).ok()?;
            let json_str = core::str::from_utf8(&json).ok()?;
            let patch: Vec<JsonPatchOp> = serde_json::from_str(json_str).ok()?;
            Some(OscFrame::Update(UpdateFields {
                id: id.to_string(),
                patch,
            }))
        }
        OscOpcode::Unmount => {
            if rest.is_empty() {
                return None;
            }
            Some(OscFrame::Unmount(UnmountFields {
                id: rest.to_string(),
            }))
        }
        OscOpcode::Input => {
            let (id, payload) = split_once(rest, ';')?;
            let json = B64.decode(payload).ok()?;
            let json_str = core::str::from_utf8(&json).ok()?;
            let decoded: serde_json::Value = serde_json::from_str(json_str).ok()?;
            let input = decoded.get("input")?.as_str()?.to_string();
            let key = decoded.get("key")?.as_object()?.clone();
            Some(OscFrame::Input(InputFields {
                id: id.to_string(),
                input,
                key,
            }))
        }
        OscOpcode::WindowSize => {
            let (cols_str, rows_str) = split_once(rest, ';')?;
            let columns: u32 = cols_str.parse().ok()?;
            let rows: u32 = rows_str.parse().ok()?;
            Some(OscFrame::WindowSize(WindowSizeFields { columns, rows }))
        }
        OscOpcode::Focus => {
            let (id, flag) = split_once(rest, ';')?;
            Some(OscFrame::Focus(FocusFields {
                id: id.to_string(),
                focused: flag == "true",
            }))
        }
    }
}

fn split_once(value: &str, sep: char) -> Option<(&str, &str)> {
    let idx = value.find(sep)?;
    Some((&value[..idx], &value[idx + sep.len_utf8()..]))
}

/// Streaming parser — accumulates arbitrary chunks and yields complete
/// frames as they become available. Direct port of `OscFrameParser`.
#[derive(Debug, Default)]
pub struct OscFrameParser {
    buffer: String,
}

impl OscFrameParser {
    /// Constructs a new empty parser.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            buffer: String::new(),
        }
    }

    /// Feeds a chunk and returns any frames that completed.
    pub fn push(&mut self, chunk: &str) -> Vec<OscFrame> {
        self.buffer.push_str(chunk);
        let mut out = Vec::new();
        loop {
            let Some(start) = self.buffer.find(PREFIX) else {
                if self.buffer.len() > PREFIX.len() {
                    let split = self.buffer.len() - PREFIX.len();
                    let leftover: String = self.buffer[split..].to_string();
                    self.buffer = leftover;
                }
                break;
            };
            let search_start = start + PREFIX.len();
            let Some(rel_end) = self.buffer[search_start..].find(BEL) else {
                // No terminator yet — drop leading garbage and wait.
                let leftover: String = self.buffer[start..].to_string();
                self.buffer = leftover;
                break;
            };
            let end = search_start + rel_end;
            let envelope: String = self.buffer[start..=end].to_string();
            let leftover: String = self.buffer[end + BEL.len_utf8()..].to_string();
            self.buffer = leftover;
            if let Some(frame) = decode_frame(&envelope) {
                out.push(frame);
            }
        }
        out
    }
}

/// Sink abstraction. Anything that can absorb `&str` writes works.
/// Mirrors `OscSink` from `osc.ts`.
pub trait OscSink {
    /// Writes a string chunk.
    fn write_str(&mut self, chunk: &str);
}

impl OscSink for String {
    fn write_str(&mut self, chunk: &str) {
        self.push_str(chunk);
    }
}

/// In-memory `Vec<String>` sink — preserves write boundaries (matches the
/// TS test helper `captureSink()`).
#[derive(Clone, Debug, Default)]
pub struct VecSink {
    /// Recorded write chunks (in order).
    pub chunks: Vec<String>,
}

impl VecSink {
    /// Constructs a fresh empty sink.
    #[must_use]
    pub const fn new() -> Self {
        Self { chunks: Vec::new() }
    }

    /// Concatenates all chunks into a single string.
    #[must_use]
    pub fn joined(&self) -> String {
        self.chunks.concat()
    }
}

impl OscSink for VecSink {
    fn write_str(&mut self, chunk: &str) {
        self.chunks.push(chunk.to_string());
    }
}

/// Convenience helper: encode `frame` and write it to `sink`.
pub fn emit(sink: &mut dyn OscSink, frame: &OscFrame) {
    sink.write_str(&encode_frame(frame));
}

// Make sure ESC / BEL stay reachable from other modules' doc tests.
#[doc(hidden)]
pub const _ESC: char = ESC;
