// SPDX-License-Identifier: Apache-2.0
//! Web research + deep research + import-into-notebook.

use std::time::Duration;

use serde_json::{json, Value};

use crate::client::NotebookClient;
use crate::error::{NotebookError, Result};
use crate::rpc_ids::{CREATE_DEEP_RESEARCH, CREATE_WEB_SEARCH, IMPORT_RESEARCH, POLL_RESEARCH};
use crate::types::ResearchHit;

/// Research depth selector.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResearchMode {
    /// `Ljjv0c` — fast single-pass web search.
    Fast,
    /// `QA9ei` — Gemini Deep Research multi-step report (Ultra subscription
    /// required for unconstrained quotas).
    Deep,
}

impl NotebookClient {
    /// Kick off a research task and return its `(research_id, optional_artifact_id)`.
    pub async fn create_web_search(
        &self,
        notebook_id: &str,
        query: &str,
        mode: ResearchMode,
    ) -> Result<(String, Option<String>)> {
        let path = format!("/notebook/{notebook_id}");
        match mode {
            ResearchMode::Fast => {
                let payload = json!([[query, 1], Value::Null, 1, notebook_id]);
                let envelope =
                    self.transport.rpc_raw(CREATE_WEB_SEARCH, &payload, Some(&path)).await?;
                let id = envelope
                    .get(0)
                    .and_then(Value::as_str)
                    .ok_or_else(|| {
                        NotebookError::Parse(
                            "create_web_search(fast): no research id".to_string(),
                        )
                    })?
                    .to_string();
                Ok((id, None))
            },
            ResearchMode::Deep => self.create_deep_research(notebook_id, query).await,
        }
    }

    /// Explicit deep-research entry point — equivalent to
    /// `create_web_search(.., ResearchMode::Deep)`.
    pub async fn create_deep_research(
        &self,
        notebook_id: &str,
        query: &str,
    ) -> Result<(String, Option<String>)> {
        let path = format!("/notebook/{notebook_id}");
        let payload = json!([Value::Null, [1], [query, 1], 5, notebook_id]);
        let envelope =
            self.transport.rpc_raw(CREATE_DEEP_RESEARCH, &payload, Some(&path)).await?;
        let id = envelope
            .get(0)
            .and_then(Value::as_str)
            .ok_or_else(|| {
                NotebookError::Parse("create_deep_research: no research id".to_string())
            })?
            .to_string();
        let report_id = envelope.get(1).and_then(Value::as_str).map(str::to_string);
        Ok((id, report_id))
    }

    /// `e3bVqc` — poll until the research task reports status >= 2 (`COMPLETED`).
    /// Returns the discovered hits and, for deep research, the inline report body.
    pub async fn poll_research(
        &self,
        notebook_id: &str,
        timeout: Duration,
    ) -> Result<(Vec<ResearchHit>, Option<String>)> {
        let path = format!("/notebook/{notebook_id}");
        let payload = json!([Value::Null, Value::Null, notebook_id]);
        let start = std::time::Instant::now();
        loop {
            let envelope = self.transport.rpc_raw(POLL_RESEARCH, &payload, Some(&path)).await?;
            let status = envelope.get(0).and_then(Value::as_u64).unwrap_or(0);
            if status >= 2 {
                let hits = extract_research_hits(&envelope);
                let report = envelope
                    .get(2)
                    .and_then(Value::as_str)
                    .filter(|s| !s.is_empty())
                    .map(str::to_string);
                return Ok((hits, report));
            }
            if start.elapsed() > timeout {
                return Ok((Vec::new(), None));
            }
            tokio::time::sleep(Duration::from_secs(5)).await;
        }
    }

    /// `LBwxtb` — fold finished research hits (and an optional report) back
    /// into the notebook as new sources.
    pub async fn import_research(
        &self,
        notebook_id: &str,
        research_id: &str,
        hits: &[ResearchHit],
        report: Option<&str>,
    ) -> Result<()> {
        let path = format!("/notebook/{notebook_id}");
        let mut sources = Vec::new();
        if let Some(report_body) = report {
            sources.push(json!([
                Value::Null,
                ["Deep Research Report", report_body],
                Value::Null,
                3,
                Value::Null, Value::Null, Value::Null, Value::Null, Value::Null, Value::Null,
                3
            ]));
        }
        for hit in hits {
            sources.push(json!([
                Value::Null, Value::Null, [hit.url, hit.title], Value::Null,
                Value::Null, Value::Null, Value::Null, Value::Null, Value::Null, Value::Null,
                2
            ]));
        }
        if sources.is_empty() {
            return Ok(());
        }
        let payload = json!([Value::Null, [1], research_id, notebook_id, sources]);
        self.transport.rpc_raw(IMPORT_RESEARCH, &payload, Some(&path)).await?;
        Ok(())
    }
}

fn extract_research_hits(envelope: &Value) -> Vec<ResearchHit> {
    let mut out = Vec::new();
    let Some(rows) = envelope.get(1).and_then(Value::as_array) else { return out };
    for row in rows {
        let Some(items) = row.as_array() else { continue };
        let url = items.first().and_then(Value::as_str).unwrap_or_default().to_string();
        let title = items.get(1).and_then(Value::as_str).unwrap_or_default().to_string();
        let description = items.get(2).and_then(Value::as_str).unwrap_or_default().to_string();
        if !url.is_empty() {
            out.push(ResearchHit { url, title, description });
        }
    }
    out
}
