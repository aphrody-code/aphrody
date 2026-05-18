// aphrody-x — X (Twitter) client cookie-based (no API key required).
//
// Wraps `agent-twitter-client = "0.1.2"` (Rust port de ai16z/agent-twitter-
// client TS, 11k★ upstream). Auth via cookies seulement — pas besoin de
// X dev portal, pas de free-tier cap 50req/24h.
//
// ⚠ STATUS (2026-05-18, validé par test live avec cookies frais) :
// La crate `agent-twitter-client 0.1.2` (upstream cornip/agent-twitter-
// client, repo DELETED) ne fonctionne PAS out-of-the-box pour X :
//
// Bug #1 — `domain("twitter.com")` hard-codé (Dec 2024, pre-rebrand)
//   Symptôme : cookies .x.com pas envoyés → 401 Unauthorized
//   Fork qui le fixe : `distrihub/agent-twitter-client` (Nov 2025,
//   `.domain("x.com")` lignes 515/581/616/637). Cargo.toml de cette
//   crate pointe sur ce fork via git dep.
//
// Bug #2 (PERSISTANT même avec le fork distrihub) :
//   Même avec cookies frais `auth_token + ct0 + twid + kdt + guest_id
//   + personalization_id + __cf_bm` injectés, l'API X retourne
//   `{"errors":[{"message":"Could not authenticate you","code":32}]}`
//   sur tous les endpoints GraphQL. Suggère que la lib oublie un header
//   ou que la signature ct0 n'est pas calculée correctement (CSRF
//   format X interne). Pas réparable depuis cette crate sans patcher
//   la lib sous-jacente.
//
// **Path de prod recommandé pour scraping X chez aphrody :**
//   → `bxc fetch <url> --cookies-file <file>` (commit 2481a5d9b, GREEN
//     test ultime : user-id + display name + handle extraits du HTML).
//   → Pour post / actions write : passer par X Premium API officiel
//     (Bearer Token v2 sur api.x.com/2/*) plutôt que cookie scraping
//     (TOS-clean + plus stable).
//
// Cette crate est conservée comme placeholder du jour où une lib Rust
// auth-correct sera dispo (suivre rig-twitter ou edisontim fork).
//
// Auth lookup order :
//   1. CLI flag `--cookie-string "auth_token=...; ct0=...; ..."`
//   2. Env `X_COOKIE_STRING` (loaded depuis `.env` via dotenvy)
//   3. Env `X_AUTH_TOKEN` + `X_CT0` (compose la cookie string)
//
// Usage :
//     aphrody-x whoami                # verify_credentials équivalent
//     aphrody-x profile aphrody_code  # fetch profil public d'un handle
//     aphrody-x post "<text>"         # post un tweet
//     aphrody-x latest aphrody_code   # 20 derniers tweets

use anyhow::{anyhow, Context, Result};
use clap::{Parser, Subcommand};

use agent_twitter_client::scraper::Scraper;
use agent_twitter_client::search::SearchMode;

#[derive(Parser)]
#[command(
    name = "aphrody-x",
    version,
    about = "X (Twitter) client cookie-based (no API key required)"
)]
struct Cli {
    /// Cookie string complet "auth_token=...; ct0=...; ...". Overrides env.
    #[arg(long, global = true)]
    cookie_string: Option<String>,

    #[command(subcommand)]
    op: Op,
}

#[derive(Subcommand)]
enum Op {
    /// Imprime le profil de l'utilisateur authentifié (équiv. verify_credentials).
    Whoami,

    /// Fetch le profil public d'un handle.
    Profile { handle: String },

    /// Post un nouveau tweet.
    Post {
        /// Texte du tweet (max 280 chars sauf compte X Premium).
        text: String,
    },

    /// Liste les N derniers tweets d'un handle.
    Latest {
        handle: String,
        #[arg(long, default_value_t = 20)]
        count: u32,
    },

    /// Recherche full-text "Latest" sur X.
    Search {
        query: String,
        #[arg(long, default_value_t = 20)]
        count: u32,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    // Tente .env (silencieux si absent — env vars peuvent venir du shell).
    let _ = dotenvy::dotenv();
    let cli = Cli::parse();

    let cookie_string = resolve_cookie_string(cli.cookie_string.as_deref())?;

    let mut scraper = Scraper::new()
        .await
        .map_err(|e| anyhow!("Scraper::new failed: {e}"))?;
    scraper
        .set_from_cookie_string(&cookie_string)
        .await
        .map_err(|e| anyhow!("set_from_cookie_string failed: {e}"))?;

    match cli.op {
        Op::Whoami => {
            // agent-twitter-client expose `me()` ou équivalent ; à défaut
            // on utilise `profile()` sur le handle stocké dans env.
            let handle = std::env::var("X_HANDLE").unwrap_or_else(|_| "aphrody_code".into());
            let profile = scraper
                .get_profile(&handle)
                .await
                .map_err(|e| anyhow!("get_profile({handle}) failed: {e}"))?;
            println!("{}", serde_json::to_string_pretty(&profile)?);
        }
        Op::Profile { handle } => {
            let profile = scraper
                .get_profile(&handle)
                .await
                .map_err(|e| anyhow!("get_profile({handle}) failed: {e}"))?;
            println!("{}", serde_json::to_string_pretty(&profile)?);
        }
        Op::Post { text } => {
            let resp = scraper
                .send_tweet(&text, None, None)
                .await
                .map_err(|e| anyhow!("send_tweet failed: {e}"))?;
            println!("{}", serde_json::to_string_pretty(&resp)?);
        }
        Op::Latest { handle, count } => {
            // get_user_tweets prend un user_id numérique, pas un handle —
            // on résout le profil d'abord pour obtenir l'id.
            let profile = scraper
                .get_profile(&handle)
                .await
                .map_err(|e| anyhow!("get_profile({handle}) failed: {e}"))?;
            let user_id = profile.id.clone();
            if user_id.is_empty() {
                return Err(anyhow!("profile({handle}) returned empty user id"));
            }
            let tweets = scraper
                .get_user_tweets(&user_id, count as i32, None)
                .await
                .map_err(|e| anyhow!("get_user_tweets({user_id}) failed: {e}"))?;
            println!("{}", serde_json::to_string_pretty(&tweets)?);
        }
        Op::Search { query, count } => {
            let results = scraper
                .search_tweets(&query, count as i32, SearchMode::Latest, None)
                .await
                .map_err(|e| anyhow!("search_tweets failed: {e}"))?;
            println!("{}", serde_json::to_string_pretty(&results)?);
        }
    }
    Ok(())
}

fn resolve_cookie_string(flag: Option<&str>) -> Result<String> {
    if let Some(s) = flag {
        return Ok(s.to_string());
    }
    if let Ok(s) = std::env::var("X_COOKIE_STRING") {
        if !s.is_empty() {
            return Ok(s);
        }
    }
    // Fallback : compose depuis auth_token + ct0 (les 2 cookies vraiment
    // essentiels pour X). Les autres (twid, kdt, __cf_bm) sont automatiques.
    let auth = std::env::var("X_AUTH_TOKEN").context(
        "no cookie source — set --cookie-string flag, X_COOKIE_STRING env, \
         or X_AUTH_TOKEN + X_CT0 envs (typically loaded from .env)",
    )?;
    let ct0 = std::env::var("X_CT0")
        .context("X_AUTH_TOKEN set but X_CT0 missing — both required for write actions")?;
    Ok(format!("auth_token={auth}; ct0={ct0}"))
}
