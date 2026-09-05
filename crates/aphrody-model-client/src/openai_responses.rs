// SPDX-License-Identifier: Apache-2.0
//! Local OpenAI Responses-compatible streaming adapter.
//!
//! This is the provider seam used by the Codex architecture: Responses is
//! streamed as SSE and translated into the same neutral events consumed by
//! `aphrody-engine`. It deliberately accepts no API key and is intended for
//! local OpenAI-compatible runtimes such as llama.cpp or Ollama. Gemini
//! remains available through the sibling adapter.

use async_trait::async_trait;
use futures::{Stream, StreamExt};
use serde_json::{Value, json};

use crate::{
    ChatMessage, ChatTurn, ModelClient, ModelError, ModelStream, ModelStreamEvent, Part, Role,
    TokenUsage,
};

/// Build an OpenAI Responses request from Aphrody's provider-neutral turn.
#[must_use]
pub fn build_responses_request(turn: &ChatTurn, model: &str) -> Value {
    let input: Vec<Value> = turn.messages.iter().flat_map(message_to_items).collect();
    let tools: Vec<Value> = turn
        .tools
        .iter()
        .map(|tool| {
            json!({
                "type": "function",
                "name": tool.name,
                "description": tool.description,
                "parameters": tool.parameters,
                "strict": false,
            })
        })
        .collect();

    let mut body = json!({ "model": model, "input": input, "stream": true });
    if let Some(system) = &turn.system {
        body["instructions"] = Value::String(system.clone());
    }
    if !tools.is_empty() {
        body["tools"] = Value::Array(tools);
    }
    body
}

fn message_to_items(message: &ChatMessage) -> Vec<Value> {
    message
        .parts
        .iter()
        .map(|part| match (message.role, part) {
            (Role::User, Part::Text(text)) => json!({
                "role": "user",
                "content": [{ "type": "input_text", "text": text }],
            }),
            (Role::Model, Part::Text(text)) => json!({
                "role": "assistant",
                "content": [{ "type": "output_text", "text": text }],
            }),
            (_, Part::FunctionCall { name, args }) => json!({
                "type": "function_call",
                "call_id": name,
                "name": name,
                "arguments": args.to_string(),
            }),
            (_, Part::FunctionResponse { name, response }) => json!({
                "type": "function_call_output",
                "call_id": name,
                "output": response.to_string(),
            }),
            (_, Part::Text(text)) => json!({
                "role": "user",
                "content": [{ "type": "input_text", "text": text }],
            }),
        })
        .collect()
}

/// Parse one Responses API SSE data payload into neutral model events.
pub fn parse_responses_event(data: &str) -> Result<Vec<ModelStreamEvent>, ModelError> {
    let value: Value = serde_json::from_str(data)?;
    let kind = value.get("type").and_then(Value::as_str).unwrap_or_default();
    let event = match kind {
        "response.created" => Vec::new(),
        "response.output_text.delta" => value
            .get("delta")
            .and_then(Value::as_str)
            .filter(|delta| !delta.is_empty())
            .map(|delta| vec![ModelStreamEvent::TextDelta(delta.to_owned())])
            .unwrap_or_default(),
        "response.reasoning_text.delta" | "response.reasoning_summary_text.delta" => value
            .get("delta")
            .and_then(Value::as_str)
            .filter(|delta| !delta.is_empty())
            .map(|delta| vec![ModelStreamEvent::ReasoningDelta(delta.to_owned())])
            .unwrap_or_default(),
        "response.function_call_arguments.done" => {
            let name = value.get("name").and_then(Value::as_str).unwrap_or_default();
            let id = value
                .get("call_id")
                .or_else(|| value.get("item_id"))
                .and_then(Value::as_str)
                .unwrap_or(name);
            let arguments = value
                .get("arguments")
                .and_then(Value::as_str)
                .map(serde_json::from_str)
                .transpose()?
                .unwrap_or(Value::Null);
            vec![ModelStreamEvent::ToolCall { id: id.to_owned(), name: name.to_owned(), arguments }]
        },
        "response.completed" => {
            let response = value.get("response").unwrap_or(&value);
            let usage = response.get("usage").map(|usage| TokenUsage {
                input_tokens: usage.get("input_tokens").and_then(Value::as_u64).unwrap_or_default(),
                output_tokens: usage
                    .get("output_tokens")
                    .and_then(Value::as_u64)
                    .unwrap_or_default(),
                total_tokens: usage.get("total_tokens").and_then(Value::as_u64).unwrap_or_default(),
            });
            vec![ModelStreamEvent::Completed {
                usage,
                finish_reason: response.get("status").and_then(Value::as_str).map(str::to_owned),
            }]
        },
        "response.failed" | "error" => {
            let message = value
                .pointer("/error/message")
                .or_else(|| value.get("message"))
                .and_then(Value::as_str)
                .unwrap_or("OpenAI Responses stream failed");
            return Err(ModelError::Api { status: 502, message: message.to_owned() });
        },
        _ => Vec::new(),
    };
    Ok(event)
}

