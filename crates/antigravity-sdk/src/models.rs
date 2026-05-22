// SPDX-License-Identifier: Apache-2.0
//! Typed request / response models for the Cloud Code `v1internal` methods and
//! the Gemini `generateContent` endpoint.
//!
//! These mirror the realistic wire shapes observed against the Antigravity
//! 2.0.1 desktop client and the public Cloud Code "Private API" surface
//! (`cloudcode-pa.googleapis.com/v1internal:*`). They are deliberately
//! **unknown-field tolerant**: every struct omits `#[serde(deny_unknown_fields)]`
//! so the SDK keeps deserializing when Google adds fields, and optional /
//! forward-compatible members are modelled as [`Option`].
//!
//! All field names use `#[serde(rename_all = "camelCase")]` to match the JSON
//! produced by the gRPC-transcoded REST surface.
//!
//! # Examples
//!
//! ```
//! use antigravity_sdk::models::{LoadCodeAssistRequest, ClientMetadata};
//!
//! let req = LoadCodeAssistRequest {
//!     cloudaicompanion_project: Some("my-project".to_owned()),
//!     metadata: ClientMetadata {
//!         ide_type: Some("IDE_UNSPECIFIED".to_owned()),
//!         platform: Some("PLATFORM_UNSPECIFIED".to_owned()),
//!         plugin_type: Some("GEMINI".to_owned()),
//!     },
//! };
//! let json = serde_json::to_string(&req).unwrap();
//! assert!(json.contains("cloudaicompanionProject"));
//! assert!(json.contains("pluginType"));
//! ```

use serde::{Deserialize, Serialize};

// ---------------------------------------------------------------------------
// Shared client metadata
// ---------------------------------------------------------------------------

/// Client/environment metadata sent alongside Cloud Code `v1internal` calls.
///
/// The desktop client populates `ideType`, `platform` and `pluginType` with
/// gRPC enum *names* (e.g. `"VSCODE"`, `"DARWIN_AMD64"`, `"GEMINI"`). We keep
/// every field optional and stringly-typed so unknown enum variants round-trip
/// losslessly.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ClientMetadata {
    /// IDE family identifier (e.g. `"IDE_UNSPECIFIED"`, `"VSCODE"`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ide_type: Option<String>,

    /// Host platform identifier (e.g. `"PLATFORM_UNSPECIFIED"`, `"LINUX_AMD64"`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub platform: Option<String>,

    /// Plugin/product identifier (e.g. `"GEMINI"`, `"CLOUD_CODE"`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub plugin_type: Option<String>,
}

// ---------------------------------------------------------------------------
// loadCodeAssist
// ---------------------------------------------------------------------------

/// Request body for `POST /v1internal:loadCodeAssist`.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LoadCodeAssistRequest {
    /// Optional Cloud AI Companion project id. When omitted the server resolves
    /// the user's default / free-tier project.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cloudaicompanion_project: Option<String>,

    /// Client / environment metadata.
    pub metadata: ClientMetadata,
}

/// A subscription / entitlement tier returned by `loadCodeAssist`.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Tier {
    /// Stable tier identifier (e.g. `"free-tier"`, `"standard-tier"`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,

    /// Human-readable tier name.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,

    /// Optional tier description.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,

    /// Whether this tier is the user's default when none is selected.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_default: Option<bool>,

    /// Whether selecting this tier requires the caller to set a user-defined
    /// Cloud AI Companion project id.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user_defined_cloudaicompanion_project: Option<bool>,
}

/// Response body for `POST /v1internal:loadCodeAssist`.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LoadCodeAssistResponse {
    /// The tier the user is currently provisioned on, if any.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub current_tier: Option<Tier>,

    /// All tiers the user may onboard onto.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub allowed_tiers: Vec<Tier>,

    /// Resolved Cloud AI Companion project id, when the server assigned one.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cloudaicompanion_project: Option<String>,
}

// ---------------------------------------------------------------------------
// fetchAvailableModels
// ---------------------------------------------------------------------------

/// Request body for `POST /v1internal:fetchAvailableModels`.
///
/// The call is parameterless in practice; metadata is optional and forwarded
/// when present.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FetchAvailableModelsRequest {
    /// Optional client metadata.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<ClientMetadata>,
}

