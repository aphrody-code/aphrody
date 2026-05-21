// SPDX-License-Identifier: Apache-2.0
//! Pure-HTTP transport: pushes an encoded `f.req` body to `batchexecute` and
//! returns the raw response text. No browser, no FFI, no curl-impersonate.
//!
//! When the upstream eventually rejects the request because the cookie
//! session has expired, callers should surface the [`NotebookError::Auth`]
//! variant to their session-refresh layer (typically `aphrody bxc` headless
//! re-login flow, kept out-of-crate per `project_aphrody_owned_tools`).

use std::sync::Arc;
use std::time::Duration;

use reqwest::header::{HeaderMap, HeaderName, HeaderValue};
use serde::de::DeserializeOwned;
use serde::Serialize;
use serde_json::Value;

use crate::auth::Auth;
use crate::boq::{encode_f_req, first_envelope};
use crate::error::{NotebookError, Result};
use crate::rpc_ids::{URL_BATCH_EXECUTE, URL_CHAT_STREAM};

const USER_AGENT_CHROME: &str = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) \
    AppleWebKit/537.36 (KHTML, like Gecko) Chrome/131.0.0.0 Safari/537.36";

/// Per-session opaque tokens captured from the NotebookLM bootstrap page.
/// These are short-lived (~1h on the cookie path, ~10m for `at`) and must
/// be refreshed by replaying the page load.
#[derive(Debug, Clone, Default, Serialize, serde::Deserialize)]
pub struct SessionTokens {
    /// XSRF token threaded through `f.req` (`at` field).
    pub at: String,
    /// Bootstrap config hash (`bl` query param).
    pub bl: String,
    /// Session id (`f.sid` query param). Optional but recommended.
    pub fsid: Option<String>,
    /// Browser language hint propagated to `hl` query param.
    pub language: Option<String>,
}

/// Public transport object — holds the reqwest client, the credentials, the
/// session tokens and the monotonic `_reqid` counter.
#[derive(Debug, Clone)]
pub struct HttpTransport {
    client: reqwest::Client,
    auth: Arc<Auth>,
    tokens: SessionTokens,
    req_counter: std::sync::Arc<std::sync::atomic::AtomicU64>,
}

impl HttpTransport {
    /// Build a fresh transport. The client honours system proxies and forces
    /// Chrome-friendly TLS via reqwest's `rustls-no-provider` feature (the
    /// crypto provider is installed by the calling binary, e.g. aphrody main).
    pub fn new(auth: Auth, tokens: SessionTokens) -> Result<Self> {
        let client = reqwest::Client::builder()
            .user_agent(USER_AGENT_CHROME)
            .timeout(Duration::from_secs(120))
            .connect_timeout(Duration::from_secs(15))
            .build()
            .map_err(|e| NotebookError::Network(format!("reqwest builder: {e}")))?;
        Ok(Self {
            client,
            auth: Arc::new(auth),
            tokens,
            req_counter: std::sync::Arc::new(std::sync::atomic::AtomicU64::new(100_000)),
        })
    }

    /// Replace the session tokens (typically after a 401 + refresh dance).
    pub fn set_tokens(&mut self, tokens: SessionTokens) {
        self.tokens = tokens;
    }

    /// Borrow the active session tokens (caller-side diagnostics).
    pub fn tokens(&self) -> &SessionTokens {
        &self.tokens
    }

    /// Borrow the active auth (used by the Scotty upload helper which has
    /// to ship the same Cookie/Authorization header set on a side-channel
    /// reqwest client).
    pub(crate) fn auth_for_upload(&self) -> &Auth {
        self.auth.as_ref()
    }

    fn next_req_id(&self) -> u64 {
        use std::sync::atomic::Ordering;
        // 100_000 stride matches the JS reference's `jitteredIncrement` baseline.
        self.req_counter.fetch_add(100_000, Ordering::SeqCst)
    }

    fn build_headers(&self, content_length: usize) -> Result<HeaderMap> {
        let mut headers = HeaderMap::new();
        headers.insert(
            reqwest::header::CONTENT_TYPE,
            HeaderValue::from_static("application/x-www-form-urlencoded;charset=UTF-8"),
        );
        headers.insert(
            reqwest::header::CONTENT_LENGTH,
            HeaderValue::from_str(&content_length.to_string()).map_err(|e| {
                NotebookError::Network(format!("invalid content-length header: {e}"))
            })?,
        );
        headers.insert(
            HeaderName::from_static("origin"),
            HeaderValue::from_static("https://notebooklm.google.com"),
        );
        headers.insert(
            HeaderName::from_static("referer"),
            HeaderValue::from_static("https://notebooklm.google.com/"),
        );
        headers.insert(
            HeaderName::from_static("x-same-domain"),
            HeaderValue::from_static("1"),
        );
        for (name, value) in self.auth.request_headers() {
            // `HeaderName::from_static` panics on non-lowercase names, and the
            // auth layer hands us canonical-cased names ("Cookie",
            // "Authorization"); parse case-insensitively at runtime instead.
            let header_name = HeaderName::from_bytes(name.as_bytes()).map_err(|e| {
                NotebookError::Auth(format!("invalid auth header name {name}: {e}"))
            })?;
            let header_value = HeaderValue::from_str(&value).map_err(|e| {
                NotebookError::Auth(format!("invalid auth header {name}: {e}"))
            })?;
            headers.insert(header_name, header_value);
        }
        Ok(headers)
    }

