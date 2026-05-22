// SPDX-License-Identifier: Apache-2.0
//! X private API methods — GraphQL + REST v1.1.
//!
//! All methods live on `XClient`. They:
//! 1. Build the request body (pure function, unit-testable without network).
//! 2. POST / GET to the X private endpoint.
//! 3. Check HTTP status; on non-2xx, surface the X `errors[]` array via
//!    `XError::Api` so callers can distinguish error codes (32 = auth, 353 =
//!    transaction-id enforcement, 187 = duplicate tweet, etc.).
//! 4. Deserialise the successful JSON into a typed result struct.
//!
//! # QueryID stability
//!
//! X's private GraphQL endpoints include a `queryId` in the path and in the
//! request body. These IDs change with each deployment of x.com's JavaScript
//! bundle. The constants below were valid in May 2026. If you receive HTTP 404
//! on a GraphQL path, re-extract the current `queryId` from:
//!   https://x.com/sw.js (search for the operation name, e.g. "CreateTweet")
//! or from a live browser DevTools Network capture.

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use crate::client::{XClient, API_BASE};
use crate::{Result, XError};

// ---------------------------------------------------------------------------
// QueryID constants — re-extract from x.com/sw.js if 404s appear.
// ---------------------------------------------------------------------------

/// GraphQL queryId for `CreateTweet` (POST mutation).
/// Verified against x.com bundle, May 2026.
pub const CREATE_TWEET_QID: &str = "a1p9RWpkYKBjWv_I3WzS-A";

/// GraphQL queryId for `DeleteTweet` (POST mutation).
pub const DELETE_TWEET_QID: &str = "VaenaVgh5q5ih7kvyVjgtg";

/// GraphQL queryId for `FavoriteTweet` (like, POST mutation).
pub const FAVORITE_TWEET_QID: &str = "lI07N6Otwv1PhnEgXILM7A";

/// GraphQL queryId for `UnfavoriteTweet` (unlike, POST mutation).
pub const UNFAVORITE_TWEET_QID: &str = "ZYKSe-w7KEslx3JhSIk5LA";

/// GraphQL queryId for `CreateRetweet` (POST mutation).
pub const CREATE_RETWEET_QID: &str = "ojPdsZsimiJrUGLR1sjUtA";

/// GraphQL queryId for `DeleteRetweet` (unretweet, POST mutation).
pub const DELETE_RETWEET_QID: &str = "iQtK4dl5hBmXewYZuEOKVw";

/// GraphQL queryId for `UserByScreenName` (GET query).
pub const USER_BY_SCREEN_NAME_QID: &str = "NimuplG1OB7Fd2btCLdBOw";

/// GraphQL queryId for `HomeTimeline` (GET query).
pub const HOME_TIMELINE_QID: &str = "HCosVQA0txR8qAE3yCEwig";

// ---------------------------------------------------------------------------
// Feature flags sent with CreateTweet.
//
// X's GraphQL mutations require a `features` map that enables server-side
// rendering paths. The set evolves with bundle releases; sending an older
// subset is generally safe (unknown flags default to false on the server).
// This blob was captured from a live CreateTweet call in May 2026.
// ---------------------------------------------------------------------------

/// JSON-encoded feature flags blob for `CreateTweet`.
pub const CREATE_TWEET_FEATURES: &str = r#"{
    "interactive_text_enabled": true,
    "longform_notetweets_inline_media_enabled": true,
    "longform_notetweets_rich_text_read_enabled": true,
    "longform_notetweets_consumption_enabled": true,
    "tweet_awards_web_tipping_enabled": false,
    "freedom_of_speech_not_reach_fetch_enabled": true,
    "standardized_nudges_misinfo": true,
    "tweet_with_visibility_results_prefer_gql_limited_actions_policy_enabled": true,
    "rweb_video_timestamps_enabled": true,
    "longform_notetweets_prompts_enabled": true,
    "creator_subscriptions_tweet_preview_api_enabled": true,
    "c9s_tweet_anatomy_moderator_badge_enabled": true,
    "articles_preview_enabled": true,
    "rweb_tipjar_consumption_enabled": true,
    "responsive_web_graphql_exclude_directive_enabled": true,
    "verified_phone_label_enabled": false,
    "responsive_web_graphql_skip_user_profile_image_extensions_enabled": false,
    "responsive_web_graphql_timeline_navigation_enabled": true,
    "responsive_web_enhance_cards_enabled": false
}"#;

