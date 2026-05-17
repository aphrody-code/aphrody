// SPDX-License-Identifier: Apache-2.0
//! AG-UI canonical wire-format types.
//!
//! These Serde structs exactly mirror the JSON shape produced by the TS
//! `agui-adapter` package (`packages/agui-adapter/src/types.ts`).  The
//! discriminant field is `kind`; each variant is represented as a tagged
//! union using `#[serde(tag = "kind", rename_all = "camelCase")]` so that
//! `serde_json::from_str` resolves the right variant from the stream token
//! `{"kind":"agent.message", ...}`.
//!
//! ## Event shape overview
//!
//! Every event carries the common envelope fields from `AGUIEventBase`:
//!
//! | field   | type   | required |
//! |---------|--------|----------|
//! | `kind`  | string | yes      |
//! | `runId` | string | yes      |
//! | `seq`   | u64    | no       |
//! | `ts`    | u64    | yes      |
//!
//! The `kind` values and their per-variant extra fields are documented on
//! each variant below.  All string fields that are optional in the spec are
//! represented as `Option<String>` so a missing key and a `null` value both
//! round-trip cleanly.

use serde::{Deserialize, Serialize};

// ---------------------------------------------------------------------------
// Shared envelope
// ---------------------------------------------------------------------------

/// Fields present on every AG-UI event.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EventBase {
    /// The OD run this event belongs to.
    pub run_id: String,
    /// Optional monotonic per-run sequence number.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub seq: Option<u64>,
    /// Wall-clock timestamp (unix ms) when the event was emitted.
    pub ts: u64,
}

// ---------------------------------------------------------------------------
// Per-variant payload extras
// ---------------------------------------------------------------------------

/// Payload for `agent.message` events.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentMessagePayload {
    /// Streaming chunk text. Concatenate consecutive chunks to rebuild the
    /// assistant turn.
    pub text: String,
    /// `true` on the final chunk of a streaming turn.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub done: Option<bool>,
}

/// Payload for `tool_call` events.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ToolCallPayload {
    pub tool_name: String,
    /// Pre-validated arguments the agent passed; any JSON value.
    pub args: serde_json::Value,
    /// Optional id correlating start + result events for the same call.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub call_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<ToolCallStatus>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<serde_json::Value>,
}

/// Status transitions for a single tool invocation.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ToolCallStatus {
    Started,
    Completed,
    Failed,
}

/// Payload for `state_update` events.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StateUpdatePayload {
    /// Dot-segmented key path into the run-state cache. Empty = replace all.
    pub path: String,
    pub value: serde_json::Value,
}

/// Payload for `ui.surface_requested` events.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SurfaceRequestedPayload {
    pub surface_id: String,
    pub surface_kind: SurfaceKind,
    pub payload: serde_json::Value,
}

/// The four declarative surface kinds AG-UI v1 supports.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum SurfaceKind {
    Form,
    Choice,
    Confirmation,
    OauthPrompt,
}

/// Payload for `ui.surface_responded` events.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SurfaceRespondedPayload {
    pub surface_id: String,
    pub value: serde_json::Value,
    pub responded_by: RespondedBy,
}

/// Who (or what) resolved a surface request.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum RespondedBy {
    User,
    Agent,
    Auto,
    Cache,
}

/// Payload for `run.lifecycle` events.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RunLifecyclePayload {
    pub status: RunStatus,
    /// Stage id when status starts with `pipeline_stage_`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stage_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub iteration: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
}

/// Run-lifecycle status values.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RunStatus {
    Started,
    PipelineStageStarted,
    PipelineStageCompleted,
    Completed,
    Cancelled,
    Failed,
}

// ---------------------------------------------------------------------------
// Top-level discriminated union — the canonical wire type
// ---------------------------------------------------------------------------

