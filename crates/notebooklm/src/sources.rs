// SPDX-License-Identifier: Apache-2.0
//! Sources: add (URL/text/file), delete, refresh, rename, content + summary.

use std::path::Path;
use std::time::Duration;

use reqwest::header::{HeaderMap, HeaderName, HeaderValue};
use serde_json::{json, Value};

use crate::auth::Auth;
use crate::client::NotebookClient;
use crate::error::{NotebookError, Result};
use crate::rpc_ids::{
    ADD_SOURCE, ADD_SOURCE_FILE, DELETE_SOURCE, GET_SOURCE_CONTENT, GET_SOURCE_SUMMARY,
    REFRESH_SOURCE, UPDATE_SOURCE, URL_UPLOAD,
};

impl NotebookClient {
    /// `izAoDd` — attach a URL source. Returns `(source_id, title)`.
    pub async fn add_url(&self, notebook_id: &str, url: &str) -> Result<(String, String)> {
        let path = format!("/notebook/{notebook_id}");
        let payload = json!([
            [[
                Value::Null, Value::Null, [url], Value::Null, Value::Null, Value::Null,
                Value::Null, Value::Null, Value::Null, Value::Null, 1
            ]],
            notebook_id,
            [2],
            [1, Value::Null, Value::Null, Value::Null, Value::Null, Value::Null, Value::Null,
                Value::Null, Value::Null, Value::Null, [1]],
        ]);
        parse_add_source(self.transport.rpc_raw(ADD_SOURCE, &payload, Some(&path)).await?)
    }

    /// `izAoDd` — attach an inline plain-text source.
    pub async fn add_text(
        &self,
        notebook_id: &str,
        title: &str,
        content: &str,
    ) -> Result<(String, String)> {
        let path = format!("/notebook/{notebook_id}");
        let payload = json!([
            [[
                Value::Null,
                [title, content],
                Value::Null,
                2,
                Value::Null, Value::Null, Value::Null, Value::Null, Value::Null, Value::Null,
                1
            ]],
            notebook_id,
            [2],
            [1, Value::Null, Value::Null, Value::Null, Value::Null, Value::Null, Value::Null,
                Value::Null, Value::Null, Value::Null, [1]],
        ]);
        parse_add_source(self.transport.rpc_raw(ADD_SOURCE, &payload, Some(&path)).await?)
    }

    /// `o4cbdc` + Scotty multipart upload — attach a local file as a source.
    ///
    /// Supported formats (server-side): pdf, txt, md, docx, csv, pptx, epub,
    /// mp3, wav, m4a, png, jpg, gif, etc. The function reads the entire file
    /// into memory; callers handling files > 100 MB should split or stream
    /// outside the crate.
    pub async fn add_file(
        &self,
        notebook_id: &str,
        path: impl AsRef<Path>,
    ) -> Result<(String, String)> {
        let abs = path.as_ref().to_path_buf();
        let metadata = std::fs::metadata(&abs)
            .map_err(|e| NotebookError::Network(format!("stat {}: {e}", abs.display())))?;
        if !metadata.is_file() {
            return Err(NotebookError::Network(format!(
                "not a regular file: {}",
                abs.display()
            )));
        }
        let file_size = metadata.len();
        let file_name = abs
            .file_name()
            .and_then(|s| s.to_str())
            .ok_or_else(|| NotebookError::Network("file name not utf-8".to_string()))?
            .to_string();

        let nb_path = format!("/notebook/{notebook_id}");
        let payload = json!([
            [[file_name.clone()]],
            notebook_id,
            [2],
            [1, Value::Null, Value::Null, Value::Null, Value::Null, Value::Null, Value::Null,
                Value::Null, Value::Null, Value::Null, [1]],
        ]);
        let envelope = self
            .transport
            .rpc_raw(ADD_SOURCE_FILE, &payload, Some(&nb_path))
            .await?;
        let (source_id, _title) = parse_add_source(envelope)?;

        let file_bytes = std::fs::read(&abs)
            .map_err(|e| NotebookError::Network(format!("read {}: {e}", abs.display())))?;
        scotty_upload(
            self.transport.tokens(),
            // Re-deref the Arc<Auth> through a borrow that matches the helper signature.
            self.transport.auth_for_upload(),
            notebook_id,
            &file_name,
            &source_id,
            file_size,
            file_bytes,
        )
        .await?;

        Ok((source_id, file_name))
    }

    /// `tGMBJ` — delete one source.
    pub async fn delete_source(&self, source_id: &str) -> Result<()> {
        let payload = json!([[[source_id]], [2]]);
        self.transport.rpc_raw(DELETE_SOURCE, &payload, None).await?;
        Ok(())
    }

    /// `tr032e` — fetch the auto-generated summary for a single source.
    pub async fn get_source_summary(&self, source_id: &str) -> Result<String> {
        let payload = json!([[[[source_id]]]]);
        let envelope = self
            .transport
            .rpc_raw(GET_SOURCE_SUMMARY, &payload, None)
            .await?;
        let summary = envelope
            .get(0)
            .and_then(|v| v.get(0))
            .and_then(Value::as_str)
            .or_else(|| envelope.get(0).and_then(Value::as_str))
            .unwrap_or_default()
            .to_string();
        Ok(summary)
    }

