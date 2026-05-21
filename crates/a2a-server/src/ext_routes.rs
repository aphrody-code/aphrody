// SPDX-License-Identifier: Apache-2.0
// Copyright AGNTCY Contributors (https://github.com/agntcy)
//
// Extension HTTP routes ported from
// `packages/gemini-cli/packages/a2a-server/src/http/app.ts`:
//
//   - `GET  /listCommands`            → command catalog
//   - `POST /executeCommand`          → run a registered command
//   - `GET  /tasks/{id}/metadata`     → per-task metadata wrapper
//   - `GET  /tasks/metadata`          → all-task metadata listing
//   - `POST /tasks`                   → create-only task constructor
//
// These extend the AGNTCY-spec `rest_router` (in `crate::rest`)
// with the gemini-cli operational surface. They are gated behind
// `with_command_registry` / `with_task_store_for_metadata` so the
// crate stays usable without them.

use std::{collections::HashMap, sync::Arc};

use a2a::{Task, TaskState, TaskStatus};
use axum::{
    Json, Router,
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post},
};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::{
    commands::{CommandDescriptor, CommandRegistry},
    task_store::TaskStore,
    workspace::workspace_override_configured,
};

/// Shared state for the extension routes.
pub struct ExtState {
    pub registry: Arc<CommandRegistry>,
    pub task_store: Arc<dyn TaskStore>,
}

impl Clone for ExtState {
    fn clone(&self) -> Self {
        ExtState { registry: self.registry.clone(), task_store: self.task_store.clone() }
    }
}

/// Build the extension router. Mount on the same `axum::Router::nest("/")`
/// as the AGNTCY `rest_router` to extend the surface.
pub fn ext_router(registry: Arc<CommandRegistry>, task_store: Arc<dyn TaskStore>) -> Router {
    let state = ExtState { registry, task_store };
    Router::new()
        .route("/listCommands", get(handle_list_commands))
        .route("/executeCommand", post(handle_execute_command))
        .route("/tasks", post(handle_create_task))
        .route("/tasks/metadata", get(handle_list_task_metadata))
        .route("/tasks/{id}/metadata", get(handle_get_task_metadata))
        .with_state(state)
}

#[derive(Serialize)]
struct ListCommandsResponse {
    commands: Vec<CommandDescriptor>,
}

async fn handle_list_commands(State(state): State<ExtState>) -> impl IntoResponse {
    let commands = state.registry.catalog().await;
    Json(ListCommandsResponse { commands })
}

#[derive(Deserialize)]
struct ExecuteCommandPayload {
    command: String,
    #[serde(default)]
    args: Vec<String>,
}

#[derive(Serialize)]
struct ErrorPayload {
    error: String,
}

async fn handle_execute_command(
    State(state): State<ExtState>,
    Json(payload): Json<ExecuteCommandPayload>,
) -> axum::response::Response {
    let Some(command) = state.registry.get(&payload.command).await else {
        return (
            StatusCode::NOT_FOUND,
            Json(ErrorPayload { error: format!("command not found: {}", payload.command) }),
        )
            .into_response();
    };

    if command.requires_workspace() && !workspace_override_configured() {
        return (
            StatusCode::BAD_REQUEST,
            Json(ErrorPayload {
                error: format!(
                    "command \"{}\" requires a workspace, but {} is not set",
                    payload.command,
                    crate::workspace::WORKSPACE_PATH_ENV
                ),
            }),
        )
            .into_response();
    }

    match command.execute(payload.args).await {
        Ok(result) => Json(result).into_response(),
        Err(err) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorPayload { error: err.message }),
        )
            .into_response(),
    }
}

#[derive(Deserialize, Default)]
struct CreateTaskPayload {
    #[serde(default)]
    context_id: Option<String>,
    #[serde(default)]
    metadata: Option<Value>,
}

#[derive(Serialize)]
struct CreateTaskResponse {
    id: String,
}

