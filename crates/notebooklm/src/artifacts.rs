// SPDX-License-Identifier: Apache-2.0
//! Artifacts: generate, poll, list, rename, delete, get interactive HTML,
//! export, share.

use std::time::Duration;

use serde_json::{json, Value};

use crate::client::NotebookClient;
use crate::error::{NotebookError, Result};
use crate::rpc_ids::{
    DEFAULT_USER_CONFIG, DELETE_ARTIFACT, EXPORT_ARTIFACT, GENERATE_ARTIFACT,
    GET_ARTIFACTS_FILTERED, GET_INTERACTIVE_HTML, RENAME_ARTIFACT, SHARE_ARTIFACT,
};
use crate::types::{Artifact, ArtifactKind};

fn default_user_config() -> Value {
    serde_json::from_str(DEFAULT_USER_CONFIG).expect("DEFAULT_USER_CONFIG must parse")
}

impl NotebookClient {
    /// `R7cb6c` — kick off artifact generation.
    ///
    /// `prompt` carries optional natural-language instructions appended to
    /// the upstream workflow (audio style hints, report template name, …).
    /// Returns `(artifact_id, title)`. The artifact starts in `PENDING`
    /// state; poll with [`Self::poll_artifact`] until `download_url` is set.
    pub async fn generate(
        &self,
        notebook_id: &str,
        source_ids: &[String],
        kind: ArtifactKind,
        prompt: Option<&str>,
    ) -> Result<(String, String)> {
        let path = format!("/notebook/{notebook_id}");
        let sids_triple: Vec<Value> = source_ids
            .iter()
            .map(|id| Value::Array(vec![Value::Array(vec![Value::String(id.clone())])]))
            .collect();
        let sids_double: Vec<Value> = source_ids
            .iter()
            .map(|id| Value::Array(vec![Value::String(id.clone())]))
            .collect();
        let kind_id = kind.as_wire_id();
        let instructions = prompt.unwrap_or("");
        // Inner payload shape mirrors `buildArtifactPayload` in the JS reference:
        //   [ <kind_id>, <sids_triple>, [ <prompt>, ... ], <sids_double>, ... ]
        let inner = json!([
            kind_id,
            sids_triple,
            [instructions, Value::Null, Value::Null, Value::Null, Value::Null, Value::Null,
                Value::Null, Value::Null, Value::Null, Value::Null, [1]],
            sids_double,
        ]);
        let payload = json!([default_user_config(), notebook_id, inner]);
        let envelope = self.transport.rpc_raw(GENERATE_ARTIFACT, &payload, Some(&path)).await?;
        let id = envelope
            .get(0)
            .and_then(Value::as_str)
            .or_else(|| envelope.get(0).and_then(|v| v.get(0)).and_then(Value::as_str))
            .ok_or_else(|| {
                NotebookError::Parse("generate_artifact: missing artifact id".to_string())
            })?
            .to_string();
        let title = envelope
            .get(1)
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_string();
        Ok((id, title))
    }

    /// `gArtLc` — list every artifact currently attached to a notebook
    /// (suggested artifacts are filtered out by default).
    pub async fn get_artifacts_filtered(&self, notebook_id: &str) -> Result<Vec<Artifact>> {
        let path = format!("/notebook/{notebook_id}");
        let payload = json!([
            default_user_config(),
            notebook_id,
            "NOT artifact.status = \"ARTIFACT_STATUS_SUGGESTED\"",
        ]);
        let envelope = self
            .transport
            .rpc_raw(GET_ARTIFACTS_FILTERED, &payload, Some(&path))
            .await?;
        let mut artifacts = Vec::new();
        let Some(rows) = envelope.get(0).and_then(Value::as_array) else { return Ok(artifacts) };
        for row in rows {
            let Some(items) = row.as_array() else { continue };
            let id = items.first().and_then(Value::as_str).unwrap_or_default().to_string();
            if id.is_empty() {
                continue;
            }
            let title = items.get(1).and_then(Value::as_str).unwrap_or_default().to_string();
            let kind_wire = items.get(2).and_then(Value::as_u64).unwrap_or(0) as u32;
            let kind = ArtifactKind::from_wire_id(kind_wire).unwrap_or(ArtifactKind::Report);
            let download_url = items.get(3).and_then(Value::as_str).map(str::to_string);
            let stream_url = items.get(4).and_then(Value::as_str).map(str::to_string);
            let duration_seconds =
                items.get(5).and_then(Value::as_u64).map(|n| n as u32);
            let source_ids = items
                .get(6)
                .and_then(Value::as_array)
                .map(|arr| {
                    arr.iter()
                        .filter_map(|v| v.as_str().map(str::to_string))
                        .collect()
                })
                .unwrap_or_default();
            artifacts.push(Artifact {
                id,
                title,
                kind,
                download_url,
                stream_url,
                duration_seconds,
                source_ids,
            });
        }
        Ok(artifacts)
    }