// ---------------------------------------------------------------------------
// Result structs (tolerant of missing fields via serde defaults).
// ---------------------------------------------------------------------------

/// Result of a successful tweet creation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TweetResult {
    /// Numeric tweet ID string (REST ID format, e.g. `"1234567890123456789"`).
    pub id: String,
    /// Full text of the created tweet.
    pub text: String,
}

/// Public user information returned by `UserByScreenName`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserInfo {
    /// Numeric user REST ID (e.g. `"2244994945"`).
    pub id: String,
    /// Display name (e.g. `"aphrody"`).
    pub name: String,
    /// Screen name / handle without `@` (e.g. `"aphrody_code"`).
    pub screen_name: String,
    /// Follower count (`None` if missing in response).
    #[serde(default)]
    pub followers_count: Option<u64>,
    /// Following count (`None` if missing in response).
    #[serde(default)]
    pub friends_count: Option<u64>,
}

/// A tweet entry from the home timeline.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimelineTweet {
    /// Numeric tweet REST ID.
    pub id: String,
    /// Full tweet text.
    pub text: String,
}

// ---------------------------------------------------------------------------
// Internal helper: surface X API errors from a JSON response body.
// ---------------------------------------------------------------------------

/// Extract the first X `errors[]` entry, if present, and return it as
/// `XError::Api`. Returns `Ok(body)` when no `errors` key is found.
fn check_api_errors(body: &Value) -> Result<()> {
    if let Some(errors) = body.get("errors").and_then(Value::as_array) {
        if let Some(first) = errors.first() {
            let code = first
                .get("code")
                .and_then(Value::as_i64)
                .unwrap_or(-1);
            let message = first
                .get("message")
                .and_then(Value::as_str)
                .unwrap_or("unknown error")
                .to_owned();
            return Err(XError::Api { code, message });
        }
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Pure body builders — unit-testable without network.
// ---------------------------------------------------------------------------

/// Build the JSON body for a `CreateTweet` GraphQL mutation.
///
/// Extracted as a pure function so unit tests can verify the structure
/// without constructing an `XClient` or hitting the network.
pub fn build_create_tweet_body(text: &str, reply_to: Option<&str>) -> Value {
    let features: Value = serde_json::from_str(CREATE_TWEET_FEATURES)
        .expect("CREATE_TWEET_FEATURES is valid JSON — bug in constant");

    let mut variables = json!({
        "tweet_text": text,
        "dark_request": false,
        "media": {
            "media_entities": [],
            "possibly_sensitive": false
        },
        "semantic_annotation_ids": []
    });

    if let Some(reply_id) = reply_to {
        variables["reply"] = json!({
            "in_reply_to_tweet_id": reply_id,
            "exclude_reply_user_ids": []
        });
    }

    json!({
        "variables": variables,
        "features": features,
        "queryId": CREATE_TWEET_QID
    })
}

// ---------------------------------------------------------------------------
// XClient API methods
// ---------------------------------------------------------------------------

impl XClient {
    // -----------------------------------------------------------------------
    // Tweets
    // -----------------------------------------------------------------------

    /// Post a new tweet, optionally as a reply to an existing tweet.
    ///
    /// # Arguments
    ///
    /// - `text` — tweet text (max 280 chars unless X Premium subscriber).
    /// - `reply_to` — numeric tweet ID to reply to, or `None` for a root tweet.
    ///
    /// # Errors
    ///
    /// - `XError::Api { code: 32 }` — authentication failure (stale cookies or
    ///   wrong `ct0` / bearer).
    /// - `XError::Api { code: 187 }` — duplicate tweet (same text posted twice
    ///   within the dedup window).
    /// - `XError::Api { code: 353 }` — `x-client-transaction-id` enforcement;
    ///   set `XSession::transaction_id` with a real value from a browser session.
    pub async fn create_tweet(
        &self,
        text: &str,
        reply_to: Option<&str>,
    ) -> Result<TweetResult> {
        let url = format!(
            "{}/graphql/{}/CreateTweet",
            API_BASE, CREATE_TWEET_QID
        );
        let body = build_create_tweet_body(text, reply_to);

        let resp = self
            .inner
            .post(&url)
            .json(&body)
            .send()
            .await?;

        let status = resp.status();
        let json: Value = resp.json().await?;

        if !status.is_success() {
            check_api_errors(&json)?;
            return Err(XError::Api {
                code: status.as_u16().into(),
                message: format!("HTTP {status}"),
            });
        }
        check_api_errors(&json)?;

        // Parse: data.create_tweet.tweet_results.result.rest_id + legacy.full_text
        let result = json
            .pointer("/data/create_tweet/tweet_results/result")
            .ok_or_else(|| XError::Api {
                code: -1,
                message: "CreateTweet response missing data.create_tweet.tweet_results.result"
                    .into(),
            })?;
        let id = result
            .get("rest_id")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_owned();
        let text = result
            .pointer("/legacy/full_text")
            .and_then(Value::as_str)
            .unwrap_or(text)
            .to_owned();

        Ok(TweetResult { id, text })
    }

    /// Delete a tweet by its numeric ID.
    pub async fn delete_tweet(&self, id: &str) -> Result<()> {
        let url = format!(
            "{}/graphql/{}/DeleteTweet",
            API_BASE, DELETE_TWEET_QID
        );
        let body = json!({
            "variables": {
                "tweet_id": id,
                "dark_request": false
            },
            "queryId": DELETE_TWEET_QID
        });

        let resp = self.inner.post(&url).json(&body).send().await?;
        let status = resp.status();
        let json: Value = resp.json().await?;
        if !status.is_success() {
            check_api_errors(&json)?;
            return Err(XError::Api {
                code: status.as_u16().into(),
                message: format!("HTTP {status}"),
            });
        }
        check_api_errors(&json)?;
        Ok(())
    }

    // -----------------------------------------------------------------------
    // Like / Unlike
    // -----------------------------------------------------------------------

    /// Like (favorite) a tweet.
    pub async fn like(&self, tweet_id: &str) -> Result<()> {
        let url = format!(
            "{}/graphql/{}/FavoriteTweet",
            API_BASE, FAVORITE_TWEET_QID
        );
        let body = json!({
            "variables": { "tweet_id": tweet_id },
            "queryId": FAVORITE_TWEET_QID
        });
        let resp = self.inner.post(&url).json(&body).send().await?;
        let status = resp.status();
        let json: Value = resp.json().await?;
        if !status.is_success() {
            check_api_errors(&json)?;
            return Err(XError::Api {
                code: status.as_u16().into(),
                message: format!("HTTP {status}"),
            });
        }
        check_api_errors(&json)?;
        Ok(())
    }

    /// Unlike (remove favorite) a tweet.
    pub async fn unlike(&self, tweet_id: &str) -> Result<()> {
        let url = format!(
            "{}/graphql/{}/UnfavoriteTweet",
            API_BASE, UNFAVORITE_TWEET_QID
        );
        let body = json!({
            "variables": { "tweet_id": tweet_id },
            "queryId": UNFAVORITE_TWEET_QID
        });
        let resp = self.inner.post(&url).json(&body).send().await?;
        let status = resp.status();
        let json: Value = resp.json().await?;
        if !status.is_success() {
            check_api_errors(&json)?;
            return Err(XError::Api {
                code: status.as_u16().into(),
                message: format!("HTTP {status}"),
            });
        }
        check_api_errors(&json)?;
        Ok(())
    }

    // -----------------------------------------------------------------------
    // Retweet / Unretweet
    // -----------------------------------------------------------------------

    /// Retweet a tweet.
    pub async fn retweet(&self, tweet_id: &str) -> Result<()> {
        let url = format!(
            "{}/graphql/{}/CreateRetweet",
            API_BASE, CREATE_RETWEET_QID
        );
        let body = json!({
            "variables": {
                "tweet_id": tweet_id,
                "dark_request": false
            },
            "queryId": CREATE_RETWEET_QID
        });
        let resp = self.inner.post(&url).json(&body).send().await?;
        let status = resp.status();
        let json: Value = resp.json().await?;
        if !status.is_success() {
            check_api_errors(&json)?;
            return Err(XError::Api {
                code: status.as_u16().into(),
                message: format!("HTTP {status}"),
            });
        }
        check_api_errors(&json)?;
        Ok(())
    }

    /// Remove a retweet.
    pub async fn unretweet(&self, tweet_id: &str) -> Result<()> {
        let url = format!(
            "{}/graphql/{}/DeleteRetweet",
            API_BASE, DELETE_RETWEET_QID
        );
        let body = json!({
            "variables": {
                "source_tweet_id": tweet_id,
                "dark_request": false
            },
            "queryId": DELETE_RETWEET_QID
        });
        let resp = self.inner.post(&url).json(&body).send().await?;
        let status = resp.status();
        let json: Value = resp.json().await?;
        if !status.is_success() {
            check_api_errors(&json)?;
            return Err(XError::Api {
                code: status.as_u16().into(),
                message: format!("HTTP {status}"),
            });
        }
        check_api_errors(&json)?;
        Ok(())
    }

    // -----------------------------------------------------------------------
    // Follow / Unfollow (REST v1.1)
    // -----------------------------------------------------------------------

    /// Follow a user by their numeric user ID.
    ///
    /// Uses the REST v1.1 `POST /friendships/create.json` endpoint with
    /// `application/x-www-form-urlencoded` body (not GraphQL).
    pub async fn follow(&self, user_id: &str) -> Result<()> {
        let url = format!("{}/1.1/friendships/create.json", API_BASE);
        let params = [("user_id", user_id)];
        let resp = self
            .inner
            .post(&url)
            .header("content-type", "application/x-www-form-urlencoded")
            .form(&params)
            .send()
            .await?;
        let status = resp.status();
        let json: Value = resp.json().await?;
        if !status.is_success() {
            check_api_errors(&json)?;
            return Err(XError::Api {
                code: status.as_u16().into(),
                message: format!("HTTP {status}"),
            });
        }
        check_api_errors(&json)?;
        Ok(())
    }

    /// Unfollow a user by their numeric user ID.
    ///
    /// Uses the REST v1.1 `POST /friendships/destroy.json` endpoint.
    pub async fn unfollow(&self, user_id: &str) -> Result<()> {
        let url = format!("{}/1.1/friendships/destroy.json", API_BASE);
        let params = [("user_id", user_id)];
        let resp = self
            .inner
            .post(&url)
            .header("content-type", "application/x-www-form-urlencoded")
            .form(&params)
            .send()
            .await?;
        let status = resp.status();
        let json: Value = resp.json().await?;
        if !status.is_success() {
            check_api_errors(&json)?;
            return Err(XError::Api {
                code: status.as_u16().into(),
                message: format!("HTTP {status}"),
            });
        }
        check_api_errors(&json)?;
        Ok(())
    }

    // -----------------------------------------------------------------------
    // User lookup
    // -----------------------------------------------------------------------

    /// Look up a user by screen name (handle, without `@`).
    ///
    /// Parses the GraphQL `UserByScreenName` response and returns a `UserInfo`
    /// struct. Follower and following counts are best-effort and may be `None`
    /// if the account has them hidden.
    pub async fn user_by_screen_name(&self, handle: &str) -> Result<UserInfo> {
        let variables = serde_json::to_string(&json!({
            "screen_name": handle,
            "withSafetyModeUserFields": true
        }))
        .unwrap();
        let features = json!({
            "hidden_profile_likes_enabled": true,
            "hidden_profile_subscriptions_enabled": true,
            "responsive_web_graphql_exclude_directive_enabled": true,
            "verified_phone_label_enabled": false,
            "responsive_web_graphql_skip_user_profile_image_extensions_enabled": false,
            "responsive_web_graphql_timeline_navigation_enabled": true
        });
        let features_str = serde_json::to_string(&features).unwrap();

        let url = format!(
            "{}/graphql/{}/UserByScreenName",
            API_BASE, USER_BY_SCREEN_NAME_QID
        );

        let resp = self
            .inner
            .get(&url)
            .query(&[("variables", &variables), ("features", &features_str)])
            .send()
            .await?;

        let status = resp.status();
        let json: Value = resp.json().await?;
        if !status.is_success() {
            check_api_errors(&json)?;
            return Err(XError::Api {
                code: status.as_u16().into(),
                message: format!("HTTP {status}"),
            });
        }
        check_api_errors(&json)?;

        // Parse: data.user.result.{rest_id, legacy.{name, screen_name, followers_count, friends_count}}
        let result = json
            .pointer("/data/user/result")
            .ok_or_else(|| XError::Api {
                code: -1,
                message: "UserByScreenName response missing data.user.result".into(),
            })?;
        let legacy = result.get("legacy").unwrap_or(&Value::Null);

        let id = result
            .get("rest_id")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_owned();
        let name = legacy
            .get("name")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_owned();
        let screen_name = legacy
            .get("screen_name")
            .and_then(Value::as_str)
            .unwrap_or(handle)
            .to_owned();
        let followers_count = legacy
            .get("followers_count")
            .and_then(Value::as_u64);
        let friends_count = legacy
            .get("friends_count")
            .and_then(Value::as_u64);

        Ok(UserInfo {
            id,
            name,
            screen_name,
            followers_count,
            friends_count,
        })
    }

    // -----------------------------------------------------------------------
    // Home timeline
    // -----------------------------------------------------------------------

    /// Fetch tweets from the authenticated user's home timeline.
    ///
    /// Returns up to `count` tweets. Timeline entries are heavily nested in
    /// X's GraphQL response; this parser is tolerant of missing fields and
    /// will silently skip entries it cannot decode.
    pub async fn home_timeline(&self, count: u32) -> Result<Vec<TimelineTweet>> {
        let variables = serde_json::to_string(&json!({
            "count": count,
            "includePromotedContent": false,
            "latestControlAvailable": true,
            "requestContext": "launch"
        }))
        .unwrap();
        let features = json!({
            "rweb_video_timestamps_enabled": true,
            "longform_notetweets_consumption_enabled": true,
            "responsive_web_graphql_exclude_directive_enabled": true,
            "tweet_with_visibility_results_prefer_gql_limited_actions_policy_enabled": true,
            "responsive_web_graphql_timeline_navigation_enabled": true
        });
        let features_str = serde_json::to_string(&features).unwrap();

        let url = format!(
            "{}/graphql/{}/HomeTimeline",
            API_BASE, HOME_TIMELINE_QID
        );

        let resp = self
            .inner
            .get(&url)
            .query(&[("variables", &variables), ("features", &features_str)])
            .send()
            .await?;

        let status = resp.status();
        let json: Value = resp.json().await?;
        if !status.is_success() {
            check_api_errors(&json)?;
            return Err(XError::Api {
                code: status.as_u16().into(),
                message: format!("HTTP {status}"),
            });
        }
        check_api_errors(&json)?;

        let mut tweets = Vec::new();

        // Walk: data.home.home_timeline_urt.instructions[].entries[].content
        //       .itemContent.tweet_results.result
        if let Some(instructions) = json
            .pointer("/data/home/home_timeline_urt/instructions")
            .and_then(Value::as_array)
        {
            for instruction in instructions {
                let Some(entries) = instruction.get("entries").and_then(Value::as_array) else {
                    continue;
                };
                for entry in entries {
                    let Some(result) = entry.pointer(
                        "/content/itemContent/tweet_results/result",
                    ) else {
                        continue;
                    };
                    let id = result
                        .get("rest_id")
                        .and_then(Value::as_str)
                        .unwrap_or_default()
                        .to_owned();
                    let text = result
                        .pointer("/legacy/full_text")
                        .and_then(Value::as_str)
                        .unwrap_or_default()
                        .to_owned();
                    if !id.is_empty() {
                        tweets.push(TimelineTweet { id, text });
                    }
                }
            }
        }

        Ok(tweets)
    }

    // -----------------------------------------------------------------------
    // Direct messages
    // -----------------------------------------------------------------------

    /// Send a direct message to a recipient by their numeric user ID.
    ///
    /// Uses the private REST v1.1 `POST /dm/new2.json` endpoint.
    ///
    /// # Rate limits
    ///
    /// X limits DMs per day. Exceeding the limit returns error code 226
    /// ("This request looks automated"). The limit is not publicly documented
    /// but is generally in the hundreds per day for normal accounts.
    pub async fn send_dm(&self, recipient_id: &str, text: &str) -> Result<()> {
        let url = format!("{}/1.1/dm/new2.json", API_BASE);
        let body = json!({
            "conversation_id": format!("{}-{}", recipient_id, recipient_id),
            "recipient_ids": false,
            "request_id": uuid_v4_hex(),
            "text": text,
            "cards_platform": "Web-12",
            "include_cards": 1,
            "include_quote_count": true,
            "dm_users": false
        });

        let resp = self.inner.post(&url).json(&body).send().await?;
        let status = resp.status();
        let json: Value = resp.json().await?;
        if !status.is_success() {
            check_api_errors(&json)?;
            return Err(XError::Api {
                code: status.as_u16().into(),
                message: format!("HTTP {status}"),
            });
        }
        check_api_errors(&json)?;
        Ok(())
    }
}

/// Generate a hex UUID-v4-like nonce for DM `request_id`.
///
/// Uses timestamp + process-ID + static salt for a collision-resistant
/// value that does not require an external RNG dependency.
fn uuid_v4_hex() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let ts = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    let pid = std::process::id() as u128;
    format!("{:032x}", ts ^ (pid << 64) ^ 0xdeadbeef_cafebabe)
}

// ---------------------------------------------------------------------------
// Unit tests
// ---------------------------------------------------------------------------
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn build_create_tweet_body_has_tweet_text() {
        let body = build_create_tweet_body("Hello world", None);
        assert_eq!(
            body.pointer("/variables/tweet_text")
                .and_then(Value::as_str),
            Some("Hello world")
        );
    }

    #[test]
    fn build_create_tweet_body_has_query_id() {
        let body = build_create_tweet_body("test", None);
        assert_eq!(
            body.get("queryId").and_then(Value::as_str),
            Some(CREATE_TWEET_QID)
        );
    }

    #[test]
    fn build_create_tweet_body_no_reply_when_none() {
        let body = build_create_tweet_body("test", None);
        assert!(
            body.pointer("/variables/reply").is_none(),
            "reply key must be absent when reply_to is None"
        );
    }

    #[test]
    fn build_create_tweet_body_with_reply_sets_in_reply_to() {
        let body = build_create_tweet_body("test reply", Some("1234567890123456789"));
        assert_eq!(
            body.pointer("/variables/reply/in_reply_to_tweet_id")
                .and_then(Value::as_str),
            Some("1234567890123456789")
        );
    }

    #[test]
    fn build_create_tweet_body_reply_has_exclude_user_ids() {
        let body = build_create_tweet_body("reply text", Some("999"));
        let exclude = body
            .pointer("/variables/reply/exclude_reply_user_ids")
            .and_then(Value::as_array)
            .expect("exclude_reply_user_ids must be an array");
        assert!(exclude.is_empty(), "exclude_reply_user_ids must be empty by default");
    }

    #[test]
    fn build_create_tweet_body_features_is_object() {
        let body = build_create_tweet_body("test", None);
        assert!(
            body.get("features").and_then(Value::as_object).is_some(),
            "features must be a JSON object"
        );
    }

    #[test]
    fn create_tweet_features_is_valid_json() {
        let v: Value = serde_json::from_str(CREATE_TWEET_FEATURES)
            .expect("CREATE_TWEET_FEATURES must be valid JSON");
        assert!(v.is_object());
    }

    #[test]
    fn check_api_errors_returns_ok_when_no_errors_key() {
        let body = json!({"data": {"foo": "bar"}});
        assert!(check_api_errors(&body).is_ok());
    }

    #[test]
    fn check_api_errors_surfaces_code_and_message() {
        let body = json!({
            "errors": [{"code": 32, "message": "Could not authenticate you"}]
        });
        let err = check_api_errors(&body).unwrap_err();
        match err {
            XError::Api { code, message } => {
                assert_eq!(code, 32);
                assert!(message.contains("authenticate"));
            }
            other => panic!("expected XError::Api, got {other:?}"),
        }
    }

    #[test]
    fn check_api_errors_empty_errors_array_is_ok() {
        let body = json!({"errors": []});
        assert!(check_api_errors(&body).is_ok());
    }
}
