// SPDX-License-Identifier: Apache-2.0
//! aphrody-context — LLM context-window manager.
//!
//! This crate owns the *bookkeeping* side of an LLM conversation: which turns
//! sit in the rolling prompt budget, how many tokens they consume, when the
//! window grows past its threshold and the orchestrator should summarize, and
//! how to snapshot/restore the entire state for crash recovery or session
//! handoff.
//!
//! It deliberately stays orthogonal to the LLM provider layer: there is no
//! HTTP client, no tokenizer dependency, no model registry. The
//! [`TokenEstimator`] trait is the only seam, and the bundled
//! [`HeuristicTokenEstimator`] is a small `chars / divisor` approximation that
//! is good enough for budgeting on the hot path — callers wanting BPE-grade
//! precision can plug in their own estimator without pulling extra weight
//! into this crate.
//!
//! ## Cohesion with the wider aphrody workspace
//!
//! * **`aphrody-memory::types`** supplies the `MessageRole` /
//!   `MemoryRecord` shape this crate echoes for terminology cohesion (User,
//!   Assistant, Tool, System).
//! * **`aphrody-cron::Scheduler`** inspired the `Vec<…>` + iteration pattern
//!   for the message store plus the `next_after`-style trim cursor.
//! * **`aphrody-marketplace::MarketplaceIndex`** is the model for
//!   `find_by_id` / `entries` / thiserror enum shapes used by
//!   [`ContextWindow::pin`] and [`ContextError`].
//!
//! ## Pinned messages
//!
//! A message marked `pinned` is exempt from eviction by
//! [`ContextWindow::trim_to_fit`]. Use this for the system prompt, tool
//! schemas, or any persistent assistant context that must survive the
//! sliding-window rotation.
//!
//! ## Summarization trigger
//!
//! When [`ContextWindow::utilization`] crosses
//! `summary_threshold` (default `0.85`), [`ContextWindow::should_summarize`]
//! returns `true`. The caller then drains the oldest non-pinned messages via
//! [`ContextWindow::trim_to_fit`] and ships them as a
//! [`SummaryRequest`] to an external LLM, replacing them with a single
//! summary turn at the head of the window.
//!
//! See `tests/context_smoke.rs` for the executable contract.

#![forbid(unsafe_code)]

use std::sync::Arc;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use thiserror::Error;

// ---------------------------------------------------------------------------
// MessageRole — mirrored from aphrody-memory::types for naming cohesion.
// ---------------------------------------------------------------------------

/// Role of a conversational turn inside the context window.
///
/// The four variants line up with the OpenAI-style chat completion contract
/// (`system`, `user`, `assistant`, `tool`) which Anthropic, Gemini, and the
/// Antigravity gateway all consume in canonicalised form.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MessageRole {
    /// Persistent system prompt — almost always pinned.
    System,
    /// Human / upstream caller turn.
    User,
    /// Model-generated turn.
    Assistant,
    /// Tool / function result returned to the model.
    Tool,
}

impl MessageRole {
    /// Stable lowercase identifier suitable for logs and JSON labelling.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::System => "system",
            Self::User => "user",
            Self::Assistant => "assistant",
            Self::Tool => "tool",
        }
    }
}

// ---------------------------------------------------------------------------
// ContextMessage — one entry in the window.
// ---------------------------------------------------------------------------

/// A single conversational turn tracked by the context window.
///
/// `token_estimate` is *cached*: it is computed at insertion time via the
/// active [`TokenEstimator`] and never recomputed. This keeps
/// [`ContextWindow::total_tokens`] O(n) over message count, not over total
/// character length.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ContextMessage {
    /// Stable identifier — caller-assigned or hash-derived via
    /// [`ContextMessage::hash_id`].
    pub id: String,
    /// Role of this turn (system / user / assistant / tool).
    pub role: MessageRole,
    /// Raw textual content. Tool results should be serialized to JSON before
    /// being placed here so the estimator sees their full footprint.
    pub content: String,
    /// Wall-clock creation timestamp (UTC).
    pub ts: DateTime<Utc>,
    /// Whether eviction is forbidden for this message. System prompts and
    /// tool schemas typically set this to `true`.
    #[serde(default)]
    pub pinned: bool,
    /// Pre-computed token estimate. Set by [`ContextWindow::push`]; callers
    /// rarely populate it directly.
    pub token_estimate: u32,
}