    /// Poll [`Self::get_artifacts_filtered`] until the supplied id reports a
    /// non-empty `download_url`, or the timeout elapses.
    pub async fn poll(
        &self,
        notebook_id: &str,
        artifact_id: &str,
        timeout: Duration,
    ) -> Result<Artifact> {
        let start = std::time::Instant::now();
        loop {
            let artifacts = self.get_artifacts_filtered(notebook_id).await?;
            if let Some(artifact) = artifacts.into_iter().find(|a| a.id == artifact_id) {
                if artifact.download_url.is_some() {
                    return Ok(artifact);
                }
            }
            if start.elapsed() > timeout {
                return Err(NotebookError::Rpc {
                    rpc_id: GET_ARTIFACTS_FILTERED.to_string(),
                    status: 0,
                    message: format!("poll timeout after {:?}", timeout),
                });
            }
            tokio::time::sleep(Duration::from_secs(5)).await;
        }
    }

    /// `V5N4be` — delete one artifact.
    pub async fn delete_artifact(&self, artifact_id: &str) -> Result<()> {
        let payload = json!([default_user_config(), artifact_id]);
        self.transport.rpc_raw(DELETE_ARTIFACT, &payload, None).await?;
        Ok(())
    }

    /// `rc3d8d` — rename an artifact.
    pub async fn rename_artifact(&self, artifact_id: &str, new_title: &str) -> Result<()> {
        let payload = json!([artifact_id, new_title]);
        self.transport.rpc_raw(RENAME_ARTIFACT, &payload, None).await?;
        Ok(())
    }

    /// `v9rmvd` — fetch the interactive-HTML body of an artifact (mind map,
    /// flashcards deck, interactive quiz, …).
    pub async fn get_interactive_html(&self, artifact_id: &str) -> Result<String> {
        let payload = json!([artifact_id]);
        let envelope = self
            .transport
            .rpc_raw(GET_INTERACTIVE_HTML, &payload, None)
            .await?;
        let html = match envelope {
            Value::String(s) => s,
            Value::Array(items) => items
                .iter()
                .find_map(|v| match v {
                    Value::String(s) if s.len() > 200 && s.contains('<') => Some(s.clone()),
                    Value::Array(inner) => inner.iter().find_map(|x| match x {
                        Value::String(s) if s.len() > 200 && s.contains('<') => Some(s.clone()),
                        _ => None,
                    }),
                    _ => None,
                })
                .unwrap_or_default(),
            _ => String::new(),
        };
        Ok(html)
    }

    /// `Krh3pd` — produce a downloadable export of an artifact (PPTX, PDF,
    /// CSV, …). Returns the resolved download URL.
    pub async fn export_artifact(&self, artifact_id: &str) -> Result<String> {
        let payload = json!([artifact_id]);
        let envelope = self
            .transport
            .rpc_raw(EXPORT_ARTIFACT, &payload, None)
            .await?;
        let url = envelope
            .get(0)
            .and_then(Value::as_str)
            .or_else(|| envelope.as_str())
            .unwrap_or_default()
            .to_string();
        Ok(url)
    }

    /// `RGP97b` — toggle public sharing for an artifact.
    pub async fn share_artifact(&self, artifact_id: &str, public: bool) -> Result<()> {
        let payload = json!([artifact_id, if public { 1 } else { 0 }]);
        self.transport.rpc_raw(SHARE_ARTIFACT, &payload, None).await?;
        Ok(())
    }
}
