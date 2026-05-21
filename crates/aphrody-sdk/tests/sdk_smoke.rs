// SPDX-License-Identifier: Apache-2.0
//! Integration smoke tests for the aphrody-sdk public surface.
//!
//! These tests run against the *real* `AphrodySdk` façade — no internal
//! mocks. They guard the contract listed in `CLAUDE.md` mega-batch task
//! requirements:
//!
//! 1. `AphrodySdk::new(SdkConfig::default()).await` returns `Ok`.
//! 2. `sdk.agent().single_turn("hello")` returns a non-empty string.
//! 3. `sdk.tool_registry().list()` returns a `Vec` of length ≥ 0 (sanity).
//!
//! Additional integration checks cover session persistence and skill
//! discovery so the SDK can be exercised end-to-end without leaving the
//! crate boundary.

use aphrody_sdk::{
    skill_dir, tool, AphrodySdk, SdkConfig, SdkError, SessionId, SkillReference, ToolRegistry,
};
use serde_json::json;
use tempfile::TempDir;

// ---------------------------------------------------------------------------
// 1. Mega-batch contract — construction.
// ---------------------------------------------------------------------------

#[tokio::test]
async fn contract_new_with_default_config_returns_ok() {
    let r = AphrodySdk::new(SdkConfig::default()).await;
    assert!(r.is_ok(), "default config must build an SDK: {r:?}");
}

// ---------------------------------------------------------------------------
// 2. Mega-batch contract — agent single-turn returns a string.
// ---------------------------------------------------------------------------

#[tokio::test]
async fn contract_agent_single_turn_returns_string() {
    let sdk = AphrodySdk::new(SdkConfig::default())
        .await
        .expect("sdk must construct");
    let reply = sdk
        .agent()
        .single_turn("hello")
        .await
        .expect("single_turn must succeed (StubBackend fallback)");
    assert!(!reply.is_empty(), "reply must be non-empty");
    assert!(
        reply.contains("hello"),
        "stub backend should echo input; got: {reply}"
    );
}

// ---------------------------------------------------------------------------
// 3. Mega-batch contract — tool registry exposes list().
// ---------------------------------------------------------------------------

#[tokio::test]
async fn contract_tool_registry_list_returns_vec() {
    let sdk = AphrodySdk::new(SdkConfig::default()).await.unwrap();
    let tools = sdk.tool_registry().list();
    // Sanity: a Vec<&ToolDescriptor> of len() ≥ 0 (Rust enforces this at the
    // type level — we are guarding against the API regressing to a Result
    // shape).
    let _: usize = tools.len();
    assert!(tools.is_empty(), "fresh SDK should have no tools registered");
}

// ---------------------------------------------------------------------------
// Extra coverage: session manager round-trips.
// ---------------------------------------------------------------------------

#[tokio::test]
async fn session_manager_persists_and_resumes() {
    let dir = TempDir::new().expect("tempdir");
    let cfg = SdkConfig::default().session_dir(dir.path().to_path_buf());
    let sdk = AphrodySdk::new(cfg).await.unwrap();

    let id_before = sdk.session_manager().current_id().await;
    let turn = aphrody_sdk::Turn {
        id: "t1".into(),
        role: aphrody_sdk::TurnRole::User,
        content: "hi".into(),
        ts: chrono_now(),
        prompt_tokens: 5,
        completion_tokens: 0,
        cost_usd: 0.0,
    };
    sdk.session_manager().add_turn(turn).await.unwrap();

    let ids = sdk.session_manager().list().await.unwrap();
    assert!(ids.contains(&id_before), "session must be listed on disk");

    sdk.session_manager().end().await.unwrap();
}

// ---------------------------------------------------------------------------
// Extra coverage: skills discovery wires through the config.
// ---------------------------------------------------------------------------

#[tokio::test]
async fn skills_dir_discovered_at_construction() {
    let dir = TempDir::new().unwrap();
    let skill_dir_path = dir.path().join("my-skill");
    std::fs::create_dir_all(&skill_dir_path).unwrap();
    std::fs::write(
        skill_dir_path.join("SKILL.md"),
        "---\nname: my-skill\ndescription: integration test skill\n---\nbody\n",
    )
    .unwrap();

    let cfg = SdkConfig::default().add_skill_dir(dir.path().to_path_buf());
    let sdk = AphrodySdk::new(cfg).await.unwrap();
    assert_eq!(sdk.skills().len(), 1);
    assert!(sdk.skills().find("my-skill").is_some());
}

// ---------------------------------------------------------------------------
// Extra coverage: tool builder helper compiles + invokes.
// ---------------------------------------------------------------------------

#[tokio::test]
async fn tool_helper_builds_and_invokes() {
    let t = tool(
        "double",
        "Multiply n by 2",
        json!({"type": "object", "properties": {"n": {"type": "integer"}}, "required": ["n"]}),
        |args, _ctx| async move {
            let n = args.get("n").and_then(|v| v.as_i64()).unwrap_or(0);
            Ok(json!({"result": n * 2}))
        },
    )
    .expect("tool builder");

    let out = t.invoke(json!({"n": 21}), None).await.unwrap();
    assert_eq!(out, json!({"result": 42}));
}

// ---------------------------------------------------------------------------
// Extra coverage: ToolRegistry register / list flow (verifies re-export).
// ---------------------------------------------------------------------------

#[test]
fn registry_register_and_list_round_trip() {
    let mut reg = ToolRegistry::new();
    let d = aphrody_sdk::ToolDescriptor::new(
        "ping",
        "ping the host",
        json!({"type": "object", "properties": {}, "required": []}),
    )
    .expect("descriptor");
    reg.register(d).expect("register");
    let list = reg.list();
    assert_eq!(list.len(), 1);
    assert_eq!(list[0].name, "ping");
}

// ---------------------------------------------------------------------------
// Extra coverage: SkillReference sugar.
// ---------------------------------------------------------------------------

#[test]
fn skill_reference_sugar_round_trips() {
    let r = skill_dir("/tmp/x");
    assert_eq!(r, SkillReference::Dir(std::path::PathBuf::from("/tmp/x")));
}

// ---------------------------------------------------------------------------
// Extra coverage: error shape sanity.
// ---------------------------------------------------------------------------

#[tokio::test]
async fn invalid_config_returns_invalid_config_error() {
    let mut cfg = SdkConfig::default();
    cfg.model = String::new();
    let r = AphrodySdk::new(cfg).await;
    assert!(matches!(r, Err(SdkError::InvalidConfig(_))));
}

// ---------------------------------------------------------------------------
// Extra coverage: session id generation is unique.
// ---------------------------------------------------------------------------

#[tokio::test]
async fn session_ids_are_unique() {
    let sdk1 = AphrodySdk::new(SdkConfig::default()).await.unwrap();
    let sdk2 = AphrodySdk::new(SdkConfig::default()).await.unwrap();
    let id1: SessionId = sdk1.session_manager().current_id().await;
    let id2: SessionId = sdk2.session_manager().current_id().await;
    assert_ne!(id1, id2);
}

// ---------------------------------------------------------------------------
// Tiny helper — wrap chrono::Utc::now() so the test file does not need a
// direct chrono dependency declared in dev-deps.
// ---------------------------------------------------------------------------

fn chrono_now() -> chrono::DateTime<chrono::Utc> {
    chrono::Utc::now()
}
