// SPDX-License-Identifier: Apache-2.0
//! Notebook CRUD: list / get / create / rename / delete + recently-viewed scrub.

use serde_json::{json, Value};

use crate::client::NotebookClient;
use crate::error::{NotebookError, Result};
use crate::rpc_ids::{
    CREATE_NOTEBOOK, DELETE_NOTEBOOK, GET_NOTEBOOK, LIST_NOTEBOOKS, REMOVE_RECENTLY_VIEWED,
    RENAME_NOTEBOOK,
};
use crate::types::{Notebook, Source, SourceKind};

impl NotebookClient {
    /// `CCqFvf` — create a fresh empty notebook. Returns `(notebook_id, thread_id)`.
    pub async fn create_notebook(&self) -> Result<(String, String)> {
        let payload = json!([
            "",
            Value::Null,
            Value::Null,
            [2],
            [1, Value::Null, Value::Null, Value::Null, Value::Null, Value::Null, Value::Null,
                Value::Null, Value::Null, Value::Null, [1]],
        ]);
        let envelope = self.transport.rpc_raw(CREATE_NOTEBOOK, &payload, Some("/")).await?;
        let notebook_id = envelope
            .get(2)
            .and_then(Value::as_str)
            .ok_or_else(|| NotebookError::Parse("create_notebook: no notebook id".to_string()))?
            .to_string();
        let thread_id = envelope
            .get(4)
            .and_then(|v| v.get(0))
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_string();
        Ok((notebook_id, thread_id))
    }

    /// `wXbhsf` — list every notebook the active account owns.
    pub async fn list_notebooks(&self) -> Result<Vec<Notebook>> {
        let payload = json!([Value::Null, 1, Value::Null, [2]]);
        let envelope = self.transport.rpc_raw(LIST_NOTEBOOKS, &payload, Some("/")).await?;
        let mut out = Vec::new();
        let Some(rows) = envelope.get(0).and_then(Value::as_array) else { return Ok(out) };
        for row in rows {
            // Row layout (captured from live wire) :
            //   [ <id>, [ <title>, ...], ..., <source_count>?, ...]
            let Some(items) = row.as_array() else { continue };
            let id = items.first().and_then(Value::as_str).unwrap_or_default().to_string();
            if id.is_empty() {
                continue;
            }
            let title = items
                .get(2)
                .and_then(|v| v.as_array())
                .and_then(|a| a.first())
                .and_then(Value::as_str)
                .or_else(|| items.get(2).and_then(Value::as_str))
                .unwrap_or_default()
                .to_string();
            let source_count = items.get(4).and_then(Value::as_u64).map(|n| n as u32);
            out.push(Notebook {
                id,
                title,
                source_count,
                created_at: None,
                updated_at: None,
            });
        }
        Ok(out)
    }

    /// `rLM1Ne` — fetch a single notebook + its source list.
    pub async fn get_notebook(&self, notebook_id: &str) -> Result<(Notebook, Vec<Source>)> {
        let path = format!("/notebook/{notebook_id}");
        let payload = json!([notebook_id, Value::Null, [2], Value::Null, 1]);
        let envelope = self.transport.rpc_raw(GET_NOTEBOOK, &payload, Some(&path)).await?;
        let title = envelope
            .get(2)
            .and_then(|v| v.get(0))
            .and_then(Value::as_str)
            .or_else(|| envelope.get(2).and_then(Value::as_str))
            .unwrap_or_default()
            .to_string();
        let mut sources = Vec::new();
        if let Some(rows) = envelope.get(1).and_then(Value::as_array) {
            for row in rows {
                let Some(items) = row.as_array() else { continue };
                let id =
                    items.first().and_then(Value::as_str).unwrap_or_default().to_string();
                if id.is_empty() {
                    continue;
                }
                let title = items
                    .get(2)
                    .and_then(|v| v.get(0))
                    .and_then(Value::as_str)
                    .unwrap_or_default()
                    .to_string();
                let kind_code = items.get(3).and_then(Value::as_u64).unwrap_or(0);
                let kind = match kind_code {
                    1 => SourceKind::Url,
                    2 => SourceKind::Text,
                    3 => SourceKind::File,
                    4 => SourceKind::YouTube,
                    _ => SourceKind::Url,
                };
                let url = items.get(4).and_then(Value::as_str).map(str::to_string);
                let word_count = items.get(5).and_then(Value::as_u64).map(|n| n as u32);
                let status_code = items.get(6).and_then(Value::as_u64).map(|n| n as u16);
                sources.push(Source {
                    id,
                    title,
                    kind,
                    word_count,
                    status_code,
                    url,
                });
            }
        }
        let notebook = Notebook {
            id: notebook_id.to_string(),
            title,
            source_count: Some(sources.len() as u32),
            created_at: None,
            updated_at: None,
        };
        Ok((notebook, sources))
    }

    /// `s0tc2d` — rename a notebook.
    pub async fn rename_notebook(&self, notebook_id: &str, new_title: &str) -> Result<()> {
        let payload = json!([
            notebook_id,
            [[Value::Null, Value::Null, Value::Null, [Value::Null, new_title]]],
        ]);
        self.transport.rpc_raw(RENAME_NOTEBOOK, &payload, Some("/")).await?;
        Ok(())
    }

    /// `WWINqb` — delete a notebook permanently. The upstream API takes a
    /// batch but we expose the single-id form (simplest mapping).
    pub async fn delete_notebook(&self, notebook_id: &str) -> Result<()> {
        let payload = json!([[notebook_id], [2]]);
        self.transport.rpc_raw(DELETE_NOTEBOOK, &payload, Some("/")).await?;
        Ok(())
    }

    /// `fejl7e` — scrub the notebook from the "recently viewed" dashboard
    /// strip without actually deleting it.
    pub async fn remove_recently_viewed(&self, notebook_id: &str) -> Result<()> {
        let payload = json!([notebook_id]);
        self.transport.rpc_raw(REMOVE_RECENTLY_VIEWED, &payload, Some("/")).await?;
        Ok(())
    }
}
