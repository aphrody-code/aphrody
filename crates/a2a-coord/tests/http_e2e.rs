// SPDX-License-Identifier: Apache-2.0
use std::net::{SocketAddr, TcpListener};

use a2a_coord::{
    Envelope, EnvelopeKind, ListenerConfig, agent_card_from_manifest, listener::router,
    manifest::AiManifest, mailbox::Mailbox,
};
use axum::body::Body;
use http_body_util::BodyExt;
use serde_json::json;
use tower::ServiceExt;

fn free_addr() -> SocketAddr {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    listener.local_addr().unwrap()
}

fn sample_manifest() -> AiManifest {
    serde_json::from_value(json!({
        "version": "1.0.0",
        "spec": "a2a/v1.0",
        "id": "aphrody@aphrody-code/aphrody",
        "schema_version": "1.0.0",
        "kind": "agent",
        "name": "aphrody-test",
        "a2a_protocol_version": "1.0",
        "capabilities": { "streaming": false, "pushNotifications": false },
        "default_input_modes": ["text/plain"],
        "default_output_modes": ["text/plain"],
        "coord": { "mailbox_dir": ".coord" }
    }))
    .unwrap()
}

#[tokio::test]
async fn ping_agent_card_and_msg_roundtrip() {
    let tmp = tempfile::tempdir().unwrap();
    std::fs::write(tmp.path().join("ai.json"), "{}").ok();
    let coord = tmp.path().join(".coord");
    std::fs::create_dir_all(&coord).unwrap();

    let mut manifest = sample_manifest();
    manifest.coord = Some(a2a_coord::manifest::CoordConfig {
        mailbox_dir: Some(".coord".into()),
        inbox_pattern: None,
        http_bind: None,
        envelope: None,
    });

    let bind = free_addr();
    let cfg = ListenerConfig {
        bind,
        manifest: manifest.clone(),
        repo_root: tmp.path().to_path_buf(),
        dry_run: false,
    };
    let app = router(&cfg);

    let ping = app
        .clone()
        .oneshot(
            axum::http::Request::builder()
                .uri("/ping")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(ping.status(), 200);

    let card = agent_card_from_manifest(&manifest, &format!("http://{bind}"));
    assert_eq!(card.name, "aphrody-test");

    let env = Envelope::new(
        "grok@aphrody-code/aphrody",
        "aphrody@aphrody-code/aphrody",
        EnvelopeKind::Ping,
        "e2e",
        "hello",
    );
    let msg = app
        .oneshot(
            axum::http::Request::builder()
                .method("POST")
                .uri("/msg")
                .header("content-type", "application/json")
                .body(Body::from(serde_json::to_string(&env).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(msg.status(), 200);

    let mailbox = Mailbox::new(coord);
    let last = mailbox.read_last("grok").unwrap().expect("envelope stored");
    assert_eq!(last.topic, "e2e");
}

#[tokio::test]
async fn jsonrpc_send_message_dry_run() {
    let tmp = tempfile::tempdir().unwrap();
    let coord = tmp.path().join(".coord");
    std::fs::create_dir_all(&coord).unwrap();

    let mut manifest = sample_manifest();
    manifest.coord = Some(a2a_coord::manifest::CoordConfig {
        mailbox_dir: Some(".coord".into()),
        inbox_pattern: None,
        http_bind: None,
        envelope: None,
    });

    let bind = free_addr();
    let cfg = a2a_coord::ListenerConfig {
        bind,
        manifest: manifest.clone(),
        repo_root: tmp.path().to_path_buf(),
        dry_run: true,
    };
    let app = a2a_coord::listener::router(&cfg);

    let body = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "SendMessage",
        "params": {
            "message": {
                "messageId": "m-e2e-1",
                "role": "ROLE_USER",
                "parts": [{ "text": "@grok: echo hello from e2e" }]
            },
            "metadata": { "aphrody_peer": "grok" }
        }
    });
    let resp = app
        .oneshot(
            axum::http::Request::builder()
                .method("POST")
                .uri("/")
                .header("content-type", "application/json")
                .body(Body::from(body.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);
    let bytes = resp.into_body().collect().await.unwrap().to_bytes();
    let v: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert!(v.get("result").is_some(), "expected result: {v}");
}