/// A single model entry returned by `fetchAvailableModels`.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ModelInfo {
    /// Canonical model name (e.g. `"gemini-2.0-flash"`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,

    /// Human-readable display name.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub display_name: Option<String>,

    /// Optional model description.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,

    /// Optional capability tags / flags (e.g. `["TEXT", "VISION"]`). Kept as raw
    /// JSON so server-side schema evolution does not break deserialization.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub capabilities: Option<serde_json::Value>,
}

/// Response body for `POST /v1internal:fetchAvailableModels`.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FetchAvailableModelsResponse {
    /// Models available to the user's account / tier.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub models: Vec<ModelInfo>,
}

// ---------------------------------------------------------------------------
// onboardUser
// ---------------------------------------------------------------------------

/// Request body for `POST /v1internal:onboardUser`.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OnboardUserRequest {
    /// Identifier of the tier to onboard the user onto.
    pub tier_id: String,

    /// Optional Cloud AI Companion project id (required by some user-defined
    /// tiers, see [`Tier::user_defined_cloudaicompanion_project`]).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cloudaicompanion_project: Option<String>,

    /// Client / environment metadata.
    pub metadata: ClientMetadata,
}

/// Response body for `POST /v1internal:onboardUser`.
///
/// `onboardUser` is a long-running operation: the server returns `done = false`
/// while provisioning and `done = true` (with a populated `name` /
/// `response`) once complete.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OnboardUserResponse {
    /// Operation resource name, when assigned.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,

    /// Whether the onboarding operation has completed.
    #[serde(default)]
    pub done: bool,

    /// Operation payload returned once `done` is true (raw JSON; typically
    /// echoes the provisioned tier).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub response: Option<serde_json::Value>,
}

// ---------------------------------------------------------------------------
// Gemini generateContent
// ---------------------------------------------------------------------------

/// A single part of a [`Content`] block. Currently only the `text` modality is
/// modelled; unknown part kinds round-trip via deserialization tolerance.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Part {
    /// Plain-text content of this part.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
}

impl Part {
    /// Convenience constructor for a text part.
    #[must_use]
    pub fn text(text: impl Into<String>) -> Self {
        Self {
            text: Some(text.into()),
        }
    }
}

/// A turn of conversation content: a role plus an ordered list of parts.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Content {
    /// Conversation role (`"user"` or `"model"`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub role: Option<String>,

    /// Ordered content parts.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub parts: Vec<Part>,
}

impl Content {
    /// Build a single-text-part `user` content turn.
    #[must_use]
    pub fn user_text(text: impl Into<String>) -> Self {
        Self {
            role: Some("user".to_owned()),
            parts: vec![Part::text(text)],
        }
    }
}

/// Request body for `POST /v1beta/models/{model}:generateContent`.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GenerateContentRequest {
    /// Ordered conversation turns.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub contents: Vec<Content>,

    /// Optional generation config (temperature, max tokens, …). Kept as raw
    /// JSON to avoid coupling to the full Gemini config schema.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub generation_config: Option<serde_json::Value>,

    /// Optional system instruction content.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub system_instruction: Option<Content>,
}

impl GenerateContentRequest {
    /// Build a request with a single user text prompt.
    #[must_use]
    pub fn from_prompt(prompt: impl Into<String>) -> Self {
        Self {
            contents: vec![Content::user_text(prompt)],
            generation_config: None,
            system_instruction: None,
        }
    }
}

/// A single candidate completion returned by `generateContent`.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Candidate {
    /// The generated content for this candidate.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content: Option<Content>,

    /// Reason generation stopped (e.g. `"STOP"`, `"MAX_TOKENS"`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub finish_reason: Option<String>,

    /// Zero-based index of this candidate.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub index: Option<u32>,
}

/// Response body for `POST /v1beta/models/{model}:generateContent`.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GenerateContentResponse {
    /// Generated candidates (usually one).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub candidates: Vec<Candidate>,

    /// Optional usage / token-count metadata (raw JSON).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub usage_metadata: Option<serde_json::Value>,

    /// Optional model version string echoed by the server.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model_version: Option<String>,
}

