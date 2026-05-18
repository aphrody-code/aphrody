// SPDX-License-Identifier: Apache-2.0
//! Integration tests for aphrody-terminal-llm.

use std::sync::Arc;

use aphrody_terminal_llm::{
    EventBus, HookEntry, HookEventLog, LlmEvent, McpServerSpec, McpStatusRegistry, McpTransport,
    OAuthConfig, SkillSlot, SkillState, SubAgentRegistry, TaskTree, load_mcp_json,
    parse_aphrody_llm_osc, probe_loop,
};
use base64::{Engine as _, engine::general_purpose::STANDARD as B64};

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn osc(op: &str, payload: &str) -> Vec<u8> {
    format!("\x1b]aphrody-{op};{payload}\x07").into_bytes()
}

fn osc_b64(op: &str, data: &str) -> Vec<u8> {
    osc(op, &B64.encode(data.as_bytes()))
}

// ---------------------------------------------------------------------------
// 1. EventBus: single publisher, single subscriber
// ---------------------------------------------------------------------------

#[tokio::test]
async fn event_bus_publish_subscribe_roundtrip() {
    let bus = EventBus::new(16);
    let mut rx = bus.subscribe();

    let ev = LlmEvent::Markdown { body: "# Hello".into() };
    let sent = bus.publish(ev).unwrap();
    assert_eq!(sent, 1);

    let received = rx.recv().await.unwrap();
    match received {
        LlmEvent::Markdown { body } => assert_eq!(body, "# Hello"),
        other => panic!("unexpected event: {other:?}"),
    }
}

// ---------------------------------------------------------------------------
// 2. EventBus: two subscribers both receive
// ---------------------------------------------------------------------------

#[tokio::test]
async fn event_bus_multi_subscriber() {
    let bus = EventBus::new(16);
    let mut rx1 = bus.subscribe();
    let mut rx2 = bus.subscribe();

    bus.publish(LlmEvent::Task {
        id: "t1".into(),
        status: "pending".into(),
        subject: "bootstrap".into(),
    })
    .unwrap();

    let e1 = rx1.recv().await.unwrap();
    let e2 = rx2.recv().await.unwrap();

    for ev in [e1, e2] {
        match ev {
            LlmEvent::Task { id, .. } => assert_eq!(id, "t1"),
            other => panic!("unexpected: {other:?}"),
        }
    }
}

// ---------------------------------------------------------------------------
// 3. OSC: parse markdown base64
// ---------------------------------------------------------------------------

#[test]
fn osc_parse_markdown_base64() {
    let raw = osc_b64("md", "# Hi");
    let ev = parse_aphrody_llm_osc(&raw).expect("should parse");
    match ev {
        LlmEvent::Markdown { body } => assert_eq!(body, "# Hi"),
        other => panic!("unexpected: {other:?}"),
    }
}

// ---------------------------------------------------------------------------
// 4. OSC: parse JSON inspect
// ---------------------------------------------------------------------------

#[test]
fn osc_parse_json_inspect() {
    let json = r#"{"key":"value","num":42}"#;
    let raw = osc_b64("json", json);
    let ev = parse_aphrody_llm_osc(&raw).expect("should parse");
    match ev {
        LlmEvent::JsonInspect { payload } => {
            assert_eq!(payload["key"].as_str().unwrap(), "value");
            assert_eq!(payload["num"].as_i64().unwrap(), 42);
        },
        other => panic!("unexpected: {other:?}"),
    }
}

// ---------------------------------------------------------------------------
// 5. OSC: parse sub-agent fields
// ---------------------------------------------------------------------------

#[test]
fn osc_parse_sub_agent_fields() {
    let raw = osc("sub-agent", "a1;done;ok");
    let ev = parse_aphrody_llm_osc(&raw).expect("should parse");
    match ev {
        LlmEvent::SubAgent { id, status, text } => {
            assert_eq!(id, "a1");
            assert_eq!(status, "done");
            assert_eq!(text, "ok");
        },
        other => panic!("unexpected: {other:?}"),
    }
}

// ---------------------------------------------------------------------------
// 6. OSC: parse MCP with and without optional rpc field
// ---------------------------------------------------------------------------