impl ContextMessage {
    /// Build a fresh message. `token_estimate` defaults to zero — it is
    /// overwritten by [`ContextWindow::push`].
    #[must_use]
    pub fn new(id: impl Into<String>, role: MessageRole, content: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            role,
            content: content.into(),
            ts: Utc::now(),
            pinned: false,
            token_estimate: 0,
        }
    }

    /// Builder helper — mark this message as pinned.
    #[must_use]
    pub fn pinned(mut self) -> Self {
        self.pinned = true;
        self
    }

    /// Deterministic, dependency-free id derived from the message content
    /// and role. Uses the FNV-1a 64-bit hash so the same content always
    /// yields the same id without pulling in `uuid` or `sha2`.
    #[must_use]
    pub fn hash_id(role: MessageRole, content: &str) -> String {
        const FNV_OFFSET: u64 = 0xcbf2_9ce4_8422_2325;
        const FNV_PRIME: u64 = 0x0000_0100_0000_01b3;
        let mut hash = FNV_OFFSET;
        for &b in role.as_str().as_bytes() {
            hash ^= u64::from(b);
            hash = hash.wrapping_mul(FNV_PRIME);
        }
        // Domain-separator between role and content so "user" + "tool" can't
        // collide with "us" + "ertool".
        hash ^= 0xff;
        hash = hash.wrapping_mul(FNV_PRIME);
        for &b in content.as_bytes() {
            hash ^= u64::from(b);
            hash = hash.wrapping_mul(FNV_PRIME);
        }
        format!("msg-{hash:016x}")
    }
}

// ---------------------------------------------------------------------------
// TokenEstimator — pluggable counting strategy.
// ---------------------------------------------------------------------------

/// Counts tokens for a given piece of text.
///
/// Implementations must be cheap to call — [`ContextWindow::push`] invokes
/// this on every insertion. The reference [`HeuristicTokenEstimator`] runs
/// in linear time over the input bytes with no allocation.
pub trait TokenEstimator: Send + Sync {
    /// Return the estimated token count for `text`. Implementations are
    /// permitted to over-estimate (safer for budget planning) but should
    /// never return zero for non-empty input.
    fn estimate(&self, text: &str) -> u32;
}

/// Linear `bytes / divisor` heuristic, deliberately dependency-free.
///
/// The default divisor of `4` matches the widely-cited "≈4 chars per token"
/// rule of thumb for English-heavy GPT/Claude-class BPE vocabularies. The
/// constructor enforces a minimum divisor of `1` to avoid division by zero
/// and clamps oversized inputs to `u32::MAX`.
#[derive(Debug, Clone, Copy)]
pub struct HeuristicTokenEstimator {
    chars_per_token: u32,
}

impl HeuristicTokenEstimator {
    /// Build a new estimator. `chars_per_token` is clamped to at least `1`.
    #[must_use]
    pub const fn new(chars_per_token: u32) -> Self {
        Self {
            chars_per_token: if chars_per_token == 0 { 1 } else { chars_per_token },
        }
    }

    /// Default `4 chars / token` estimator.
    #[must_use]
    pub const fn default_factor() -> Self {
        Self::new(4)
    }
}

impl Default for HeuristicTokenEstimator {
    fn default() -> Self {
        Self::default_factor()
    }
}

impl TokenEstimator for HeuristicTokenEstimator {
    fn estimate(&self, text: &str) -> u32 {
        if text.is_empty() {
            return 0;
        }
        // `chars().count()` is more accurate than `len()` for non-ASCII
        // content (CJK, emoji) since it counts code points instead of bytes.
        let chars = text.chars().count();
        // Ceiling division so a 1-character input still costs 1 token.
        let divisor = self.chars_per_token as usize;
        let estimate = chars.div_ceil(divisor);
        u32::try_from(estimate).unwrap_or(u32::MAX).max(1)
    }
}

// ---------------------------------------------------------------------------
// ContextSnapshot — serde-round-trippable persistence shape.
// ---------------------------------------------------------------------------

