// SPDX-License-Identifier: Apache-2.0
//! aphrody-x — X / Twitter control CLI (cookie auth, no API key required).
//!
//! Auth lookup order:
//!   1. CLI flag `--cookie-string "auth_token=...; ct0=..."`
//!   2. Session file `~/.aphrody/x-session.json`
//!   3. Env vars `X_AUTH_TOKEN` + `X_CT0`
//!
//! Usage examples:
//!   aphrody-x post "Hello from aphrody"
//!   aphrody-x reply 1234567890 "great thread"
//!   aphrody-x like 1234567890
//!   aphrody-x user aphrody_code
//!   aphrody-x timeline --count 10
//!   aphrody-x dm 2244994945 "hi"

use anyhow::{Context, Result};
use clap::{Parser, Subcommand};

use aphrody_x_client::{XClient, XSession};

#[derive(Parser)]
#[command(
    name = "aphrody-x",
    version,
    about = "X / Twitter control CLI — cookie auth, no API key required"
)]
struct Cli {
    /// Cookie string `auth_token=<val>; ct0=<val>` (overrides session file and env).
    #[arg(long, global = true, env = "X_COOKIE_STRING")]
    cookie_string: Option<String>,

    #[command(subcommand)]
    op: Op,
}

#[derive(Subcommand)]
enum Op {
    /// Post a new tweet.
    Post {
        /// Tweet text (max 280 chars unless X Premium subscriber).
        text: String,
    },
    /// Reply to an existing tweet.
    Reply {
        /// Numeric tweet ID to reply to.
        tweet_id: String,
        /// Reply text.
        text: String,
    },
    /// Delete a tweet by its numeric ID.
    Delete {
        /// Numeric tweet ID.
        id: String,
    },
    /// Like (favorite) a tweet.
    Like {
        /// Numeric tweet ID.
        id: String,
    },
    /// Unlike (remove favorite) a tweet.
    Unlike {
        /// Numeric tweet ID.
        id: String,
    },
    /// Retweet a tweet.
    Retweet {
        /// Numeric tweet ID.
        id: String,
    },
    /// Remove a retweet.
    Unretweet {
        /// Numeric tweet ID.
        id: String,
    },
    /// Follow a user by their numeric user ID.
    Follow {
        /// Numeric user ID (not the handle — use `user <handle>` to resolve it).
        user_id: String,
    },
    /// Unfollow a user by their numeric user ID.
    Unfollow {
        /// Numeric user ID.
        user_id: String,
    },
    /// Look up a user by their handle (without @).
    User {
        /// X handle, e.g. `aphrody_code`.
        handle: String,
    },
    /// Fetch the authenticated user's home timeline.
    Timeline {
        /// Number of tweets to fetch (default: 20).
        #[arg(long, default_value_t = 20)]
        count: u32,
    },
    /// Send a direct message.
    Dm {
        /// Numeric recipient user ID.
        user_id: String,
        /// Message text.
        text: String,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    let session = resolve_session(cli.cookie_string.as_deref())?;
    let client = XClient::new(session).context("failed to build X HTTP client")?;

    match cli.op {
        Op::Post { text } => {
            let result = client
                .create_tweet(&text, None)
                .await
                .context("create_tweet failed")?;
            println!("{}", serde_json::to_string_pretty(&result)?);
        }
        Op::Reply { tweet_id, text } => {
            let result = client
                .create_tweet(&text, Some(&tweet_id))
                .await
                .context("create_tweet (reply) failed")?;
            println!("{}", serde_json::to_string_pretty(&result)?);
        }
        Op::Delete { id } => {
            client
                .delete_tweet(&id)
                .await
                .context("delete_tweet failed")?;
            println!("{{\"deleted\":\"{id}\"}}");
        }
        Op::Like { id } => {
            client.like(&id).await.context("like failed")?;
            println!("{{\"liked\":\"{id}\"}}");
        }
        Op::Unlike { id } => {
            client.unlike(&id).await.context("unlike failed")?;
            println!("{{\"unliked\":\"{id}\"}}");
        }
        Op::Retweet { id } => {
            client.retweet(&id).await.context("retweet failed")?;
            println!("{{\"retweeted\":\"{id}\"}}");
        }
        Op::Unretweet { id } => {
            client.unretweet(&id).await.context("unretweet failed")?;
            println!("{{\"unretweeted\":\"{id}\"}}");
        }
        Op::Follow { user_id } => {
            client
                .follow(&user_id)
                .await
                .context("follow failed")?;
            println!("{{\"followed\":\"{user_id}\"}}");
        }
        Op::Unfollow { user_id } => {
            client
                .unfollow(&user_id)
                .await
                .context("unfollow failed")?;
            println!("{{\"unfollowed\":\"{user_id}\"}}");
        }
        Op::User { handle } => {
            let info = client
                .user_by_screen_name(&handle)
                .await
                .context("user_by_screen_name failed")?;
            println!("{}", serde_json::to_string_pretty(&info)?);
        }
        Op::Timeline { count } => {
            let tweets = client
                .home_timeline(count)
                .await
                .context("home_timeline failed")?;
            println!("{}", serde_json::to_string_pretty(&tweets)?);
        }
        Op::Dm { user_id, text } => {
            client
                .send_dm(&user_id, &text)
                .await
                .context("send_dm failed")?;
            println!("{{\"dm_sent_to\":\"{user_id}\"}}");
        }
    }

    Ok(())
}

/// Resolve an `XSession` from the most specific credential source available.
///
/// Priority:
/// 1. `--cookie-string` CLI flag (or `X_COOKIE_STRING` env via clap).
/// 2. `~/.aphrody/x-session.json`.
/// 3. `X_AUTH_TOKEN` + `X_CT0` env vars.
fn resolve_session(cookie_string: Option<&str>) -> Result<XSession> {
    if let Some(cs) = cookie_string {
        return XSession::from_cookie_string(cs)
            .context("failed to parse --cookie-string");
    }
    XSession::load_or_env().context(
        "no X credentials found — provide --cookie-string, \
         ~/.aphrody/x-session.json, or X_AUTH_TOKEN + X_CT0 env vars",
    )
}
