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
use std::future::Future;
use std::pin::Pin;
use std::time::Duration;

use anyhow::{Context, Result};
use clap::{Parser, Subcommand};

use aphrody_x_client::catalog;
use aphrody_x_client::{TweetPage, UserPage, XClient, XSession};

/// Boxed future returning a tweet page (for the generic paginator).
type TweetFut<'a> = Pin<Box<dyn Future<Output = Result<TweetPage>> + 'a>>;
/// Boxed future returning a user page.
type UserFut<'a> = Pin<Box<dyn Future<Output = Result<UserPage>> + 'a>>;

/// Loop a tweet-page fetcher across cursors, honoring `--all` / `--max-pages`.
async fn paginate_tweets<'a>(
    all: bool,
    max_pages: u32,
    start_cursor: Option<String>,
    mut fetch: impl FnMut(Option<String>) -> TweetFut<'a>,
) -> Result<TweetPage> {
    let mut tweets = Vec::new();
    let mut cursor = start_cursor;
    let mut pages = 0u32;
    loop {
        let page = fetch(cursor.clone()).await?;
        let got = page.tweets.len();
        tweets.extend(page.tweets);
        pages += 1;
        cursor = page.next_cursor;
        if !all || cursor.is_none() || got == 0 || pages >= max_pages {
            break;
        }
    }
    Ok(TweetPage {
        tweets,
        next_cursor: cursor,
    })
}

/// Loop a user-page fetcher across cursors, honoring `--all` / `--max-pages`.
async fn paginate_users<'a>(
    all: bool,
    max_pages: u32,
    start_cursor: Option<String>,
    mut fetch: impl FnMut(Option<String>) -> UserFut<'a>,
) -> Result<UserPage> {
    let mut users = Vec::new();
    let mut cursor = start_cursor;
    let mut pages = 0u32;
    loop {
        let page = fetch(cursor.clone()).await?;
        let got = page.users.len();
        users.extend(page.users);
        pages += 1;
        cursor = page.next_cursor;
        if !all || cursor.is_none() || got == 0 || pages >= max_pages {
            break;
        }
    }
    Ok(UserPage {
        users,
        next_cursor: cursor,
    })
}

/// Extract a numeric tweet id from a URL or return the input if already numeric.
fn extract_tweet_id(input: &str) -> Result<String> {
    let trimmed = input.trim();
    if trimmed.chars().all(|c| c.is_ascii_digit()) && !trimmed.is_empty() {
        return Ok(trimmed.to_owned());
    }
    // .../status/<id> or .../statuses/<id>, ignoring query/fragment.
    for marker in ["/status/", "/statuses/"] {
        if let Some(rest) = trimmed.split(marker).nth(1) {
            let id: String = rest.chars().take_while(char::is_ascii_digit).collect();
            if !id.is_empty() {
                return Ok(id);
            }
        }
    }
    Err(anyhow::anyhow!("could not extract a tweet id from: {input}"))
}

/// Extract a numeric list id from a URL or return the input if already numeric.
fn extract_list_id(input: &str) -> Result<String> {
    let trimmed = input.trim();
    if trimmed.chars().all(|c| c.is_ascii_digit()) && !trimmed.is_empty() {
        return Ok(trimmed.to_owned());
    }
    if let Some(rest) = trimmed.split("/lists/").nth(1) {
        let id: String = rest.chars().take_while(char::is_ascii_digit).collect();
        if !id.is_empty() {
            return Ok(id);
        }
    }
    Err(anyhow::anyhow!("could not extract a list id from: {input}"))
}