/// Serializable point-in-time snapshot of a [`ContextWindow`].
///
/// The estimator implementation is *not* serialized: snapshots are
/// deliberately portable across estimator choices. On
/// [`ContextWindow::restore`] the caller supplies a fresh estimator and the
/// cached `token_estimate` per message is preserved as-is.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ContextSnapshot {
    /// All messages currently in the window, oldest first.
    pub messages: Vec<ContextMessage>,
    /// Per-window token budget.
    pub max_tokens: u32,
    /// Utilization ratio at which [`ContextWindow::should_summarize`] flips.
    pub summary_threshold: f32,
    /// Wall-clock time the snapshot was taken.
    pub created_at: DateTime<Utc>,
}

// ---------------------------------------------------------------------------
// SummaryRequest — payload for an external summarizer.
// ---------------------------------------------------------------------------

/// Bundle of messages the orchestrator should compress into a single
/// summary turn before re-inserting at the head of the window.
///
/// The crate produces this struct but does *not* execute the summarization
/// — that is the LLM router's job (see `aphrody-router`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SummaryRequest {
    /// Messages selected for compression. Order is oldest-first.
    pub messages_to_summarize: Vec<ContextMessage>,
    /// Token ceiling the summary itself must respect once re-inserted.
    pub target_token_budget: u32,
}

// ---------------------------------------------------------------------------
// ContextError — failure surface.
// ---------------------------------------------------------------------------

/// All recoverable failures surfaced by [`ContextWindow`] operations.
#[derive(Debug, Error)]
pub enum ContextError {
    /// JSON (de)serialization failed for snapshot import/export.
    #[error("json error: {0}")]
    Json(#[from] serde_json::Error),
    /// A message id was requested but no entry matches.
    #[error("message not found: {0}")]
    MessageNotFound(String),
    /// `summary_threshold` was set outside the `0.0..=1.0` range.
    #[error("invalid summary_threshold: {0} (must be in 0.0..=1.0)")]
    InvalidThreshold(f32),
    /// An operation requires at least one message but the window is empty.
    #[error("context window is empty")]
    EmptyWindow,
}

// ---------------------------------------------------------------------------
// ContextWindow — the rolling LLM prompt budget.
// ---------------------------------------------------------------------------

/// Default fraction of `max_tokens` that triggers summarization.
pub const DEFAULT_SUMMARY_THRESHOLD: f32 = 0.85;

/// Rolling LLM context window with pinned-message preservation, sliding
/// eviction, and snapshot/restore.
///
/// `ContextWindow` is **not** thread-safe on its own — wrap it in
/// `tokio::sync::Mutex` for concurrent producers/consumers. The estimator
/// is held behind an [`Arc`] so cheap clones share the same instance.
pub struct ContextWindow {
    messages: Vec<ContextMessage>,
    max_tokens: u32,
    estimator: Arc<dyn TokenEstimator>,
    summary_threshold: f32,
}

impl std::fmt::Debug for ContextWindow {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ContextWindow")
            .field("messages_len", &self.messages.len())
            .field("max_tokens", &self.max_tokens)
            .field("summary_threshold", &self.summary_threshold)
            .field("total_tokens", &self.total_tokens())
            .finish()
    }
}

impl ContextWindow {
    /// Build a new window with the default summary threshold.
    #[must_use]
    pub fn new(max_tokens: u32, estimator: Arc<dyn TokenEstimator>) -> Self {
        Self {
            messages: Vec::new(),
            max_tokens: max_tokens.max(1),
            estimator,
            summary_threshold: DEFAULT_SUMMARY_THRESHOLD,
        }
    }

    /// Build a window with a caller-chosen summary threshold.
    ///
    /// # Errors
    ///
    /// Returns [`ContextError::InvalidThreshold`] when `summary_threshold`
    /// is not in `0.0..=1.0` or is NaN.
    pub fn with_threshold(
        max_tokens: u32,
        estimator: Arc<dyn TokenEstimator>,
        summary_threshold: f32,
    ) -> Result<Self, ContextError> {
        Self::validate_threshold(summary_threshold)?;
        Ok(Self {
            messages: Vec::new(),
            max_tokens: max_tokens.max(1),
            estimator,
            summary_threshold,
        })
    }