/// Streaming client for a local `/v1/responses` endpoint.
///
/// No API key or cloud authentication is accepted by this type. The endpoint
/// must be a local, OpenAI-compatible runtime selected by the caller.
#[derive(Clone)]
pub struct LocalResponsesClient {
    http: reqwest::Client,
    base_url: String,
    model: String,
}

impl LocalResponsesClient {
    /// Construct a client with a local OpenAI-compatible Responses endpoint.
    #[must_use]
    pub fn new(base_url: impl Into<String>, model: impl Into<String>) -> Self {
        Self { http: reqwest::Client::new(), base_url: base_url.into(), model: model.into() }
    }

    fn url(&self) -> String {
        format!("{}/v1/responses", self.base_url.trim_end_matches('/'))
    }
}

#[async_trait]
impl ModelClient for LocalResponsesClient {
    async fn stream(&self, turn: ChatTurn) -> Result<ModelStream, ModelError> {
        let response = self
            .http
            .post(self.url())
            .json(&build_responses_request(&turn, &self.model))
            .send()
            .await?;
        let status = response.status();
        if !status.is_success() {
            return Err(ModelError::Api {
                status: status.as_u16(),
                message: response.text().await.unwrap_or_default(),
            });
        }
        Ok(Box::pin(responses_sse_stream(response.bytes_stream())))
    }

    fn model_id(&self) -> &str {
        &self.model
    }
}

fn responses_sse_stream<S>(
    bytes: S,
) -> impl Stream<Item = Result<ModelStreamEvent, ModelError>> + Send
where
    S: Stream<Item = reqwest::Result<bytes::Bytes>> + Send + 'static,
{
    futures::stream::once(async { Ok(ModelStreamEvent::Created) }).chain(
        futures::stream::unfold(
            (Box::pin(bytes), String::new()),
            |(mut stream, mut buffer)| async move {
                loop {
                    let frames = crate::split_sse_events(&mut buffer);
                    if let Some(frame) = frames.into_iter().next() {
                        let data = crate::sse_event_data(&frame);
                        if data.trim().is_empty() || data.trim() == "[DONE]" {
                            continue;
                        }
                        let items = match parse_responses_event(data.trim()) {
                            Ok(events) => events.into_iter().map(Ok).collect::<Vec<_>>(),
                            Err(error) => vec![Err(error)],
                        };
                        if !items.is_empty() {
                            return Some((futures::stream::iter(items), (stream, buffer)));
                        }
                    }
                    match stream.next().await {
                        Some(Ok(chunk)) => buffer.push_str(
                            std::str::from_utf8(&chunk)
                                .map_err(|error| ModelError::Stream(error.to_string()))
                                .unwrap_or_default(),
                        ),
                        Some(Err(error)) => {
                            return Some((
                                futures::stream::iter(vec![Err(ModelError::Http(error))]),
                                (stream, buffer),
                            ));
                        },
                        None => return None,
                    }
                }
            },
        )
        .flatten(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn responses_text_delta_is_mapped() {
        assert_eq!(
            parse_responses_event(r#"{"type":"response.output_text.delta","delta":"hi"}"#).unwrap(),
            vec![ModelStreamEvent::TextDelta("hi".into())]
        );
    }

    #[test]
    fn responses_function_call_is_mapped() {
        let events = parse_responses_event(r#"{"type":"response.function_call_arguments.done","call_id":"call_1","name":"shell","arguments":"{\"cmd\":\"pwd\"}"}"#).unwrap();
        assert!(
            matches!(&events[0], ModelStreamEvent::ToolCall { id, name, arguments } if id == "call_1" && name == "shell" && arguments["cmd"] == "pwd")
        );
    }
}
