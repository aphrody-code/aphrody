// SPDX-License-Identifier: Apache-2.0
//! A2A 1.0 [`AgentExecutor`] — routes `SendMessage` to native Claude / Grok / agy / bxc CLIs.

use std::{collections::HashMap, sync::Arc};

use a2a::{
    Artifact, Message, Part, Role, StreamResponse, Task, TaskState, TaskStatus, A2AError,
};
use a2a_server::{AgentExecutor, ExecutorContext};
use futures::stream::{self, BoxStream};
use serde_json::Value;

use crate::{
    envelope::{Envelope, EnvelopeKind},
    mailbox::Mailbox,
    manifest::AiManifest,
    peer::{PeerId, PeerInvoker},
};

/// Routes inbound A2A messages to [`PeerInvoker`] and mirrors activity into JSONL mailboxes.
pub struct CoordPeerExecutor {
    manifest: Arc<AiManifest>,
    invoker: PeerInvoker,
    mailbox: Mailbox,
    default_peer: PeerId,
}

impl CoordPeerExecutor {
    pub fn new(
        manifest: AiManifest,
        repo_root: std::path::PathBuf,
        dry_run: bool,
    ) -> Self {
        let coord_dir = manifest.coord_dir(&repo_root);
        let invoker = PeerInvoker::new(&repo_root).with_dry_run(dry_run);
        Self {
            manifest: Arc::new(manifest),
            invoker,
            mailbox: Mailbox::new(coord_dir),
            default_peer: PeerId::Grok,
        }
    }

    fn resolve_peer(&self, ctx: &ExecutorContext) -> PeerId {
        if let Some(meta) = &ctx.metadata {
            if let Some(p) = meta.get("aphrody_peer").or(meta.get("peer")) {
                if let Some(s) = p.as_str() {
                    if let Some(id) = PeerId::parse(s) {
                        return id;
                    }
                }
            }
        }
        if let Some(msg) = &ctx.message {
            if let Some(text) = msg.text() {
                if let Some(peer) = parse_peer_prefix(text) {
                    return peer;
                }
            }
        }
        self.default_peer
    }

    fn prompt_from_message(&self, ctx: &ExecutorContext) -> String {
        let Some(msg) = &ctx.message else {
            return String::new();
        };
        let text = msg.text().unwrap_or("").to_owned();
        let t = text.trim();
        if parse_peer_prefix(t).is_none() {
            return text;
        }
        if let Some(rest) = t.strip_prefix('@') {
            let after = rest.trim_start();
            if let Some((_, body)) = after.split_once(':') {
                return body.trim().to_owned();
            }
            if let Some(idx) = after.find(char::is_whitespace) {
                return after[idx..].trim().to_owned();
            }
            return String::new();
        }
        if let Some(rest) = t.strip_prefix("/peer ") {
            if let Some((_, body)) = rest.split_once(' ') {
                return body.trim().to_owned();
            }
        }
        text
    }

}

fn parse_peer_prefix(text: &str) -> Option<PeerId> {
    let t = text.trim();
    if let Some(rest) = t.strip_prefix('@') {
        let word = rest.split_whitespace().next()?;
        return PeerId::parse(word.trim_end_matches(':'));
    }
    if let Some(rest) = t.strip_prefix("/peer ") {
        let word = rest.split_whitespace().next()?;
        return PeerId::parse(word);
    }
    None
}

#[async_trait::async_trait]
impl AgentExecutor for CoordPeerExecutor {
    fn execute(
        &self,
        ctx: ExecutorContext,
    ) -> BoxStream<'static, Result<StreamResponse, A2AError>> {
        let peer = self.resolve_peer(&ctx);
        let prompt = self.prompt_from_message(&ctx);
        let invoker = self.invoker.clone();
        let manifest_id = self.manifest.id.clone();
        let coord_dir = self.mailbox.coord_dir.clone();
        let task_id = ctx.task_id.clone();
        let context_id = ctx.context_id.clone();
        let user_message = ctx.message.clone();

        Box::pin(stream::once(async move {
            let result = tokio::task::spawn_blocking(move || invoker.invoke_prompt(peer, &prompt))
                .await
                .map_err(|e| A2AError::internal(format!("peer task join: {e}")))?
                .map_err(|e| A2AError::internal(e.to_string()))?;

            let summary = format!(
                "peer={} exit={:?} stdout_bytes={}",
                result.peer.short(),
                result.exit_code,
                result.stdout.len()
            );
            let mailbox = Mailbox::new(coord_dir);
            let _ = mailbox.append(peer.short(), &Envelope::new(
                manifest_id,
                format!("{}@aphrody-code/{}", peer.short(), peer.short()),
                EnvelopeKind::Fact,
                format!("a2a task {task_id} done"),
                summary.clone(),
            ));

            let artifact = Artifact {
                artifact_id: a2a::new_artifact_id(),
                name: Some(format!("{}-output", peer.short())),
                description: Some("Native peer CLI stdout".to_owned()),
                parts: vec![Part::text(&result.stdout).with_media_type("text/plain")],
                metadata: Some(HashMap::from([
                    ("stderr".to_owned(), Value::String(result.stderr)),
                    ("exit_code".to_owned(), Value::from(result.exit_code)),
                ])),
                extensions: None,
            };

            let state = if result.exit_code == Some(0) {
                TaskState::Completed
            } else {
                TaskState::Failed
            };

            let agent_note = Message::new(
                Role::Agent,
                vec![Part::text(format!(
                    "Invoked {} (exit {:?}). See artifact for stdout.",
                    peer.short(),
                    result.exit_code
                ))],
            );

            let task = Task {
                id: task_id,
                context_id,
                status: TaskStatus {
                    state,
                    message: Some(agent_note),
                    timestamp: None,
                },
                artifacts: Some(vec![artifact]),
                history: user_message.map(|m| vec![m]),
                metadata: Some(HashMap::from([(
                    "aphrody_peer".to_owned(),
                    Value::String(peer.short().to_owned()),
                )])),
            };

            Ok(StreamResponse::Task(task))
        }))
    }

    fn cancel(
        &self,
        ctx: ExecutorContext,
    ) -> BoxStream<'static, Result<StreamResponse, A2AError>> {
        let task = Task {
            id: ctx.task_id.clone(),
            context_id: ctx.context_id.clone(),
            status: TaskStatus {
                state: TaskState::Canceled,
                message: None,
                timestamp: None,
            },
            artifacts: None,
            history: None,
            metadata: None,
        };
        Box::pin(stream::once(async move { Ok(StreamResponse::Task(task)) }))
    }
}