    fn validate_threshold(t: f32) -> Result<(), ContextError> {
        if t.is_nan() || !(0.0..=1.0).contains(&t) {
            return Err(ContextError::InvalidThreshold(t));
        }
        Ok(())
    }

    /// Insert a message, re-estimating its `token_estimate`.
    ///
    /// The caller's `token_estimate` field is overwritten — pass `0` (the
    /// default from [`ContextMessage::new`]) and let this method do the
    /// counting via the bound [`TokenEstimator`].
    pub fn push(&mut self, mut message: ContextMessage) {
        message.token_estimate = self.estimator.estimate(&message.content);
        self.messages.push(message);
    }

    /// Total token usage across every message currently in the window.
    #[must_use]
    pub fn total_tokens(&self) -> u32 {
        // Saturating addition: in the absurd case a malicious caller
        // populates millions of messages each clamped at u32::MAX we still
        // return a meaningful (saturated) total rather than panicking.
        self.messages
            .iter()
            .map(|m| m.token_estimate)
            .fold(0u32, u32::saturating_add)
    }

    /// Utilization ratio `total_tokens / max_tokens` clamped to `[0.0, ∞)`.
    ///
    /// May exceed `1.0` if the caller pushed past the budget without
    /// trimming — that is a legitimate over-budget signal, not an error.
    #[must_use]
    pub fn utilization(&self) -> f32 {
        let total = self.total_tokens() as f32;
        let max = f32::from(u16::try_from(self.max_tokens.min(u32::from(u16::MAX))).unwrap_or(u16::MAX))
            .max(self.max_tokens as f32);
        total / max
    }

    /// Whether [`Self::utilization`] has crossed `summary_threshold`.
    #[must_use]
    pub fn should_summarize(&self) -> bool {
        self.utilization() > self.summary_threshold
    }

    /// Pin the message with the given id so it survives eviction.
    ///
    /// # Errors
    ///
    /// [`ContextError::MessageNotFound`] when no message matches `id`.
    pub fn pin(&mut self, id: &str) -> Result<(), ContextError> {
        let m = self
            .messages
            .iter_mut()
            .find(|m| m.id == id)
            .ok_or_else(|| ContextError::MessageNotFound(id.to_string()))?;
        m.pinned = true;
        Ok(())
    }

    /// Remove the pin flag from the message with the given id.
    ///
    /// # Errors
    ///
    /// [`ContextError::MessageNotFound`] when no message matches `id`.
    pub fn unpin(&mut self, id: &str) -> Result<(), ContextError> {
        let m = self
            .messages
            .iter_mut()
            .find(|m| m.id == id)
            .ok_or_else(|| ContextError::MessageNotFound(id.to_string()))?;
        m.pinned = false;
        Ok(())
    }

    /// Evict oldest non-pinned messages until total tokens drop below
    /// `target_tokens`. Returns the evicted messages in oldest-first order
    /// so the caller can hand them to a summarizer.
    ///
    /// Pinned messages are never removed even when the trim cannot reach
    /// `target_tokens` — the window is allowed to remain over-budget rather
    /// than violate a pin contract. Callers that *must* free more space
    /// have to unpin first.
    pub fn trim_to_fit(&mut self, target_tokens: u32) -> Vec<ContextMessage> {
        let mut evicted = Vec::new();
        // Two-pointer sweep oldest-first; preserves relative order of the
        // surviving (pinned + recent) messages.
        let mut idx = 0;
        while self.total_tokens() > target_tokens && idx < self.messages.len() {
            if self.messages[idx].pinned {
                idx += 1;
                continue;
            }
            evicted.push(self.messages.remove(idx));
            // Do not increment idx: the next element shifted into this slot.
        }
        evicted
    }

    /// Borrow all messages, oldest first.
    #[must_use]
    pub fn messages(&self) -> &[ContextMessage] {
        &self.messages
    }

    /// Look up a message by id.
    #[must_use]
    pub fn find_by_id(&self, id: &str) -> Option<&ContextMessage> {
        self.messages.iter().find(|m| m.id == id)
    }

