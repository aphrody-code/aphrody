// SPDX-License-Identifier: Apache-2.0
//! Full read-surface probe for a single NotebookLM notebook.
//!
//! Auth comes from env (same vars as `aphrody notebooklm`):
//!   NOTEBOOKLM_COOKIES (Cookie-Editor JSON) or NOTEBOOKLM_OAUTH_TOKEN
//!   NOTEBOOKLM_AT_TOKEN  (WIZ_global_data.SNlM0e)
//!   NOTEBOOKLM_BL_TOKEN  (cfb2h / bl)
//!   NOTEBOOKLM_FSID_TOKEN (optional FdrFJe)
//!
//! Target notebook id is the single CLI arg. Read-only: lists notebook
//! metadata + sources (+ per-source summary/content), chat threads, sends one
//! non-destructive probe message, and lists artifacts. Emits one JSON blob.

use notebooklm::{Auth, NotebookClient, SessionTokens};

fn env_opt(key: &str) -> Option<String> {
    std::env::var(key).ok().filter(|s| !s.is_empty())
}

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // rustls 0.23 needs an explicit CryptoProvider before the first
    // reqwest::Client (cf. CLAUDE.md section 7); the binary does this in main.
    let _ = rustls::crypto::ring::default_provider().install_default();

    let notebook_id = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "d68c5204-a2b3-4864-8f65-278844ade83d".to_string());

    let tokens = SessionTokens {
        at: env_opt("NOTEBOOKLM_AT_TOKEN").ok_or("NOTEBOOKLM_AT_TOKEN missing")?,
        bl: env_opt("NOTEBOOKLM_BL_TOKEN").ok_or("NOTEBOOKLM_BL_TOKEN missing")?,
        fsid: env_opt("NOTEBOOKLM_FSID_TOKEN"),
        language: env_opt("NOTEBOOKLM_HL"),
    };
    let auth = Auth::from_env()?;
    eprintln!("[nblm] auth flavour = {}", auth.flavour());
    let client = NotebookClient::new(auth, tokens)?;

    let mut out = serde_json::Map::new();
    out.insert("notebook_id".into(), notebook_id.clone().into());

    // 1. notebook list (confirm ownership + grab title)
    match client.list_notebooks().await {
        Ok(nbs) => {
            let hit = nbs.iter().find(|n| n.id == notebook_id);
            out.insert("list_count".into(), nbs.len().into());
            out.insert(
                "in_list".into(),
                serde_json::json!(hit.map(|n| n.title.clone())),
            );
        }
        Err(e) => {
            out.insert("list_error".into(), e.to_string().into());
        }
    }

    // 2. notebook metadata + sources
    let mut source_ids: Vec<String> = Vec::new();
    match client.get_notebook(&notebook_id).await {
        Ok((nb, sources)) => {
            out.insert("notebook".into(), serde_json::to_value(&nb)?);
            out.insert("source_count".into(), sources.len().into());
            // Cap per-source summary/content fetches to avoid 2*N round-trips.
            const DEEP_FETCH: usize = 6;
            let mut src_arr = Vec::new();
            for (i, s) in sources.iter().enumerate() {
                source_ids.push(s.id.clone());
                let mut entry = serde_json::json!({
                    "id": s.id,
                    "title": s.title,
                    "kind": s.kind,
                    "url": s.url,
                    "word_count": s.word_count,
                });
                if i < DEEP_FETCH {
                    let summary = client
                        .get_source_summary(&s.id)
                        .await
                        .unwrap_or_else(|e| format!("<summary error: {e}>"));
                    let content = client
                        .get_source_content(&s.id)
                        .await
                        .unwrap_or_else(|e| format!("<content error: {e}>"));
                    let preview: String = content.chars().take(400).collect();
                    entry["summary"] = summary.into();
                    entry["content_len"] = content.chars().count().into();
                    entry["content_preview"] = preview.into();
                }
                src_arr.push(entry);
            }
            out.insert("sources".into(), serde_json::Value::Array(src_arr));
        }
        Err(e) => {
            out.insert("get_notebook_error".into(), e.to_string().into());
        }
    }

    // 3. chat threads
    match client.list_chat_threads(&notebook_id).await {
        Ok(threads) => {
            out.insert("chat_threads".into(), serde_json::to_value(&threads)?);
        }
        Err(e) => {
            out.insert("chat_threads_error".into(), e.to_string().into());
        }
    }

    // 4. probe chat (non-destructive)
    let probe = "Resume en 5 puces concises les points cles de toutes les sources de ce notebook.";
    match client
        .send_message(&notebook_id, probe, &source_ids, None)
        .await
    {
        Ok(reply) => {
            out.insert("probe_prompt".into(), probe.into());
            out.insert("probe_reply".into(), serde_json::to_value(&reply)?);
        }
        Err(e) => {
            out.insert("probe_error".into(), e.to_string().into());
        }
    }

    // 5. artifacts
    match client.get_artifacts_filtered(&notebook_id).await {
        Ok(arts) => {
            out.insert("artifact_count".into(), arts.len().into());
            out.insert("artifacts".into(), serde_json::to_value(&arts)?);
        }
        Err(e) => {
            out.insert("artifacts_error".into(), e.to_string().into());
        }
    }

    println!("{}", serde_json::to_string_pretty(&serde_json::Value::Object(out))?);
    Ok(())
}