#[test]
fn osc_parse_mcp_with_optional_rpc() {
    // With rpc.
    let raw = osc("mcp", "my-server;connected;tools/list");
    let ev = parse_aphrody_llm_osc(&raw).expect("should parse with rpc");
    match ev {
        LlmEvent::Mcp { server, state, rpc } => {
            assert_eq!(server, "my-server");
            assert_eq!(state, "connected");
            assert_eq!(rpc.as_deref(), Some("tools/list"));
        },
        other => panic!("unexpected: {other:?}"),
    }

    // Without rpc.
    let raw2 = osc("mcp", "my-server;disconnected");
    let ev2 = parse_aphrody_llm_osc(&raw2).expect("should parse without rpc");
    match ev2 {
        LlmEvent::Mcp { rpc, .. } => assert!(rpc.is_none()),
        other => panic!("unexpected: {other:?}"),
    }
}

// ---------------------------------------------------------------------------
// 7. OSC: invalid base64 in hook payload returns None
// ---------------------------------------------------------------------------

#[test]
fn osc_parse_hook_invalid_base64_returns_none() {
    // "!!!" is not valid base64.
    let raw = osc("hook", "!!!");
    let result = parse_aphrody_llm_osc(&raw);
    assert!(result.is_none(), "invalid base64 must return None");
}

// ---------------------------------------------------------------------------
// 8. SubAgentRegistry: update and snapshot
// ---------------------------------------------------------------------------

#[test]
fn sub_agent_registry_update_and_snapshot() {
    let reg = SubAgentRegistry::new(16);
    reg.update("sa-1", "running", "starting up");
    reg.update("sa-2", "done", "finished");
    reg.update("sa-1", "done", "all clear"); // overwrite sa-1

    let mut agents = reg.list();
    agents.sort_by(|a, b| a.id.cmp(&b.id));

    assert_eq!(agents.len(), 2);
    assert_eq!(agents[0].id, "sa-1");
    assert_eq!(agents[0].status, "done");
    assert_eq!(agents[0].last_text, "all clear");
    assert_eq!(agents[1].id, "sa-2");
}

// ---------------------------------------------------------------------------
// 9. TaskTree: parent/child resolution
// ---------------------------------------------------------------------------

#[test]
fn task_tree_parent_child_resolution() {
    let tree = TaskTree::new();

    // Insert parent first.
    tree.update("root", None, "in_progress", "root task");
    // Insert child with parent reference.
    tree.update("child-1", Some("root".into()), "pending", "child task");

    let root = tree.get("root").expect("root must exist");
    assert!(root.children.contains(&"child-1".to_owned()));
    assert!(root.parent.is_none());

    let child = tree.get("child-1").expect("child must exist");
    assert_eq!(child.parent.as_deref(), Some("root"));

    let roots = tree.roots();
    assert!(roots.contains(&"root".to_owned()));
    assert!(!roots.contains(&"child-1".to_owned()));
}

// ---------------------------------------------------------------------------
// 10. HookEventLog: capacity eviction
// ---------------------------------------------------------------------------

#[test]
fn hook_event_log_capacity_eviction() {
    let log = HookEventLog::new(3);

    for i in 0..4_u32 {
        log.push(HookEntry { event: format!("event-{i}"), payload: serde_json::Value::Null });
    }

    // Capacity is 3, so the oldest (event-0) should be gone.
    let entries = log.iter();
    assert_eq!(entries.len(), 3);
    assert!(entries.iter().all(|e| e.event != "event-0"));
    assert!(entries.iter().any(|e| e.event == "event-3"));
}

// ---------------------------------------------------------------------------
// 11. SkillSlot: activate/deactivate state transitions
// ---------------------------------------------------------------------------

#[test]
fn skill_slot_activate_deactivate() {
    let slots = SkillSlot::new();
    slots.activate("my-skill", "start", serde_json::json!({}));

    let list = slots.list();
    let entry = list.iter().find(|e| e.name == "my-skill").unwrap();
    assert_eq!(entry.state, SkillState::Active);

    slots.deactivate("my-skill");
    let list2 = slots.list();
    let entry2 = list2.iter().find(|e| e.name == "my-skill").unwrap();
    assert_eq!(entry2.state, SkillState::Idle);
}

