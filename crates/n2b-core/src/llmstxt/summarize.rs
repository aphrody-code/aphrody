// Copyright 2026 Yohan Pierre
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     https://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

//! Appel optionnel à l'API Anthropic pour générer les descriptions courtes
//! (≤ 150 chars) des pages quand la source n'a pas de <meta description>
//! ou un premier paragraphe exploitable.
//!
//! Utilise `reqwest` (rustls, alternative Rust pure à curl/xh).

use super::parse::Site;
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

const ANTHROPIC_ENDPOINT: &str = "https://api.anthropic.com/v1/messages";

#[derive(Serialize)]
struct Req<'a> {
    model: &'a str,
    max_tokens: u32,
    messages: Vec<Msg<'a>>,
}

#[derive(Serialize)]
struct Msg<'a> {
    role: &'a str,
    content: String,
}

#[derive(Deserialize)]
struct Resp {
    content: Vec<RespBlock>,
}

#[derive(Deserialize)]
struct RespBlock {
    #[serde(rename = "type")]
    _ty: String,
    text: String,
}

pub fn run(site: &Site, model: &str) -> Result<()> {
    let api_key = std::env::var("ANTHROPIC_API_KEY").context("ANTHROPIC_API_KEY non défini")?;
    let client = reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_secs(60))
        .build()?;

    // Modifier les pages via iteration — mais site.pages est borrowed.
    // Pour l'instant on print les suggestions côté stderr (non-destructif).
    // Une v2 pourra prendre `&mut Site` et muter les descriptions.
    for page in &site.pages {
        if !needs_summary(&page.description) {
            continue;
        }
        let prompt = format!(
            "Donne une description en une seule phrase de moins de 140 caractères \
            pour la page suivante. Réponds UNIQUEMENT avec la description, sans \
            guillemets ni préambule.\n\n\
            Titre : {}\n\nContenu :\n{}\n",
            page.title,
            first_n_chars(&page.content, 2000)
        );
        let req = Req {
            model,
            max_tokens: 64,
            messages: vec![Msg {
                role: "user",
                content: prompt,
            }],
        };
        let res = client
            .post(ANTHROPIC_ENDPOINT)
            .header("x-api-key", &api_key)
            .header("anthropic-version", "2023-06-01")
            .json(&req)
            .send()
            .with_context(|| format!("POST {ANTHROPIC_ENDPOINT}"))?;
        if !res.status().is_success() {
            eprintln!("  ! [{}] {} → {}", page.title, page.url, res.status());
            continue;
        }
        let body: Resp = res.json()?;
        if let Some(txt) = body.content.first().map(|b| b.text.trim().to_string()) {
            eprintln!("  ~ {} → {txt}", page.title);
        }
    }
    Ok(())
}

fn needs_summary(desc: &str) -> bool {
    desc.trim().is_empty() || desc.chars().count() < 20
}

fn first_n_chars(s: &str, n: usize) -> &str {
    let byte_idx = s.char_indices().nth(n).map(|(i, _)| i).unwrap_or(s.len());
    &s[..byte_idx]
}