    /// `hizoJc` — fetch the full extracted content of a source.
    pub async fn get_source_content(&self, source_id: &str) -> Result<String> {
        let payload = json!([[[source_id]]]);
        let envelope = self
            .transport
            .rpc_raw(GET_SOURCE_CONTENT, &payload, None)
            .await?;
        // Layout : [[ <content_text> ]] (single envelope, single field).
        let content = envelope
            .get(0)
            .and_then(|v| v.get(0))
            .and_then(Value::as_str)
            .or_else(|| envelope.as_str())
            .unwrap_or_default()
            .to_string();
        Ok(content)
    }

    /// `FLmJqe` — re-index a URL source (re-crawl).
    pub async fn refresh_source(&self, notebook_id: &str, source_id: &str) -> Result<()> {
        let path = format!("/notebook/{notebook_id}");
        let payload = json!([Value::Null, [source_id], [2]]);
        self.transport.rpc_raw(REFRESH_SOURCE, &payload, Some(&path)).await?;
        Ok(())
    }

    /// `b7Wfje` — rename a source (the only mutable field on the source row).
    pub async fn update_source(
        &self,
        notebook_id: &str,
        source_id: &str,
        new_title: &str,
    ) -> Result<()> {
        let path = format!("/notebook/{notebook_id}");
        let payload = json!([Value::Null, [source_id], [[[new_title]]]]);
        self.transport.rpc_raw(UPDATE_SOURCE, &payload, Some(&path)).await?;
        Ok(())
    }
}

fn parse_add_source(envelope: Value) -> Result<(String, String)> {
    // Layout :  [[ <source_id>, <title>, ... ]] (single inserted row).
    let row = envelope
        .get(0)
        .and_then(|v| v.get(0))
        .ok_or_else(|| NotebookError::Parse("add_source: no row".to_string()))?;
    let id = row
        .get(0)
        .and_then(Value::as_str)
        .ok_or_else(|| NotebookError::Parse("add_source: no id".to_string()))?
        .to_string();
    let title = row
        .get(2)
        .and_then(|v| v.get(0))
        .and_then(Value::as_str)
        .or_else(|| row.get(1).and_then(Value::as_str))
        .unwrap_or_default()
        .to_string();
    Ok((id, title))
}

async fn scotty_upload(
    tokens: &crate::transport::SessionTokens,
    auth: &Auth,
    notebook_id: &str,
    file_name: &str,
    source_id: &str,
    file_size: u64,
    file_bytes: Vec<u8>,
) -> Result<()> {
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(300))
        .build()
        .map_err(|e| NotebookError::Network(format!("upload client: {e}")))?;
    let mut headers = HeaderMap::new();
    headers.insert(
        HeaderName::from_static("origin"),
        HeaderValue::from_static("https://notebooklm.google.com"),
    );
    headers.insert(
        HeaderName::from_static("referer"),
        HeaderValue::from_static("https://notebooklm.google.com/"),
    );
    headers.insert(
        HeaderName::from_static("x-goog-authuser"),
        HeaderValue::from_static("0"),
    );
    for (name, value) in auth.request_headers() {
        let header_name = HeaderName::from_static(name);
        let header_value = HeaderValue::from_str(&value).map_err(|e| {
            NotebookError::Auth(format!("invalid upload auth header {name}: {e}"))
        })?;
        headers.insert(header_name, header_value);
    }
    let _ = tokens; // tokens unused on the Scotty path (cookie-only).

    let init_body = json!({
        "PROJECT_ID": notebook_id,
        "SOURCE_NAME": file_name,
        "SOURCE_ID": source_id,
    })
    .to_string();

    let mut init_headers = headers.clone();
    init_headers.insert(
        reqwest::header::CONTENT_TYPE,
        HeaderValue::from_static("application/x-www-form-urlencoded;charset=UTF-8"),
    );
    init_headers.insert(
        HeaderName::from_static("x-goog-upload-command"),
        HeaderValue::from_static("start"),
    );
    init_headers.insert(
        HeaderName::from_static("x-goog-upload-header-content-length"),
        HeaderValue::from_str(&file_size.to_string()).map_err(|e| {
            NotebookError::Network(format!("upload content-length: {e}"))
        })?,
    );
    init_headers.insert(
        HeaderName::from_static("x-goog-upload-protocol"),
        HeaderValue::from_static("resumable"),
    );

    let init_url = format!("{URL_UPLOAD}?authuser=0");
    let init_response = client.post(init_url).headers(init_headers).body(init_body).send().await?;
    let upload_url = init_response
        .headers()
        .get("x-goog-upload-url")
        .and_then(|v| v.to_str().ok())
        .ok_or_else(|| {
            NotebookError::Network("scotty init missing x-goog-upload-url header".to_string())
        })?
        .to_string();

    let mut upload_headers = headers;
    upload_headers.insert(
        reqwest::header::CONTENT_TYPE,
        HeaderValue::from_static("application/x-www-form-urlencoded;charset=utf-8"),
    );
    upload_headers.insert(
        HeaderName::from_static("x-goog-upload-command"),
        HeaderValue::from_static("upload, finalize"),
    );
    upload_headers.insert(
        HeaderName::from_static("x-goog-upload-offset"),
        HeaderValue::from_static("0"),
    );

    let upload_response = client
        .post(upload_url)
        .headers(upload_headers)
        .body(file_bytes)
        .send()
        .await?;
    if !upload_response.status().is_success() {
        return Err(NotebookError::Network(format!(
            "scotty upload returned HTTP {}",
            upload_response.status()
        )));
    }
    Ok(())
}
