// SPDX-License-Identifier: Apache-2.0
use std::{net::SocketAddr, sync::Arc};

use a2a::AgentCapabilities;
use a2a_server::{
    DefaultRequestHandler, InMemoryTaskStore, StaticAgentCard,
    agent_card::agent_card_router,
    jsonrpc::jsonrpc_router,
    rest::rest_router,
};

use crate::executor::CoordPeerExecutor;
use aphrody_terminal_backend::CoordProxy;
use axum::{
    Json, Router,
    extract::State,
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post},
};
use serde::Deserialize;
use tokio::net::TcpListener;
use tracing::info;

use crate::{
    card::agent_card_from_manifest,
    envelope::Envelope,
    mailbox::Mailbox,
    manifest::AiManifest,
};

#[derive(Debug, Clone)]
pub struct ListenerConfig {
    pub bind: SocketAddr,
    pub manifest: AiManifest,
    pub repo_root: std::path::PathBuf,
    /// When true, peer CLIs are not spawned (`APHRODY_A2A_DRY_RUN` env also enables this).
    pub dry_run: bool,
}

#[derive(Clone)]
struct AppState {
    mailbox: Mailbox,
}

/// Combined HTTP surface: coord (`/ping`, `/msg`, `/coord`) + A2A JSON-RPC + agent card.
pub fn router(cfg: &ListenerConfig) -> Router {
    let coord_dir = cfg.manifest.coord_dir(&cfg.repo_root);
    let mailbox = Mailbox::new(coord_dir.clone());
    let base = format!("http://{}", cfg.bind);
    let card = agent_card_from_manifest(&cfg.manifest, &base);
    let card_producer = Arc::new(StaticAgentCard::new(card));

    let dry_run = cfg.dry_run || std::env::var_os("APHRODY_A2A_DRY_RUN").is_some();
    let cap = cfg.manifest.capabilities.clone().unwrap_or_default();
    let capabilities = AgentCapabilities {
        streaming: Some(cap.streaming),
        push_notifications: Some(cap.push_notifications),
        extended_agent_card: Some(cap.extended_agent_card),
        extensions: None,
    };
    let executor = CoordPeerExecutor::new(cfg.manifest.clone(), cfg.repo_root.clone(), dry_run);
    let handler = Arc::new(
        DefaultRequestHandler::new(executor, InMemoryTaskStore::default())
            .with_capabilities(capabilities),
    );
    let a2a_rpc = jsonrpc_router(handler.clone());
    let a2a_rest = rest_router(handler);

    let state = AppState {
        mailbox,
    };

    let coord = Router::new()
        .route("/ping", get(handle_ping))
        .route("/msg", post(handle_msg))
        .route("/coord", get(handle_coord))
        .with_state(state);

    Router::new()
        .merge(coord)
        .merge(agent_card_router(card_producer))
        .merge(a2a_rpc)
        .merge(a2a_rest)
}

async fn handle_ping() -> impl IntoResponse {
    Json(serde_json::json!({
        "ok": true,
        "service": "aphrody-a2a-coord",
        "a2a_version": a2a::VERSION,
    }))
}

async fn handle_msg(
    State(state): State<AppState>,
    Json(env): Json<Envelope>,
) -> impl IntoResponse {
    let short = env
        .from
        .split('@')
        .next()
        .unwrap_or("unknown")
        .to_string();
    match state.mailbox.append(&short, &env) {
        Ok(()) => (StatusCode::OK, Json(serde_json::json!({ "ok": true, "id": env.id }))),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({ "error": e.to_string() })),
        ),
    }
}

async fn handle_coord(
    State(state): State<AppState>,
    headers: axum::http::HeaderMap,
) -> impl IntoResponse {
    let proxy = CoordProxy::new(state.mailbox.inbox_path("aphrody"));
    let accept = headers
        .get(axum::http::header::ACCEPT)
        .and_then(|v| v.to_str().ok());
    match proxy.handle_get_coord(accept).await {
        Ok(resp) => (
            StatusCode::from_u16(resp.status).unwrap_or(StatusCode::OK),
            [(axum::http::header::CONTENT_TYPE, resp.content_type)],
            axum::body::Body::from_stream(resp.body),
        )
            .into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({ "error": e.to_string() })),
        )
            .into_response(),
    }
}

/// Run the listener until shutdown.
pub async fn serve(cfg: ListenerConfig) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let app = router(&cfg);
    let listener = TcpListener::bind(cfg.bind).await?;
    info!(%cfg.bind, "aphrody A2A coord listener started");
    axum::serve(listener, app).await?;
    Ok(())
}

/// Duel-tick helper: compose + append envelope for a side.
#[derive(Debug, Deserialize)]
pub struct TickOptions {
    pub iteration: u64,
    pub side: String,
    pub peer: Option<String>,
    pub subject: Option<String>,
    pub body: Option<String>,
    pub kind: Option<String>,
}

pub fn run_tick(
    manifest: &AiManifest,
    repo_root: &std::path::Path,
    opts: &TickOptions,
) -> Result<Envelope, Box<dyn std::error::Error + Send + Sync>> {
    let mailbox = Mailbox::new(manifest.coord_dir(repo_root));
    mailbox.ensure_dir()?;
    let side = opts.side.as_str();
    let peer = opts.peer.as_deref().unwrap_or("winclean");
    let peer_last = mailbox.read_last(peer)?;
    let kind = opts
        .kind
        .as_deref()
        .unwrap_or("ping");
    let subject = opts
        .subject
        .clone()
        .unwrap_or_else(|| format!("coord tick {}", opts.iteration));
    let mut body = opts.body.clone().unwrap_or_else(|| {
        format!("Tick {} from {side}.", opts.iteration)
    });
    if let Some(ref last) = peer_last {
        body.push_str(&format!(" Last peer id={} kind={}.", last.id, last.kind));
    }
    let from = format!("{side}@aphrody-code/{side}");
    let to = format!("{peer}@aphrody-code/{peer}");
    let env = Envelope::new(from, to, parse_kind(kind), subject, body);
    mailbox.append(side, &env)?;
    mailbox.bump_heartbeat(side)?;
    Ok(env)
}

fn parse_kind(s: &str) -> crate::envelope::EnvelopeKind {
    match s {
        "ask" => crate::envelope::EnvelopeKind::Ask,
        "fact" => crate::envelope::EnvelopeKind::Fact,
        "ack" => crate::envelope::EnvelopeKind::Ack,
        "error" => crate::envelope::EnvelopeKind::Error,
        _ => crate::envelope::EnvelopeKind::Ping,
    }
}