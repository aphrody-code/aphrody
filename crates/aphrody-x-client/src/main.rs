// aphrody-x — X (Twitter) client cookie-based (no API key required).
//
// Wraps `agent-twitter-client = "0.1.2"` (Rust port de ai16z/agent-twitter-
// client TS, 11k★ upstream). Auth via cookies seulement — pas besoin de
// X dev portal, pas de free-tier cap 50req/24h.
//
// ⚠ BUG UPSTREAM CONNU (2026-05-18, validé par test live) :
// `agent-twitter-client 0.1.2` (cornip/agent-twitter-client, dernier
// release Dec 2024) hard-code `domain("twitter.com")` dans
// `set_from_cookie_string` (src/auth/user_auth.rs:528). Les cookies
// modernes émis sur `.x.com` (depuis le rebrand Twitter → X) ne sont
// donc pas envoyés aux endpoints `api.x.com/*`, ce qui produit un
// `401 Unauthorized` sur le premier appel auth-required (get_profile,
// send_tweet, etc.). Workaround disponibles :
//   1. Fork local de agent-twitter-client patchant la ligne 528 pour
//      attacher domain="x.com" (ou mieux: domain dérivé de l'URL)
//   2. Migrer vers `rig-twitter` (fork ai16z plus récent — à valider)
//   3. Utiliser `bxc fetch --cookies-file` (commit 2481a5d9b) en
//      attendant : bxc émet les cookies correctement sur .x.com et
//      retourne du HTML SSR utilisable (user-id + handle + name visibles).
// Issue upstream à ouvrir : github.com/cornip/agent-twitter-client.
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