/// Normalize a handle by stripping a leading `@`.
fn normalize_handle(h: &str) -> &str {
    h.trim().trim_start_matches('@')
}

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

    // -----------------------------------------------------------------------
    // Runtime queryId cache
    // -----------------------------------------------------------------------
    /// Inspect or refresh the runtime GraphQL queryId cache.
    ///
    /// X rotates queryIds whenever it ships a new web bundle. This command
    /// scrapes the live values straight from X's public bundles and caches
    /// them on disk so the client survives rotations without a recompile.
    #[command(name = "query-ids")]
    QueryIds {
        /// Force a fresh scrape of X's web bundles and overwrite the cache.
        #[arg(long)]
        refresh: bool,
    },

    // -----------------------------------------------------------------------
    // Reading (typed, paginated) — outputs JSON
    // -----------------------------------------------------------------------
    /// Read a single tweet by URL or numeric id.
    Read {
        /// Tweet URL or numeric id.
        target: String,
    },
    /// Show the full conversation thread for a tweet.
    Thread {
        /// Tweet URL or numeric id.
        target: String,
        #[command(flatten)]
        page: PageArgs,
    },
    /// List replies to a tweet (thread entries that reply to it).
    Replies {
        /// Tweet URL or numeric id.
        target: String,
        #[command(flatten)]
        page: PageArgs,
    },
    /// Search tweets matching a query.
    Search {
        /// Search query (X search syntax, e.g. `from:steipete since:2026-01-01`).
        query: String,
        /// Use the "Top" tab instead of "Latest".
        #[arg(long)]
        top: bool,
        #[command(flatten)]
        page: PageArgs,
    },
    /// Fetch a user's profile timeline (by handle).
    #[command(name = "user-tweets")]
    UserTweets {
        /// Handle with or without leading `@`.
        handle: String,
        #[command(flatten)]
        page: PageArgs,
    },
    /// Fetch your home timeline (For You, or Following with --following).
    Home {
        /// Use the chronological Following feed instead of For You.
        #[arg(long)]
        following: bool,
        #[command(flatten)]
        page: PageArgs,
    },
    /// List your liked tweets.
    Likes {
        #[command(flatten)]
        page: PageArgs,
    },
    /// List your bookmarked tweets.
    Bookmarks {
        #[command(flatten)]
        page: PageArgs,
    },
    /// Find mentions of a user (defaults to the authenticated account).
    Mentions {
        /// Handle to search mentions for (default: you).
        #[arg(long)]
        user: Option<String>,
        #[command(flatten)]
        page: PageArgs,
    },
    /// List accounts a user follows (default: you).
    Following {
        /// Handle to inspect (default: you).
        #[arg(long)]
        user: Option<String>,
        #[command(flatten)]
        page: PageArgs,
    },
    /// List accounts that follow a user (default: you).
    Followers {
        /// Handle to inspect (default: you).
        #[arg(long)]
        user: Option<String>,
        #[command(flatten)]
        page: PageArgs,
    },
    /// Fetch tweets from a list timeline (by URL or numeric id).
    #[command(name = "list-timeline")]
    ListTimeline {
        /// List URL or numeric id.
        target: String,
        #[command(flatten)]
        page: PageArgs,
    },
    /// Print which X account your cookies belong to.
    Whoami,
    /// Show which credential sources are available and where they resolve from.
    Check,
}

