// SPDX-License-Identifier: Apache-2.0
//! Gemini web app (`gemini.google.com/app`) `batchexecute` RPC identifiers and
//! URL constants.
//!
//! Captured live from build `boq_assistant-bard-web-server_20260511.16_p20`
//! (2026-05-21) and cross-referenced with the HanaokaYuzu/Gemini-API
//! reverse-engineering reference (<https://github.com/HanaokaYuzu/Gemini-API>).

// ── RPC ids ──────────────────────────────────────────────────────────────────
/// Send a user message / generate a model turn (and, multiplexed, conversation
/// list polling). Inner payload framing: `[20, <json_string>, [0, null, 1]]`.
pub const SEND_MESSAGE: &str = "MaZiqc";
/// Config flag query, e.g. `bard_activity_enabled`. Inner: `[[["<flag>"]]]`.
pub const GET_CONFIG_FLAG: &str = "ESY5D";
/// Locale init. Inner: `[2, ["<lang>"], 0]`.
pub const INIT_LOCALE: &str = "CNgdBe";
/// UI preference read (positional flag array + setting key).
pub const GET_UI_PREF: &str = "L5adhe";
/// Incremental streamed-response reader, polled with
/// `source-path=/app/<conversation_id>` during generation.
pub const READ_STREAM: &str = "VxUbXb";
/// Conversation title generation (fired after the first turn of a new thread).
pub const GENERATE_TITLE: &str = "PCck7e";
/// Init / sync (load-time and post-send housekeeping).
pub const INIT_SYNC: &str = "aPya6c";

// ── URL constants ──────────────────────────────────────────────────────────
/// App landing page; its HTML embeds `WIZ_global_data` (the bootstrap tokens).
pub const URL_APP: &str = "https://gemini.google.com/app";
/// Origin sent on every `batchexecute` POST.
pub const URL_ORIGIN: &str = "https://gemini.google.com";
/// The single-RPC `batchexecute` endpoint.
pub const URL_BATCH_EXECUTE: &str = "https://gemini.google.com/_/BardChatUi/data/batchexecute";

// ── Model routing header ─────────────────────────────────────────────────────
/// HTTP header that selects the model (NOT the `f.req` body). Values are
/// opaque per-model tokens (`HanaokaYuzu` `constants.py`).
pub const MODEL_HEADER: &str = "x-goog-ext-525001261-jspb";

/// Opaque model-selector token for **Gemini Flash** (default).
pub const MODEL_FLASH: &str = "[1,null,null,null,\"fbb127bbb056c959\"]";
/// Opaque model-selector token for **Gemini Pro**.
pub const MODEL_PRO: &str = "[1,null,null,null,\"9d8ca3786ebdfbea\"]";
/// Opaque model-selector token for the **Thinking** model.
pub const MODEL_THINKING: &str = "[1,null,null,null,\"5bf011840784117a\"]";