impl GenerateContentResponse {
    /// Concatenate the text of every part of the first candidate's content.
    ///
    /// Returns `None` when there is no candidate, no content, or no text parts.
    #[must_use]
    pub fn first_text(&self) -> Option<String> {
        let content = self.candidates.first()?.content.as_ref()?;
        let joined: String = content
            .parts
            .iter()
            .filter_map(|p| p.text.as_deref())
            .collect();
        if joined.is_empty() { None } else { Some(joined) }
    }
}

/// Request envelope for the Cloud Code `v1internal:generateContent` method —
/// the **modelbackend** path agy.exe uses. Wraps a standard
/// [`GenerateContentRequest`] with the resolved Code Assist `project` and the
/// bare `model` id.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CloudCodeGenerateContentRequest {
    /// Bare Gemini model id (e.g. `gemini-2.5-flash`).
    pub model: String,
    /// Cloud AI Companion project bound to the account (from `loadCodeAssist`).
    pub project: String,
    /// The wrapped generation request (contents, systemInstruction, …).
    pub request: GenerateContentRequest,
}

/// Response envelope for the Cloud Code `v1internal:generateContent` method.
/// The server nests the standard [`GenerateContentResponse`] under `response`
/// alongside `metadata` / `traceId` (ignored — structs are unknown-field
/// tolerant).
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CloudCodeGenerateContentResponse {
    /// The wrapped standard generate-content response.
    #[serde(default)]
    pub response: GenerateContentResponse,
}

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;

    /// Helper: serialize `value`, deserialize it back into `T`, re-serialize and
    /// assert both JSON encodings are byte-identical (structural round-trip).
    fn round_trip<T>(value: &T)
    where
        T: Serialize + serde::de::DeserializeOwned,
    {
        let json = serde_json::to_value(value).expect("serialize");
        let decoded: T = serde_json::from_value(json.clone()).expect("deserialize");
        let reencoded = serde_json::to_value(&decoded).expect("re-serialize");
        assert_eq!(json, reencoded, "round-trip mismatch");
    }

    #[test]
    fn load_code_assist_request_round_trips() {
        let req = LoadCodeAssistRequest {
            cloudaicompanion_project: Some("proj-123".to_owned()),
            metadata: ClientMetadata {
                ide_type: Some("VSCODE".to_owned()),
                platform: Some("LINUX_AMD64".to_owned()),
                plugin_type: Some("GEMINI".to_owned()),
            },
        };
        round_trip(&req);

        let json = serde_json::to_string(&req).unwrap();
        assert!(json.contains("\"cloudaicompanionProject\":\"proj-123\""));
        assert!(json.contains("\"ideType\":\"VSCODE\""));
        assert!(json.contains("\"pluginType\":\"GEMINI\""));
    }

    #[test]
    fn load_code_assist_response_parses_realistic_sample() {
        // Representative sample with extra unknown fields the server may add.
        let sample = r#"{
            "currentTier": {
                "id": "free-tier",
                "name": "Gemini Code Assist for individuals",
                "description": "Free tier",
                "isDefault": true,
                "userDefinedCloudaicompanionProject": false
            },
            "allowedTiers": [
                { "id": "free-tier", "name": "Free" },
                { "id": "standard-tier", "name": "Standard",
                  "userDefinedCloudaicompanionProject": true }
            ],
            "cloudaicompanionProject": "resolved-project-id",
            "futureUnknownField": { "nested": 42 }
        }"#;

        let resp: LoadCodeAssistResponse = serde_json::from_str(sample).expect("parse");
        assert_eq!(resp.current_tier.as_ref().unwrap().id.as_deref(), Some("free-tier"));
        assert_eq!(resp.current_tier.as_ref().unwrap().is_default, Some(true));
        assert_eq!(resp.allowed_tiers.len(), 2);
        assert_eq!(resp.allowed_tiers[1].id.as_deref(), Some("standard-tier"));
        assert_eq!(
            resp.allowed_tiers[1].user_defined_cloudaicompanion_project,
            Some(true)
        );
        assert_eq!(resp.cloudaicompanion_project.as_deref(), Some("resolved-project-id"));
        round_trip(&resp);
    }

    #[test]
    fn fetch_available_models_response_parses_sample() {
        let sample = r#"{
            "models": [
                {
                    "name": "gemini-2.0-flash",
                    "displayName": "Gemini 2.0 Flash",
                    "description": "Fast multimodal model",
                    "capabilities": ["TEXT", "VISION"]
                },
                {
                    "name": "gemini-2.0-pro"
                }
            ],
            "nextPageToken": "ignored-unknown-field"
        }"#;

        let resp: FetchAvailableModelsResponse = serde_json::from_str(sample).expect("parse");
        assert_eq!(resp.models.len(), 2);
        assert_eq!(resp.models[0].name.as_deref(), Some("gemini-2.0-flash"));
        assert_eq!(
            resp.models[0].display_name.as_deref(),
            Some("Gemini 2.0 Flash")
        );
        assert!(resp.models[0].capabilities.is_some());
        assert_eq!(resp.models[1].name.as_deref(), Some("gemini-2.0-pro"));
        assert!(resp.models[1].display_name.is_none());
        round_trip(&resp);
    }

    #[test]
    fn fetch_available_models_request_default_round_trips() {
        let req = FetchAvailableModelsRequest::default();
        round_trip(&req);
        // Empty optional metadata serializes to `{}`.
        assert_eq!(serde_json::to_string(&req).unwrap(), "{}");
    }

    #[test]
    fn onboard_user_round_trips() {
        let req = OnboardUserRequest {
            tier_id: "standard-tier".to_owned(),
            cloudaicompanion_project: Some("proj-abc".to_owned()),
            metadata: ClientMetadata {
                ide_type: Some("IDE_UNSPECIFIED".to_owned()),
                platform: Some("PLATFORM_UNSPECIFIED".to_owned()),
                plugin_type: Some("GEMINI".to_owned()),
            },
        };
        round_trip(&req);
        let json = serde_json::to_string(&req).unwrap();
        assert!(json.contains("\"tierId\":\"standard-tier\""));

        let resp_sample = r#"{
            "name": "operations/onboard/123",
            "done": true,
            "response": { "tier": { "id": "standard-tier" } },
            "metadata": { "ignored": true }
        }"#;
        let resp: OnboardUserResponse = serde_json::from_str(resp_sample).expect("parse");
        assert_eq!(resp.name.as_deref(), Some("operations/onboard/123"));
        assert!(resp.done);
        assert!(resp.response.is_some());
        round_trip(&resp);

        // `done` defaults to false when absent.
        let pending: OnboardUserResponse = serde_json::from_str("{}").expect("parse");
        assert!(!pending.done);
        assert!(pending.name.is_none());
    }

    #[test]
    fn generate_content_request_from_prompt() {
        let req = GenerateContentRequest::from_prompt("Hello, world");
        assert_eq!(req.contents.len(), 1);
        assert_eq!(req.contents[0].role.as_deref(), Some("user"));
        assert_eq!(req.contents[0].parts[0].text.as_deref(), Some("Hello, world"));
        round_trip(&req);

        let json = serde_json::to_string(&req).unwrap();
        assert!(json.contains("\"role\":\"user\""));
        assert!(json.contains("\"text\":\"Hello, world\""));
        // Empty optional fields are omitted.
        assert!(!json.contains("generationConfig"));
        assert!(!json.contains("systemInstruction"));
    }

    #[test]
    fn generate_content_response_parses_sample_and_extracts_text() {
        let sample = r#"{
            "candidates": [
                {
                    "content": {
                        "role": "model",
                        "parts": [ { "text": "Hello" }, { "text": ", world!" } ]
                    },
                    "finishReason": "STOP",
                    "index": 0,
                    "safetyRatings": []
                }
            ],
            "usageMetadata": { "totalTokenCount": 12 },
            "modelVersion": "gemini-2.0-flash-001"
        }"#;

        let resp: GenerateContentResponse = serde_json::from_str(sample).expect("parse");
        assert_eq!(resp.candidates.len(), 1);
        assert_eq!(resp.candidates[0].finish_reason.as_deref(), Some("STOP"));
        assert_eq!(resp.candidates[0].index, Some(0));
        assert_eq!(resp.first_text().as_deref(), Some("Hello, world!"));
        assert_eq!(resp.model_version.as_deref(), Some("gemini-2.0-flash-001"));
        round_trip(&resp);
    }

    #[test]
    fn generate_content_response_first_text_none_when_empty() {
        let resp = GenerateContentResponse::default();
        assert!(resp.first_text().is_none());
    }
}
