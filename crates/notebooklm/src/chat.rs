// SPDX-License-Identifier: Apache-2.0
//! Chat thread management + streaming free-form chat.

use serde_json::{json, Value};

use crate::boq::{parse_envelopes, strip_xssi};
use crate::client::NotebookClient;
use crate::error::Result;
use crate::rpc_ids::{DELETE_CHAT_THREAD, LIST_CHAT_THREADS};
use crate::types::ChatReply;

impl NotebookClient {
    /// `hPTbtc` — list every chat thread bound to a notebook (sorted newest-first).
    pub async fn list_chat_threads(&self, notebook_id: &str) -> Result<Vec<String>> {
        let path = format!("/notebook/{notebook_id}");
        let payload = json!([[], Value::Null, notebook_id, 20]);
        let envelope = self.transport.rpc_raw(LIST_CHAT_THREADS, &payload, Some(&path)).await?;
        let mut threads = Vec::new();
        if let Some(rows) = envelope.get(0).and_then(Value::as_array) {
            for row in rows {
                if let Some(id) = row.get(0).and_then(Value::as_str) {
                    threads.push(id.to_string());
                } else if let Some(id) = row.as_str() {
                    threads.push(id.to_string());
                }
            }
        }
        Ok(threads)
    }

    /// `J7Gthc` — delete a chat thread.
    pub async fn delete_chat_thread(&self, thread_id: &str) -> Result<()> {
        let payload = json!([[], thread_id, Value::Null, 1]);
        self.transport.rpc_raw(DELETE_CHAT_THREAD, &payload, None).await?;
        Ok(())
    }

    /// Send a free-form message on a notebook's default thread.
    ///
    /// `source_ids` selects which attached sources the model can ground in.
    /// `thread_id` should be filled from [`Self::list_chat_threads`] for
    /// follow-up messages; on a fresh notebook pass `None` and the server
    /// will allocate one.
    pub async fn send_message(
        &self,
        notebook_id: &str,
        message: &str,
        source_ids: &[String],
        thread_id: Option<&str>,
    ) -> Result<ChatReply> {
        let raw = self
            .transport
            .chat_stream(notebook_id, message, source_ids, thread_id, None, 0)
            .await?;
        Ok(parse_chat_stream(&raw, thread_id))
    }
}

/// Accumulate the streamed envelopes into a single [`ChatReply`].
///
/// The chat endpoint emits a `)]}'`-prefixed stream where each NDJSON line is
/// an array of envelopes; we walk every envelope, extract the text fragments
/// from `[ <text>, ..., <metadata> ]` and concatenate.
pub fn parse_chat_stream(raw: &str, fallback_thread_id: Option<&str>) -> ChatReply {
    let stripped = strip_xssi(raw);
    let mut text = String::new();
    let mut thread_id = fallback_thread_id.unwrap_or_default().to_string();
    let mut response_id = None;
    for envelope in parse_envelopes(stripped) {
        // Text fragments live at envelope[4][0][0][1][0] OR envelope[0] depending on shape.
        if let Some(fragment) = envelope
            .get(4)
            .and_then(|v| v.get(0))
            .and_then(|v| v.get(0))
            .and_then(|v| v.get(1))
            .and_then(|v| v.get(0))
            .and_then(Value::as_str)
        {
            text.push_str(fragment);
        } else if let Some(fragment) = envelope.get(0).and_then(Value::as_str) {
            text.push_str(fragment);
        }
        if let Some(id) = envelope
            .get(2)
            .and_then(Value::as_str)
            .filter(|s| !s.is_empty())
        {
            thread_id = id.to_string();
        }
        if let Some(rid) = envelope
            .get(3)
            .and_then(Value::as_str)
            .filter(|s| !s.is_empty())
        {
            response_id = Some(rid.to_string());
        }
    }
    ChatReply {
        text,
        thread_id,
        response_id,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_chat_stream_empty_returns_empty_reply() {
        let reply = parse_chat_stream(")]}'\n", Some("thread-1"));
        assert!(reply.text.is_empty());
        assert_eq!(reply.thread_id, "thread-1");
        assert!(reply.response_id.is_none());
    }
}