    /// Execute a single RPC and return the **inner** envelope as a [`serde_json::Value`].
    pub async fn rpc_raw(
        &self,
        rpc_id: &str,
        payload: &Value,
        source_path: Option<&str>,
    ) -> Result<Value> {
        let body_form = encode_f_req(rpc_id, payload)?;
        let body = format!(
            "f.req={}&at={}",
            urlencoded(&body_form),
            urlencoded(&self.tokens.at),
        );
        let req_id = self.next_req_id();
        let mut url = url::Url::parse(URL_BATCH_EXECUTE)
            .map_err(|e| NotebookError::Network(format!("invalid batch URL: {e}")))?;
        {
            let mut qp = url.query_pairs_mut();
            qp.append_pair("rpcids", rpc_id);
            qp.append_pair("source-path", source_path.unwrap_or("/"));
            qp.append_pair("bl", &self.tokens.bl);
            qp.append_pair("hl", self.tokens.language.as_deref().unwrap_or("en"));
            qp.append_pair("_reqid", &req_id.to_string());
            qp.append_pair("rt", "c");
            if let Some(sid) = &self.tokens.fsid {
                qp.append_pair("f.sid", sid);
            }
        }
        let headers = self.build_headers(body.len())?;
        let response = self
            .client
            .post(url)
            .headers(headers)
            .body(body)
            .send()
            .await?;

        let status = response.status();
        let text = response.text().await?;

        if status == reqwest::StatusCode::UNAUTHORIZED || status == reqwest::StatusCode::FORBIDDEN
        {
            return Err(NotebookError::Auth(format!(
                "{status}: cookie/OAuth rejected by batchexecute"
            )));
        }
        if !status.is_success() {
            return Err(NotebookError::Rpc {
                rpc_id: rpc_id.to_string(),
                status: status.as_u16(),
                message: text.chars().take(200).collect(),
            });
        }
        if text.contains("UserDisplayableError") {
            return Err(NotebookError::Rpc {
                rpc_id: rpc_id.to_string(),
                status: status.as_u16(),
                message: "UserDisplayableError in payload".to_string(),
            });
        }
        first_envelope(&text)
    }

    /// Typed RPC helper: serialise the payload, deserialise the envelope.
    pub async fn rpc<P, R>(&self, rpc_id: &str, payload: &P) -> Result<R>
    where
        P: Serialize,
        R: DeserializeOwned,
    {
        let payload_value = serde_json::to_value(payload)?;
        let envelope = self.rpc_raw(rpc_id, &payload_value, None).await?;
        Ok(serde_json::from_value(envelope)?)
    }

    /// Send a chat message and return the raw streaming response text. The
    /// caller is expected to feed the body to [`crate::chat::parse_chat_stream`].
    pub async fn chat_stream(
        &self,
        notebook_id: &str,
        message: &str,
        source_ids: &[String],
        thread_id: Option<&str>,
        history: Option<&Value>,
        turn_counter: u64,
    ) -> Result<String> {
        let source_id_arrays: Vec<Value> = source_ids
            .iter()
            .map(|id| Value::Array(vec![Value::Array(vec![Value::String(id.clone())])]))
            .collect();
        let inner_payload = serde_json::json!([
            source_id_arrays,
            message,
            history.cloned().unwrap_or(Value::Null),
            [2, Value::Null, [1], [1]],
            thread_id.map_or(Value::Null, |t| Value::String(t.to_string())),
            Value::Null,
            Value::Null,
            notebook_id,
            turn_counter + 1,
        ]);
        let envelope = Value::Array(vec![Value::Null, Value::String(inner_payload.to_string())]);
        let body = format!(
            "f.req={}&at={}",
            urlencoded(&envelope.to_string()),
            urlencoded(&self.tokens.at),
        );

        let req_id = self.next_req_id();
        let mut url = url::Url::parse(URL_CHAT_STREAM)
            .map_err(|e| NotebookError::Network(format!("invalid chat URL: {e}")))?;
        {
            let mut qp = url.query_pairs_mut();
            qp.append_pair("bl", &self.tokens.bl);
            qp.append_pair("hl", self.tokens.language.as_deref().unwrap_or("en"));
            qp.append_pair("_reqid", &req_id.to_string());
            qp.append_pair("rt", "c");
            if let Some(sid) = &self.tokens.fsid {
                qp.append_pair("f.sid", sid);
            }
        }
        let headers = self.build_headers(body.len())?;
        let response = self
            .client
            .post(url)
            .headers(headers)
            .body(body)
            .send()
            .await?;
        let status = response.status();
        let text = response.text().await?;
        if status == reqwest::StatusCode::UNAUTHORIZED || status == reqwest::StatusCode::FORBIDDEN
        {
            return Err(NotebookError::Auth(format!("{status}: chat stream rejected")));
        }
        if !status.is_success() {
            return Err(NotebookError::Rpc {
                rpc_id: "CHAT_STREAM".to_string(),
                status: status.as_u16(),
                message: text.chars().take(200).collect(),
            });
        }
        Ok(text)
    }
}

fn urlencoded(input: &str) -> String {
    // RFC 3986 application/x-www-form-urlencoded: anything outside the
    // unreserved set is percent-encoded; spaces become `+`.
    let mut out = String::with_capacity(input.len() * 3 / 2);
    for byte in input.bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' | b'*' => {
                out.push(byte as char);
            },
            b' ' => out.push('+'),
            _ => out.push_str(&format!("%{byte:02X}")),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn urlencode_space_becomes_plus() {
        assert_eq!(urlencoded("hello world"), "hello+world");
        assert_eq!(urlencoded("a&b=c"), "a%26b%3Dc");
        assert_eq!(urlencoded("abc-123_.~*"), "abc-123_.~*");
    }

    #[test]
    fn tokens_default_is_empty() {
        let t = SessionTokens::default();
        assert!(t.at.is_empty());
        assert!(t.bl.is_empty());
        assert!(t.fsid.is_none());
        assert!(t.language.is_none());
    }
}