/// Canonical AG-UI event as it appears on the wire (JSONL stream).
///
/// The `kind` field acts as the discriminant; serde resolves to the matching
/// variant from the adjacently-tagged JSON object.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum WireEvent {
    /// `"kind": "agent.message"` — streaming text chunk from the agent.
    #[serde(rename = "agent.message")]
    AgentMessage {
        #[serde(flatten)]
        base: EventBase,
        #[serde(flatten)]
        payload: AgentMessagePayload,
    },
    /// `"kind": "tool_call"` — tool invocation (start or result).
    #[serde(rename = "tool_call")]
    ToolCall {
        #[serde(flatten)]
        base: EventBase,
        #[serde(flatten)]
        payload: ToolCallPayload,
    },
    /// `"kind": "state_update"` — delta to the run-state cache.
    #[serde(rename = "state_update")]
    StateUpdate {
        #[serde(flatten)]
        base: EventBase,
        #[serde(flatten)]
        payload: StateUpdatePayload,
    },
    /// `"kind": "ui.surface_requested"` — agent raised a declarative surface.
    #[serde(rename = "ui.surface_requested")]
    UiSurfaceRequested {
        #[serde(flatten)]
        base: EventBase,
        #[serde(flatten)]
        payload: SurfaceRequestedPayload,
    },
    /// `"kind": "ui.surface_responded"` — surface resolved (user / agent / auto / cache).
    #[serde(rename = "ui.surface_responded")]
    UiSurfaceResponded {
        #[serde(flatten)]
        base: EventBase,
        #[serde(flatten)]
        payload: SurfaceRespondedPayload,
    },
    /// `"kind": "run.lifecycle"` — run-level status transition.
    #[serde(rename = "run.lifecycle")]
    RunLifecycle {
        #[serde(flatten)]
        base: EventBase,
        #[serde(flatten)]
        payload: RunLifecyclePayload,
    },
}

impl WireEvent {
    /// Returns the `runId` field regardless of variant.
    pub fn run_id(&self) -> &str {
        match self {
            Self::AgentMessage { base, .. }
            | Self::ToolCall { base, .. }
            | Self::StateUpdate { base, .. }
            | Self::UiSurfaceRequested { base, .. }
            | Self::UiSurfaceResponded { base, .. }
            | Self::RunLifecycle { base, .. } => &base.run_id,
        }
    }

    /// Returns the wall-clock timestamp.
    pub fn ts(&self) -> u64 {
        match self {
            Self::AgentMessage { base, .. }
            | Self::ToolCall { base, .. }
            | Self::StateUpdate { base, .. }
            | Self::UiSurfaceRequested { base, .. }
            | Self::UiSurfaceResponded { base, .. }
            | Self::RunLifecycle { base, .. } => base.ts,
        }
    }
}

// ---------------------------------------------------------------------------
// AguiNode wire shapes (the declarative UI tree embedded in surface payloads)
// ---------------------------------------------------------------------------

/// JSON shape for a `Button` node inside a surface payload.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WireButton {
    pub label: String,
    #[serde(default)]
    pub variant: String,
    #[serde(default)]
    pub disabled: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub on_click_id: Option<String>,
}

/// JSON shape for an `Input` node.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WireInput {
    #[serde(default)]
    pub label: String,
    #[serde(default)]
    pub value: String,
    #[serde(default)]
    pub placeholder: String,
    #[serde(default)]
    pub disabled: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub on_input_id: Option<String>,
}

/// JSON shape for a `ToolCall` node (rendered progress / result card).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WireToolCall {
    pub tool_name: String,
    pub args: serde_json::Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<ToolCallStatus>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<serde_json::Value>,
}

/// JSON shape for an `Image` node.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WireImage {
    pub src: String,
    #[serde(default)]
    pub alt: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub width_px: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub height_px: Option<u32>,
}

/// JSON shape for a `Code` node.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WireCode {
    pub code: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub lang: Option<String>,
}

/// Top-level discriminated union for an AguiNode as it appears in JSON.
///
/// The `type` field is the discriminant (`"text"`, `"container"`, `"button"`,
/// etc.). Children of `Container` are a `Vec<WireNode>` so the tree is
/// arbitrarily deep.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum WireNode {
    Text {
        text: String,
    },
    Container {
        children: Vec<WireNode>,
    },
    Button {
        #[serde(flatten)]
        props: WireButton,
    },
    Input {
        #[serde(flatten)]
        props: WireInput,
    },
    ToolCall {
        #[serde(flatten)]
        props: WireToolCall,
    },
    Markdown {
        text: String,
    },
    Image {
        #[serde(flatten)]
        props: WireImage,
    },
    Code {
        #[serde(flatten)]
        props: WireCode,
    },
}