async fn handle_create_task(
    State(state): State<ExtState>,
    Json(payload): Json<CreateTaskPayload>,
) -> axum::response::Response {
    let task_id = uuid::Uuid::new_v4().to_string();
    let context_id =
        payload.context_id.unwrap_or_else(|| uuid::Uuid::new_v4().to_string());
    let metadata: Option<HashMap<String, Value>> = payload.metadata.and_then(|v| match v {
        Value::Object(map) => Some(map.into_iter().collect()),
        _ => None,
    });
    let task = Task {
        id: task_id.clone(),
        context_id,
        status: TaskStatus {
            state: TaskState::Submitted,
            message: None,
            timestamp: Some(chrono::Utc::now()),
        },
        artifacts: None,
        history: None,
        metadata,
    };

    match state.task_store.create(task).await {
        Ok(_) => (StatusCode::CREATED, Json(CreateTaskResponse { id: task_id })).into_response(),
        Err(err) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorPayload { error: err.message }),
        )
            .into_response(),
    }
}

#[derive(Serialize)]
struct TaskMetadataResponse<'a> {
    id: &'a str,
    context_id: &'a str,
    task_state: TaskState,
    metadata: Option<&'a HashMap<String, Value>>,
}

async fn handle_get_task_metadata(
    State(state): State<ExtState>,
    Path(id): Path<String>,
) -> axum::response::Response {
    match state.task_store.get(&id).await {
        Ok(Some(task)) => Json(TaskMetadataResponse {
            id: &task.id,
            context_id: &task.context_id,
            task_state: task.status.state,
            metadata: task.metadata.as_ref(),
        })
        .into_response(),
        Ok(None) => (
            StatusCode::NOT_FOUND,
            Json(ErrorPayload { error: format!("task not found: {id}") }),
        )
            .into_response(),
        Err(err) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorPayload { error: err.message }),
        )
            .into_response(),
    }
}

#[derive(Serialize)]
struct TaskMetadataEntry {
    id: String,
    context_id: String,
    task_state: TaskState,
    metadata: Option<HashMap<String, Value>>,
}

async fn handle_list_task_metadata(State(state): State<ExtState>) -> axum::response::Response {
    let req = a2a::ListTasksRequest {
        context_id: None,
        status: None,
        page_size: Some(1_000),
        page_token: None,
        history_length: Some(0),
        status_timestamp_after: None,
        include_artifacts: Some(false),
        tenant: None,
    };
    match state.task_store.list(&req).await {
        Ok(resp) => {
            if resp.tasks.is_empty() {
                return StatusCode::NO_CONTENT.into_response();
            }
            let entries: Vec<TaskMetadataEntry> = resp
                .tasks
                .into_iter()
                .map(|t| TaskMetadataEntry {
                    id: t.id,
                    context_id: t.context_id,
                    task_state: t.status.state,
                    metadata: t.metadata,
                })
                .collect();
            Json(entries).into_response()
        },
        Err(err) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorPayload { error: err.message }),
        )
            .into_response(),
    }
}

#[cfg(test)]
mod tests {
    use async_trait::async_trait;
    use axum::{body::Body, http::Request};
    use http_body_util::BodyExt;
    use serde_json::json;
    use tower::ServiceExt;

    use super::*;
    use crate::{
        commands::{Command, CommandExecutionResponse, CommandRegistry},
        task_store::InMemoryTaskStore,
    };

    struct Ping;

    #[async_trait]
    impl Command for Ping {
        fn name(&self) -> &str {
            "ping"
        }
        fn description(&self) -> &str {
            "Returns pong"
        }
        async fn execute(
            &self,
            _: Vec<String>,
        ) -> Result<CommandExecutionResponse, a2a::A2AError> {
            Ok(CommandExecutionResponse { name: "ping".into(), data: json!("pong") })
        }
    }

    struct WorkspaceRequired;