// ---------------------------------------------------------------------------
// 12. McpStatusRegistry: update and snapshot
// ---------------------------------------------------------------------------

#[test]
fn mcp_registry_update_and_snapshot() {
    let reg = McpStatusRegistry::new();
    reg.update("context7", "connected", Some("resolve-library-id".into()));
    reg.update("filesystem", "disconnected", None);
    reg.update("context7", "idle", None); // overwrite

    let mut servers = reg.list();
    servers.sort_by(|a, b| a.server.cmp(&b.server));

    assert_eq!(servers.len(), 2);
    assert_eq!(servers[0].server, "context7");
    assert_eq!(servers[0].state, "idle");
    assert!(servers[0].last_rpc.is_none());
    assert_eq!(servers[1].server, "filesystem");
}

// ---------------------------------------------------------------------------
// 13. OSC: ST terminator (\x1b\\) is accepted
// ---------------------------------------------------------------------------

#[test]
fn osc_parse_st_terminator() {
    let raw = format!("\x1b]aphrody-md;{}\x1b\\", B64.encode("## ST terminated".as_bytes()));
    let ev = parse_aphrody_llm_osc(raw.as_bytes()).expect("ST terminator must be accepted");
    match ev {
        LlmEvent::Markdown { body } => assert_eq!(body, "## ST terminated"),
        other => panic!("unexpected: {other:?}"),
    }
}

// ---------------------------------------------------------------------------
// 14. McpServerSpec: serde roundtrip (stdio + http variants)
// ---------------------------------------------------------------------------

#[test]
fn mcp_server_spec_serde_roundtrip() {
    // Stdio variant.
    let stdio_spec = McpServerSpec {
        name: "my-stdio-server".to_owned(),
        transport: McpTransport::Stdio {
            command: "my-mcp-binary".to_owned(),
            args: vec!["--stdio".to_owned()],
            env: [("DEBUG".to_owned(), "1".to_owned())].into_iter().collect(),
        },
    };
    let json = serde_json::to_string(&stdio_spec).expect("serialize stdio spec");
    let back: McpServerSpec = serde_json::from_str(&json).expect("deserialize stdio spec");
    assert_eq!(back.name, "my-stdio-server");
    match &back.transport {
        McpTransport::Stdio { command, args, env } => {
            assert_eq!(command, "my-mcp-binary");
            assert_eq!(args, &["--stdio"]);
            assert_eq!(env.get("DEBUG").map(String::as_str), Some("1"));
        },
        other => panic!("unexpected transport after roundtrip: {other:?}"),
    }

    // Http variant with OAuth config.
    let http_spec = McpServerSpec {
        name: "my-http-server".to_owned(),
        transport: McpTransport::Http {
            url: "https://mcp.example.com/".to_owned(),
            oauth_provider: Some(OAuthConfig {
                server_url: "https://mcp.example.com/".to_owned(),
                redirect_uri: "http://localhost:8080/callback".to_owned(),
                scope: Some("read write".to_owned()),
                token_env_var: Some("MCP_TOKEN".to_owned()),
            }),
        },
    };
    let json2 = serde_json::to_string(&http_spec).expect("serialize http spec");
    let back2: McpServerSpec = serde_json::from_str(&json2).expect("deserialize http spec");
    assert_eq!(back2.name, "my-http-server");
    match &back2.transport {
        McpTransport::Http { url, oauth_provider } => {
            assert_eq!(url, "https://mcp.example.com/");
            let oauth = oauth_provider.as_ref().expect("oauth_provider must be Some");
            assert_eq!(oauth.token_env_var.as_deref(), Some("MCP_TOKEN"));
        },
        other => panic!("unexpected transport after roundtrip: {other:?}"),
    }
}

// ---------------------------------------------------------------------------
// 15. probe_loop: publishes LlmEvent::Mcp when state changes Uses a mock probe by wiring a spec
//     whose command is guaranteed to fail immediately ("__nonexistent_cmd__"), producing state
//     "error". The loop must publish the transition from nothing → "error".
// ---------------------------------------------------------------------------