/// Shared pagination/quote arguments for reading commands.
#[derive(clap::Args, Clone)]
struct PageArgs {
    /// Number of items per page (default: 20).
    #[arg(short = 'n', long, default_value_t = 20)]
    count: u32,
    /// Fetch all pages (until exhausted or --max-pages).
    #[arg(long)]
    all: bool,
    /// Maximum number of pages to fetch when --all is set (default: 5).
    #[arg(long, default_value_t = 5)]
    max_pages: u32,
    /// Start from an explicit pagination cursor.
    #[arg(long)]
    cursor: Option<String>,
    /// Max quoted-tweet nesting depth in output (default: 1; 0 disables).
    #[arg(long, default_value_t = 1)]
    quote_depth: u32,
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    let session = resolve_session(cli.cookie_string.as_deref())?;
    let client = XClient::new(session).context("failed to build X HTTP client")?;
    // Shadow as a shared reference so paginator closures (FnMut) can capture a
    // Copy of `&XClient` into their `async move` futures without moving it.
    let client = &client;

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
                if let Some(ref f) = filter_lower
                    && !op.name.to_lowercase().contains(f.as_str())
                {
                    continue;
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

        // -------------------------------------------------------------------
        // Runtime queryId cache
        // -------------------------------------------------------------------
        Op::QueryIds { refresh } => {
            let store = client.query_ids();
            if refresh {
                let names: Vec<&str> = catalog::all().iter().map(|o| o.name.as_str()).collect();
                store
                    .refresh(&names, true)
                    .await
                    .context("queryId refresh failed (network or x.com layout change)")?;
            }
            let snap = store.snapshot();
            let out = serde_json::json!({
                "cache_path": store.cache_path().display().to_string(),
                "fresh": snap.as_ref().map(aphrody_x_client::QueryIdSnapshot::is_fresh),
                "fetched_at": snap.as_ref().map(|s| s.fetched_at),
                "age_secs": snap.as_ref().map(aphrody_x_client::QueryIdSnapshot::age_secs),
                "count": snap.as_ref().map_or(0, |s| s.ids.len()),
                "ids": snap.map(|s| s.ids).unwrap_or_default(),
            });
            println!("{}", serde_json::to_string_pretty(&out)?);
        }

        // -------------------------------------------------------------------
        // Reading (typed, paginated)
        // -------------------------------------------------------------------
        Op::Read { target } => {
            let id = extract_tweet_id(&target)?;
            let tweet = client
                .get_tweet(&id, 1)
                .await
                .context("get_tweet failed")?;
            match tweet {
                Some(t) => println!("{}", serde_json::to_string_pretty(&t)?),
                None => {
                    eprintln!("tweet {id} not found (deleted, protected, or tombstoned)");
                    std::process::exit(1);
                }
            }
        }
        Op::Thread { target, page } => {
            let id = extract_tweet_id(&target)?;
            let result = paginate_tweets(page.all, page.max_pages, page.cursor.clone(), |cur| {
                let id = id.clone();
                Box::pin(async move {
                    client
                        .thread(&id, cur.as_deref(), page.quote_depth)
                        .await
                        .map_err(anyhow::Error::from)
                })
            })
            .await
            .context("thread failed")?;
            println!("{}", serde_json::to_string_pretty(&result)?);
        }
        Op::Replies { target, page } => {
            let id = extract_tweet_id(&target)?;
            let result = paginate_tweets(page.all, page.max_pages, page.cursor.clone(), |cur| {
                let id = id.clone();
                Box::pin(async move {
                    client
                        .thread(&id, cur.as_deref(), page.quote_depth)
                        .await
                        .map_err(anyhow::Error::from)
                })
            })
            .await
            .context("replies failed")?;
            // Keep only tweets that reply to the focal tweet.
            let replies: Vec<_> = result
                .tweets
                .into_iter()
                .filter(|t| t.in_reply_to_status_id.as_deref() == Some(id.as_str()))
                .collect();
            println!(
                "{}",
                serde_json::to_string_pretty(&serde_json::json!({
                    "tweets": replies,
                    "next_cursor": result.next_cursor,
                }))?
            );
        }
        Op::Search { query, top, page } => {
            let product = if top { "Top" } else { "Latest" }.to_owned();
            let result = paginate_tweets(page.all, page.max_pages, page.cursor.clone(), |cur| {
                let query = query.clone();
                let product = product.clone();
                Box::pin(async move {
                    client
                        .search(&query, page.count, cur.as_deref(), &product, page.quote_depth)
                        .await
                        .map_err(anyhow::Error::from)
                })
            })
            .await
            .context("search failed")?;
            println!("{}", serde_json::to_string_pretty(&result)?);
        }
        Op::UserTweets { handle, page } => {
            let uid = client
                .user_id_for(normalize_handle(&handle))
                .await
                .context("could not resolve handle to user id")?;
            let result = paginate_tweets(page.all, page.max_pages, page.cursor.clone(), |cur| {
                let uid = uid.clone();
                Box::pin(async move {
                    client
                        .user_tweets(&uid, page.count, cur.as_deref(), page.quote_depth)
                        .await
                        .map_err(anyhow::Error::from)
                })
            })
            .await
            .context("user_tweets failed")?;
            println!("{}", serde_json::to_string_pretty(&result)?);
        }
        Op::Home { following, page } => {
            let result = paginate_tweets(page.all, page.max_pages, page.cursor.clone(), |cur| {
                Box::pin(async move {
                    client
                        .home(page.count, cur.as_deref(), following, page.quote_depth)
                        .await
                        .map_err(anyhow::Error::from)
                })
            })
            .await
            .context("home failed")?;
            println!("{}", serde_json::to_string_pretty(&result)?);
        }
        Op::Likes { page } => {
            let uid = client.whoami().await.context("whoami failed")?.id;
            let result = paginate_tweets(page.all, page.max_pages, page.cursor.clone(), |cur| {
                let uid = uid.clone();
                Box::pin(async move {
                    client
                        .likes(&uid, page.count, cur.as_deref(), page.quote_depth)
                        .await
                        .map_err(anyhow::Error::from)
                })
            })
            .await
            .context("likes failed")?;
            println!("{}", serde_json::to_string_pretty(&result)?);
        }
        Op::Bookmarks { page } => {
            let result = paginate_tweets(page.all, page.max_pages, page.cursor.clone(), |cur| {
                Box::pin(async move {
                    client
                        .bookmarks(page.count, cur.as_deref(), page.quote_depth)
                        .await
                        .map_err(anyhow::Error::from)
                })
            })
            .await
            .context("bookmarks failed")?;
            println!("{}", serde_json::to_string_pretty(&result)?);
        }
        Op::Mentions { user, page } => {
            let handle = match user {
                Some(h) => normalize_handle(&h).to_owned(),
                None => client.whoami().await.context("whoami failed")?.screen_name,
            };
            let query = format!("(@{handle})");
            let result = paginate_tweets(page.all, page.max_pages, page.cursor.clone(), |cur| {
                let query = query.clone();
                Box::pin(async move {
                    client
                        .search(&query, page.count, cur.as_deref(), "Latest", page.quote_depth)
                        .await
                        .map_err(anyhow::Error::from)
                })
            })
            .await
            .context("mentions failed")?;
            println!("{}", serde_json::to_string_pretty(&result)?);
        }
        Op::Following { user, page } => {
            let uid = resolve_user_id(client, user.as_deref()).await?;
            let result = paginate_users(page.all, page.max_pages, page.cursor.clone(), |cur| {
                let uid = uid.clone();
                Box::pin(async move {
                    client
                        .following(&uid, page.count, cur.as_deref())
                        .await
                        .map_err(anyhow::Error::from)
                })
            })
            .await
            .context("following failed")?;
            println!("{}", serde_json::to_string_pretty(&result)?);
        }
        Op::Followers { user, page } => {
            let uid = resolve_user_id(client, user.as_deref()).await?;
            let result = paginate_users(page.all, page.max_pages, page.cursor.clone(), |cur| {
                let uid = uid.clone();
                Box::pin(async move {
                    client
                        .followers(&uid, page.count, cur.as_deref())
                        .await
                        .map_err(anyhow::Error::from)
                })
            })
            .await
            .context("followers failed")?;
            println!("{}", serde_json::to_string_pretty(&result)?);
        }
        Op::ListTimeline { target, page } => {
            let list_id = extract_list_id(&target)?;
            let result = paginate_tweets(page.all, page.max_pages, page.cursor.clone(), |cur| {
                let list_id = list_id.clone();
                Box::pin(async move {
                    client
                        .list_timeline(&list_id, page.count, cur.as_deref(), page.quote_depth)
                        .await
                        .map_err(anyhow::Error::from)
                })
            })
            .await
            .context("list_timeline failed")?;
            println!("{}", serde_json::to_string_pretty(&result)?);
        }
        Op::Whoami => {
            let me = client.whoami().await.context("whoami failed")?;
            println!("{}", serde_json::to_string_pretty(&me)?);
        }
        Op::Check => {
            let session_file = dirs::home_dir()
                .map(|h| h.join(".aphrody").join("x-session.json"))
                .filter(|p| p.exists());
            let out = serde_json::json!({
                "cookie_string_env": std::env::var("X_COOKIE_STRING").is_ok(),
                "session_file": session_file.map(|p| p.display().to_string()),
                "env_auth_token": std::env::var("X_AUTH_TOKEN").is_ok(),
                "env_ct0": std::env::var("X_CT0").is_ok(),
            });
            println!("{}", serde_json::to_string_pretty(&out)?);
        }
    }

    Ok(())
}

/// Resolve an optional handle to a user id, defaulting to the authenticated user.
async fn resolve_user_id(client: &XClient, handle: Option<&str>) -> Result<String> {
    match handle {
        Some(h) => client
            .user_id_for(normalize_handle(h))
            .await
            .with_context(|| format!("could not resolve @{h}")),
        None => Ok(client.whoami().await.context("whoami failed")?.id),
    }
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