    #[async_trait]
    impl Command for WorkspaceRequired {
        fn name(&self) -> &str {
            "needs-workspace"
        }
        fn description(&self) -> &str {
            "Requires CODER_AGENT_WORKSPACE_PATH"
        }
        fn requires_workspace(&self) -> bool {
            true
        }
        async fn execute(
            &self,
            _: Vec<String>,
        ) -> Result<CommandExecutionResponse, a2a::A2AError> {
            Ok(CommandExecutionResponse { name: self.name().into(), data: json!("ok") })
        }
    }

    async fn make_app() -> Router {
        let registry = Arc::new(CommandRegistry::new());
        registry.register(Arc::new(Ping)).await;
        registry.register(Arc::new(WorkspaceRequired)).await;
        let store: Arc<dyn TaskStore> = Arc::new(InMemoryTaskStore::new());
        ext_router(registry, store)
    }

    #[tokio::test]
    async fn test_list_commands_returns_catalog() {
        let app = make_app().await;
        let req = Request::builder().uri("/listCommands").body(Body::empty()).unwrap();
        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        let body = resp.into_body().collect().await.unwrap().to_bytes();
        let payload: Value = serde_json::from_slice(&body).unwrap();
        let names: Vec<&str> = payload["commands"]
            .as_array()
            .unwrap()
            .iter()
            .map(|c| c["name"].as_str().unwrap())
            .collect();
        assert!(names.contains(&"ping"));
        assert!(names.contains(&"needs-workspace"));
    }

    #[tokio::test]
    async fn test_execute_known_command() {
        let app = make_app().await;
        let body = serde_json::to_vec(&json!({"command": "ping", "args": []})).unwrap();
        let req = Request::builder()
            .uri("/executeCommand")
            .method("POST")
            .header("content-type", "application/json")
            .body(Body::from(body))
            .unwrap();
        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        let body = resp.into_body().collect().await.unwrap().to_bytes();
        let payload: Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(payload["data"], json!("pong"));
    }

    #[tokio::test]
    async fn test_execute_unknown_command_404() {
        let app = make_app().await;
        let body = serde_json::to_vec(&json!({"command": "missing"})).unwrap();
        let req = Request::builder()
            .uri("/executeCommand")
            .method("POST")
            .header("content-type", "application/json")
            .body(Body::from(body))
            .unwrap();
        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn test_execute_workspace_required_without_env_400() {
        // SAFETY: see workspace tests for rationale.
        unsafe {
            std::env::remove_var(crate::workspace::WORKSPACE_PATH_ENV);
        }
        let app = make_app().await;
        let body = serde_json::to_vec(&json!({"command": "needs-workspace"})).unwrap();
        let req = Request::builder()
            .uri("/executeCommand")
            .method("POST")
            .header("content-type", "application/json")
            .body(Body::from(body))
            .unwrap();
        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn test_create_task_returns_201_and_id() {
        let app = make_app().await;
        let body = serde_json::to_vec(&json!({"contextId": "ctx-1"})).unwrap();
        let req = Request::builder()
            .uri("/tasks")
            .method("POST")
            .header("content-type", "application/json")
            .body(Body::from(body))
            .unwrap();
        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::CREATED);
        let body = resp.into_body().collect().await.unwrap().to_bytes();
        let payload: Value = serde_json::from_slice(&body).unwrap();
        assert!(payload["id"].as_str().unwrap().len() > 0);
    }

    #[tokio::test]
    async fn test_get_task_metadata_404_for_missing() {
        let app = make_app().await;
        let req = Request::builder()
            .uri("/tasks/missing/metadata")
            .body(Body::empty())
            .unwrap();
        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn test_list_task_metadata_204_when_empty() {
        let app = make_app().await;
        let req = Request::builder().uri("/tasks/metadata").body(Body::empty()).unwrap();
        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::NO_CONTENT);
    }
}