#[tokio::test]
async fn probe_loop_publishes_event_on_state_change() {
    let registry = Arc::new(McpStatusRegistry::new());
    let bus = Arc::new(EventBus::new(16));
    let mut rx = bus.subscribe();

    // Use a deliberately nonexistent command so the probe returns "error" fast.
    let specs = vec![McpServerSpec {
        name: "probe-test-server".to_owned(),
        transport: McpTransport::Stdio {
            command: "__nonexistent_cmd_aphrody_test__".to_owned(),
            args: vec![],
            env: Default::default(),
        },
    }];

    // Run probe_loop for at most one iteration then cancel it.
    let registry_clone = Arc::clone(&registry);
    let bus_clone = Arc::clone(&bus);
    let handle = tokio::spawn(probe_loop(
        registry_clone,
        specs,
        std::time::Duration::from_secs(60), // long interval — we cancel after 1st event
        bus_clone,
    ));

    // Wait up to 5 seconds for at least one Mcp event.
    let received = tokio::time::timeout(std::time::Duration::from_secs(5), rx.recv()).await;
    handle.abort(); // cancel the loop

    let ev = received
        .expect("should receive event within 5 s")
        .expect("broadcast channel must not be closed");

    match ev {
        LlmEvent::Mcp { server, state, .. } => {
            assert_eq!(server, "probe-test-server");
            // Nonexistent command → either "error" or "timeout" depending on OS.
            assert!(
                state == "error" || state == "timeout",
                "expected error or timeout state, got {state:?}"
            );
        },
        other => panic!("unexpected event kind: {other:?}"),
    }

    // Confirm the registry was updated.
    let status = registry
        .get("probe-test-server")
        .expect("registry must contain probe-test-server after probe");
    assert!(
        status.state == "error" || status.state == "timeout",
        "registry state must be error or timeout, got {:?}",
        status.state
    );
}

// ---------------------------------------------------------------------------
// 16. load_mcp_json: parses a .mcp.json-compatible file
// ---------------------------------------------------------------------------

#[test]
fn load_mcp_json_compat() {
    // Write a temp file matching the repo .mcp.json format.
    let dir = tempfile::tempdir().expect("tempdir");
    let path = dir.path().join("mcp.json");

    std::fs::write(
        &path,
        r#"{
  "$schema": "https://code.claude.com/schema/mcp.json",
  "mcpServers": {
    "bxc": {
      "type": "stdio",
      "command": "bun",
      "args": ["run", "C:/worktree/bxc/packages/bxc-extension/server.ts"],
      "env": { "BXC_MEMORY_DB": "C:/src/aphrody/var/data/bxc-memory.sqlite" }
    },
    "github": {
      "type": "streamable-http",
      "url": "https://api.githubcopilot.com/mcp/",
      "headers": { "Authorization": "Bearer ${GITHUB_PERSONAL_ACCESS_TOKEN}" }
    }
  }
}"#,
    )
    .expect("write test mcp.json");

    let specs = load_mcp_json(&path).expect("load_mcp_json must succeed");
    assert_eq!(specs.len(), 2);

    // bxc spec must be Stdio.
    let bxc = specs.iter().find(|s| s.name == "bxc").expect("bxc spec must be present");
    match &bxc.transport {
        McpTransport::Stdio { command, args, env } => {
            assert_eq!(command, "bun");
            assert_eq!(&args[..], &["run", "C:/worktree/bxc/packages/bxc-extension/server.ts"]);
            assert_eq!(
                env.get("BXC_MEMORY_DB").map(String::as_str),
                Some("C:/src/aphrody/var/data/bxc-memory.sqlite")
            );
        },
        other => panic!("bxc transport must be Stdio, got {other:?}"),
    }

    // github spec must be Http; oauth_provider must carry the env var name.
    let gh = specs.iter().find(|s| s.name == "github").expect("github spec must be present");
    match &gh.transport {
        McpTransport::Http { url, oauth_provider } => {
            assert_eq!(url, "https://api.githubcopilot.com/mcp/");
            let oauth = oauth_provider.as_ref().expect("github must have oauth_provider");
            assert_eq!(oauth.token_env_var.as_deref(), Some("GITHUB_PERSONAL_ACCESS_TOKEN"));
        },
        other => panic!("github transport must be Http, got {other:?}"),
    }
}
