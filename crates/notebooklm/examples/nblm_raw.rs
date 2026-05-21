// SPDX-License-Identifier: Apache-2.0
//! Raw Boq envelope dumper for one notebook — used to re-map the wire layout
//! when the typed parsers drift. Read-only. Not a shipped surface.

use notebooklm::{Auth, NotebookClient, SessionTokens};
use serde_json::{json, Value};

fn env_opt(key: &str) -> Option<String> {
    std::env::var(key).ok().filter(|s| !s.is_empty())
}

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let _ = rustls::crypto::ring::default_provider().install_default();
    let id = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "d68c5204-a2b3-4864-8f65-278844ade83d".to_string());
    let tokens = SessionTokens {
        at: env_opt("NOTEBOOKLM_AT_TOKEN").unwrap(),
        bl: env_opt("NOTEBOOKLM_BL_TOKEN").unwrap(),
        fsid: env_opt("NOTEBOOKLM_FSID_TOKEN"),
        language: env_opt("NOTEBOOKLM_HL"),
    };
    let client = NotebookClient::new(Auth::from_env()?, tokens)?;
    let t = client.transport();

    let list = t
        .rpc_raw("wXbhsf", &json!([Value::Null, 1, Value::Null, [2]]), Some("/"))
        .await?;
    eprintln!("=== LIST_NOTEBOOKS raw ===");
    eprintln!("{}", serde_json::to_string(&list)?);

    let path = format!("/notebook/{id}");
    let get = t
        .rpc_raw(
            "rLM1Ne",
            &json!([id, Value::Null, [2], Value::Null, 1]),
            Some(&path),
        )
        .await?;
    eprintln!("=== GET_NOTEBOOK raw ===");
    eprintln!("{}", serde_json::to_string(&get)?);
    Ok(())
}