    /// Number of messages currently held.
    #[must_use]
    pub fn len(&self) -> usize {
        self.messages.len()
    }

    /// Whether the window contains zero messages.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.messages.is_empty()
    }

    /// Maximum token budget configured for this window.
    #[must_use]
    pub const fn max_tokens(&self) -> u32 {
        self.max_tokens
    }

    /// Active summarization threshold.
    #[must_use]
    pub const fn summary_threshold(&self) -> f32 {
        self.summary_threshold
    }

    /// Build a [`SummaryRequest`] from a previous [`Self::trim_to_fit`]
    /// result, sized so the resulting summary fits within
    /// `headroom_tokens`.
    #[must_use]
    pub fn build_summary_request(
        evicted: Vec<ContextMessage>,
        headroom_tokens: u32,
    ) -> SummaryRequest {
        SummaryRequest {
            messages_to_summarize: evicted,
            target_token_budget: headroom_tokens.max(1),
        }
    }

    /// Capture the current state as a portable [`ContextSnapshot`].
    #[must_use]
    pub fn snapshot(&self) -> ContextSnapshot {
        ContextSnapshot {
            messages: self.messages.clone(),
            max_tokens: self.max_tokens,
            summary_threshold: self.summary_threshold,
            created_at: Utc::now(),
        }
    }

    /// Replace the window state with `snapshot`. The estimator is kept as-is.
    ///
    /// # Errors
    ///
    /// [`ContextError::InvalidThreshold`] when the snapshot carries an
    /// out-of-range `summary_threshold`.
    pub fn restore(&mut self, snapshot: ContextSnapshot) -> Result<(), ContextError> {
        Self::validate_threshold(snapshot.summary_threshold)?;
        self.messages = snapshot.messages;
        self.max_tokens = snapshot.max_tokens.max(1);
        self.summary_threshold = snapshot.summary_threshold;
        Ok(())
    }

    /// Serialize the current state to JSON.
    ///
    /// # Errors
    ///
    /// Propagates [`ContextError::Json`] on serde failure.
    pub fn to_json(&self) -> Result<String, ContextError> {
        Ok(serde_json::to_string(&self.snapshot())?)
    }

    /// Restore from a JSON string, replacing current state.
    ///
    /// # Errors
    ///
    /// [`ContextError::Json`] on parse failure;
    /// [`ContextError::InvalidThreshold`] if the embedded threshold is
    /// out of range.
    pub fn from_json(&mut self, json: &str) -> Result<(), ContextError> {
        let snapshot: ContextSnapshot = serde_json::from_str(json)?;
        self.restore(snapshot)
    }
}

// ---------------------------------------------------------------------------
// Inline smoke tests — heavier matrix lives in tests/context_smoke.rs.
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn fresh_window(max: u32) -> ContextWindow {
        ContextWindow::new(max, Arc::new(HeuristicTokenEstimator::default()))
    }

    #[test]
    fn role_as_str_is_snake_case() {
        assert_eq!(MessageRole::System.as_str(), "system");
        assert_eq!(MessageRole::Assistant.as_str(), "assistant");
        assert_eq!(MessageRole::Tool.as_str(), "tool");
    }

    #[test]
    fn empty_window_has_zero_utilization() {
        let w = fresh_window(1024);
        assert_eq!(w.total_tokens(), 0);
        assert!((w.utilization() - 0.0).abs() < f32::EPSILON);
        assert!(!w.should_summarize());
        assert!(w.is_empty());
    }

    #[test]
    fn hash_id_is_deterministic() {
        let a = ContextMessage::hash_id(MessageRole::User, "ping");
        let b = ContextMessage::hash_id(MessageRole::User, "ping");
        let c = ContextMessage::hash_id(MessageRole::Assistant, "ping");
        assert_eq!(a, b);
        assert_ne!(a, c);
    }

    #[test]
    fn build_summary_request_clamps_budget() {
        let req = ContextWindow::build_summary_request(Vec::new(), 0);
        assert_eq!(req.target_token_budget, 1);
        assert!(req.messages_to_summarize.is_empty());
    }
}
