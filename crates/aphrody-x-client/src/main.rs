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
//!   aphrody-x bookmark 1234567890
//!   aphrody-x unbookmark 1234567890
//!   aphrody-x pin 1234567890
//!   aphrody-x unpin 1234567890
//!   aphrody-x note "long form body text..."
//!   aphrody-x block 2244994945
//!   aphrody-x unblock 2244994945
//!   aphrody-x mute 2244994945
//!   aphrody-x unmute 2244994945
//!   aphrody-x graphql UserByScreenName --var screen_name=aphrody_code
//!   aphrody-x graphql CreateTweet --vars-json '{"tweet_text":"hello","dark_request":false}' --wait
//!   aphrody-x catalog --mutations
//!   aphrody-x catalog --filter Timeline
//!   aphrody-x rate-limit

use std::collections::HashMap;
use std::time::Duration;

use anyhow::{Context, Result};
use clap::{Parser, Subcommand};

use aphrody_x_client::{XClient, XSession};
use aphrody_x_client::catalog;

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
    // -----------------------------------------------------------------------
    // Tweets
    // -----------------------------------------------------------------------
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
    /// Post a long-form note tweet (X Premium).
    Note {
        /// The rich-text body of the note (up to ~25,000 chars on Premium).
        body: String,
        /// Optional short preview tweet text (defaults to first 280 chars of body).
        #[arg(long)]
        preview: Option<String>,
    },

    // -----------------------------------------------------------------------
    // Engagement
    // -----------------------------------------------------------------------
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

    // -----------------------------------------------------------------------
    // Bookmarks
    // -----------------------------------------------------------------------
    /// Bookmark a tweet.
    Bookmark {
        /// Numeric tweet ID.
        id: String,
    },
    /// Remove a bookmark from a tweet.
    Unbookmark {
        /// Numeric tweet ID.
        id: String,
    },

    // -----------------------------------------------------------------------
    // Pin
    // -----------------------------------------------------------------------
    /// Pin a tweet to your profile.
    Pin {
        /// Numeric tweet ID.
        id: String,
    },
    /// Unpin the currently pinned tweet from your profile.
    Unpin {
        /// Numeric tweet ID.
        id: String,
    },

    // -----------------------------------------------------------------------
    // Social graph (REST v1.1)
    // -----------------------------------------------------------------------
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
    /// Block a user by their numeric user ID.
    Block {
        /// Numeric user ID.
        user_id: String,
    },
    /// Unblock a user by their numeric user ID.
    Unblock {
        /// Numeric user ID.
        user_id: String,
    },
    /// Mute a user by their numeric user ID.
    Mute {
        /// Numeric user ID.
        user_id: String,
    },
    /// Unmute a user by their numeric user ID.
    Unmute {
        /// Numeric user ID.
        user_id: String,
    },

    // -----------------------------------------------------------------------
    // Lookup
    // -----------------------------------------------------------------------
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

    // -----------------------------------------------------------------------
    // Direct messages
    // -----------------------------------------------------------------------
    /// Send a direct message.
    Dm {
        /// Numeric recipient user ID.
        user_id: String,
        /// Message text.
        text: String,
    },

    // -----------------------------------------------------------------------
    // Generic GraphQL invoker
    // -----------------------------------------------------------------------
    /// Invoke any operation from the 158-op GraphQL catalog.
    ///
    /// Prints the raw JSON response to stdout.
    ///
    /// Examples:
    ///   aphrody-x graphql UserByScreenName --var screen_name=aphrody_code
    ///   aphrody-x graphql CreateTweet --vars-json '{"tweet_text":"hi","dark_request":false}'
    ///   aphrody-x graphql HomeTimeline --var count=5 --wait
    Graphql {
        /// Exact operation name (case-sensitive, e.g. `CreateTweet`).
        operation: String,

        /// Individual variable as `key=value`.  Value is parsed as JSON if
        /// possible, otherwise treated as a plain string.  May be repeated.
        #[arg(long = "var", value_name = "KEY=VALUE")]
        vars: Vec<String>,

        /// Full variables object as a JSON string (overrides `--var` pairs).
        #[arg(long, value_name = "JSON")]
        vars_json: Option<String>,

        /// If set, use `graphql_waiting` which transparently sleeps out soft
        /// rate limits (window resets) instead of hard-failing.  Hard server-
        /// side limits (e.g. error 344 daily cap) are still propagated as
        /// errors.
        #[arg(long)]
        wait: bool,

        /// Maximum wait time in seconds when `--wait` is set (default: 900 = 15 min).
        #[arg(long, default_value_t = 900)]
        max_wait_secs: u64,
    },

    // -----------------------------------------------------------------------
    // Catalog browser
    // -----------------------------------------------------------------------
    /// List operations from the embedded GraphQL catalog.
    Catalog {
        /// Show only mutation operations.
        #[arg(long, conflicts_with = "queries")]
        mutations: bool,

        /// Show only query operations.
        #[arg(long, conflicts_with = "mutations")]
        queries: bool,

        /// Filter by substring (case-insensitive) in the operation name.
        #[arg(long, value_name = "SUBSTR")]
        filter: Option<String>,
    },

    // -----------------------------------------------------------------------
    // Rate-limit inspector
    // -----------------------------------------------------------------------
    /// Print the most recently captured rate-limit headers.
    ///
    /// Rate-limit headers are captured on every API response.  Run any other
    /// subcommand first (e.g. `user <handle>`) to populate the value, then
    /// use `rate-limit` in a programmatic wrapper.  In standalone use, this
    /// subcommand issues a lightweight `UserByScreenName` lookup to trigger
    /// a response and then prints the captured headers.
    #[command(name = "rate-limit")]
    RateLimit {
        /// Handle to look up for the warm-up request (default: twitter).
        #[arg(long, default_value = "twitter")]
        handle: String,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    let session = resolve_session(cli.cookie_string.as_deref())?;
    let client = XClient::new(session).context("failed to build X HTTP client")?;

    match cli.op {
        // -------------------------------------------------------------------
        // Tweets
        // -------------------------------------------------------------------
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
        Op::Note { body, preview } => {
            let result = client
                .note_tweet(preview.as_deref(), &body)
                .await
                .context("note_tweet failed")?;
            println!("{}", serde_json::to_string_pretty(&result)?);
        }

        // -------------------------------------------------------------------
        // Engagement
        // -------------------------------------------------------------------
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

        // -------------------------------------------------------------------
        // Bookmarks
        // -------------------------------------------------------------------
        Op::Bookmark { id } => {
            client.bookmark(&id).await.context("bookmark failed")?;
            println!("{{\"bookmarked\":\"{id}\"}}");
        }
        Op::Unbookmark { id } => {
            client.unbookmark(&id).await.context("unbookmark failed")?;
            println!("{{\"unbookmarked\":\"{id}\"}}");
        }

        // -------------------------------------------------------------------
        // Pin
        // -------------------------------------------------------------------
        Op::Pin { id } => {
            client.pin_tweet(&id).await.context("pin_tweet failed")?;
            println!("{{\"pinned\":\"{id}\"}}");
        }
        Op::Unpin { id } => {
            client.unpin_tweet(&id).await.context("unpin_tweet failed")?;
            println!("{{\"unpinned\":\"{id}\"}}");
        }

        // -------------------------------------------------------------------
        // Social graph
        // -------------------------------------------------------------------
        Op::Follow { user_id } => {
            client.follow(&user_id).await.context("follow failed")?;
            println!("{{\"followed\":\"{user_id}\"}}");
        }
        Op::Unfollow { user_id } => {
            client.unfollow(&user_id).await.context("unfollow failed")?;
            println!("{{\"unfollowed\":\"{user_id}\"}}");
        }
        Op::Block { user_id } => {
            client.block(&user_id).await.context("block failed")?;
            println!("{{\"blocked\":\"{user_id}\"}}");
        }
        Op::Unblock { user_id } => {
            client.unblock(&user_id).await.context("unblock failed")?;
            println!("{{\"unblocked\":\"{user_id}\"}}");
        }
        Op::Mute { user_id } => {
            client.mute(&user_id).await.context("mute failed")?;
            println!("{{\"muted\":\"{user_id}\"}}");
        }
        Op::Unmute { user_id } => {
            client.unmute(&user_id).await.context("unmute failed")?;
            println!("{{\"unmuted\":\"{user_id}\"}}");
        }

        // -------------------------------------------------------------------
        // Lookup
        // -------------------------------------------------------------------
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

        // -------------------------------------------------------------------
        // Direct messages
        // -------------------------------------------------------------------
        Op::Dm { user_id, text } => {
            client
                .send_dm(&user_id, &text)
                .await
                .context("send_dm failed")?;
            println!("{{\"dm_sent_to\":\"{user_id}\"}}");
        }

        // -------------------------------------------------------------------
        // Generic GraphQL invoker
        // -------------------------------------------------------------------
        Op::Graphql {
            operation,
            vars,
            vars_json,
            wait,
            max_wait_secs,
        } => {
            let variables = build_variables(vars, vars_json)
                .context("failed to build variables object")?;

            let resp = if wait {
                client
                    .graphql_waiting(
                        &operation,
                        variables,
                        None,
                        Duration::from_secs(max_wait_secs),
                    )
                    .await
                    .with_context(|| format!("graphql_waiting({operation}) failed"))?
            } else {
                client
                    .graphql(&operation, variables, None)
                    .await
                    .with_context(|| format!("graphql({operation}) failed"))?
            };

            println!("{}", serde_json::to_string_pretty(&resp)?);
        }

        // -------------------------------------------------------------------
        // Catalog browser
        // -------------------------------------------------------------------
        Op::Catalog {
            mutations,
            queries,
            filter,
        } => {
            let mut ops: Vec<_> = if mutations {
                catalog::mutations()
            } else if queries {
                catalog::queries()
            } else {
                catalog::all()
            };

            // Sort by name for stable output.
            ops.sort_by(|a, b| a.name.cmp(&b.name));

            let filter_lower = filter.as_deref().map(str::to_lowercase);

            let mut count = 0usize;
            for op in ops {
                if let Some(ref f) = filter_lower {
                    if !op.name.to_lowercase().contains(f.as_str()) {
                        continue;
                    }
                }
                println!(
                    "{:<40} {:>28}  {:?}",
                    op.name, op.query_id, op.op_type
                );
                count += 1;
            }
            eprintln!("({count} operations listed)");
        }

        // -------------------------------------------------------------------
        // Rate-limit inspector
        // -------------------------------------------------------------------
        Op::RateLimit { handle } => {
            // Issue a cheap lookup to populate the rate-limit headers.
            let _ = client
                .user_by_screen_name(&handle)
                .await
                .context("warm-up user lookup failed")?;

            match client.last_rate_limit() {
                Some(rl) => {
                    println!(
                        "{}",
                        serde_json::to_string_pretty(&serde_json::json!({
                            "limit": rl.limit,
                            "remaining": rl.remaining,
                            "reset_epoch": rl.reset_epoch,
                        }))?
                    );
                }
                None => {
                    println!("{{\"rate_limit\": null}}");
                }
            }
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

/// Build a `serde_json::Value::Object` from `--var key=value` pairs and/or a
/// `--vars-json` JSON string.
///
/// When `vars_json` is provided it takes precedence; `vars` pairs are ignored.
/// Each `--var` value is parsed as JSON if it successfully deserialises to a
/// scalar or array/object; otherwise it is stored as a plain string.
fn build_variables(
    vars: Vec<String>,
    vars_json: Option<String>,
) -> anyhow::Result<serde_json::Value> {
    if let Some(json_str) = vars_json {
        let v: serde_json::Value = serde_json::from_str(&json_str)
            .context("--vars-json must be valid JSON")?;
        return Ok(v);
    }

    let mut map: HashMap<String, serde_json::Value> = HashMap::new();
    for pair in vars {
        let (key, raw_value) = pair
            .split_once('=')
            .with_context(|| format!("--var must be KEY=VALUE, got: {pair}"))?;

        let val: serde_json::Value = serde_json::from_str(raw_value)
            .unwrap_or_else(|_| serde_json::Value::String(raw_value.to_owned()));

        map.insert(key.to_owned(), val);
    }

    Ok(serde_json::to_value(map)?)
}
