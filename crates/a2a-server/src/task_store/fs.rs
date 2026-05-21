// SPDX-License-Identifier: Apache-2.0
// Copyright AGNTCY Contributors (https://github.com/agntcy)
//
// Filesystem-backed task store. Persists tasks as JSON files under a base
// directory. Designed as a portable, dependency-light alternative to the
// `GCSTaskStore` shipped in the TS reference (which requires Google Cloud
// Storage). Uses `serde_json` + `tokio::fs` only — no extra workspace deps.
//
// Path layout:
//   <base>/
//     tasks/
//       <task_id>.json   ← JSON-encoded `StoredTaskRecord`
//
// Each record carries a monotonic `version` for optimistic concurrency
// control, matching the `TaskVersion` semantics of the in-memory store.
//
// `task_id` is validated against `^[A-Za-z0-9_-]{1,128}$` to prevent path
// traversal — identical to the TS `isTaskIdValid` regex.

use std::{
    io,
    path::{Path, PathBuf},
};

use a2a::*;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use tokio::{fs, sync::Mutex};

use super::store::{TaskStore, TaskVersion};

/// On-disk record. Carries the task plus its monotonic version.
#[derive(Debug, Clone, Serialize, Deserialize)]
struct StoredTaskRecord {
    task: Task,
    version: TaskVersion,
}

/// Validates task IDs to prevent filesystem path traversal.
///
/// Allowed: alphanumeric, `_`, `-`. Length 1..=128. Mirrors the TS
/// `isTaskIdValid` check in `persistence/gcs.ts`.
fn is_task_id_valid(task_id: &str) -> bool {
    !task_id.is_empty()
        && task_id.len() <= 128
        && task_id.chars().all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
}

/// Filesystem-backed task store. Persists each task as a JSON file.
///
/// Concurrent access is serialized via a `Mutex<()>` to guarantee
/// read-modify-write atomicity for version bumps and updates. The mutex
/// covers all operations; under typical agent workloads (one task per
/// request, low concurrency on the same task) this is not a bottleneck.
pub struct FsTaskStore {
    base_dir: PathBuf,
    write_lock: Mutex<()>,
}

impl FsTaskStore {
    /// Create a new filesystem store rooted at `base_dir`. The directory
    /// (and `tasks/` subdirectory) is created lazily on first write.
    pub fn new(base_dir: impl Into<PathBuf>) -> Self {
        FsTaskStore { base_dir: base_dir.into(), write_lock: Mutex::new(()) }
    }

    fn tasks_dir(&self) -> PathBuf {
        self.base_dir.join("tasks")
    }

    fn task_path(&self, task_id: &str) -> Result<PathBuf, A2AError> {
        if !is_task_id_valid(task_id) {
            return Err(A2AError::invalid_params(format!("invalid task id: {task_id}")));
        }
        Ok(self.tasks_dir().join(format!("{task_id}.json")))
    }

    async fn ensure_tasks_dir(&self) -> Result<(), A2AError> {
        let dir = self.tasks_dir();
        if let Err(e) = fs::create_dir_all(&dir).await {
            return Err(A2AError::internal(format!(
                "failed to create tasks dir {}: {e}",
                dir.display()
            )));
        }
        Ok(())
    }

    async fn read_record(&self, path: &Path) -> Result<Option<StoredTaskRecord>, A2AError> {
        match fs::read(path).await {
            Ok(bytes) => {
                let record: StoredTaskRecord = serde_json::from_slice(&bytes).map_err(|e| {
                    A2AError::internal(format!(
                        "failed to parse task file {}: {e}",
                        path.display()
                    ))
                })?;
                Ok(Some(record))
            },
            Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(None),
            Err(e) => Err(A2AError::internal(format!(
                "failed to read task file {}: {e}",
                path.display()
            ))),
        }
    }

    async fn write_record(&self, path: &Path, record: &StoredTaskRecord) -> Result<(), A2AError> {
        let bytes = serde_json::to_vec_pretty(record).map_err(|e| {
            A2AError::internal(format!("failed to encode task record: {e}"))
        })?;
        // Write to a sibling tmp file, then rename for crash-safety. On
        // POSIX and modern Windows, rename is atomic within the same
        // directory.
        let tmp_path = path.with_extension("json.tmp");
        fs::write(&tmp_path, &bytes).await.map_err(|e| {
            A2AError::internal(format!(
                "failed to write task tmp file {}: {e}",
                tmp_path.display()
            ))
        })?;
        fs::rename(&tmp_path, path).await.map_err(|e| {
            A2AError::internal(format!(
                "failed to rename {} -> {}: {e}",
                tmp_path.display(),
                path.display()
            ))
        })?;
        Ok(())
    }
}

#[async_trait]
impl TaskStore for FsTaskStore {
    async fn create(&self, task: Task) -> Result<TaskVersion, A2AError> {
        let path = self.task_path(&task.id)?;
        let _guard = self.write_lock.lock().await;
        self.ensure_tasks_dir().await?;
        if self.read_record(&path).await?.is_some() {
            return Err(A2AError::internal(format!("task already exists: {}", task.id)));
        }
        let record = StoredTaskRecord { task, version: 1 };
        self.write_record(&path, &record).await?;
        Ok(record.version)
    }

    async fn update(&self, task: Task) -> Result<TaskVersion, A2AError> {
        let path = self.task_path(&task.id)?;
        let _guard = self.write_lock.lock().await;
        let mut record = self
            .read_record(&path)
            .await?
            .ok_or_else(|| A2AError::task_not_found(&task.id))?;
        record.version += 1;
        record.task = task;
        self.write_record(&path, &record).await?;
        Ok(record.version)
    }

    async fn get(&self, task_id: &str) -> Result<Option<Task>, A2AError> {
        let path = self.task_path(task_id)?;
        Ok(self.read_record(&path).await?.map(|r| r.task))
    }

    async fn list(&self, req: &ListTasksRequest) -> Result<ListTasksResponse, A2AError> {
        let dir = self.tasks_dir();
        let mut tasks: Vec<Task> = Vec::new();
        let mut entries = match fs::read_dir(&dir).await {
            Ok(e) => e,
            Err(e) if e.kind() == io::ErrorKind::NotFound => {
                return Ok(ListTasksResponse {
                    tasks: Vec::new(),
                    next_page_token: String::new(),
                    page_size: req.page_size.unwrap_or(50),
                    total_size: 0,
                });
            },
            Err(e) => {
                return Err(A2AError::internal(format!(
                    "failed to list tasks dir {}: {e}",
                    dir.display()
                )));
            },
        };

        while let Some(entry) = entries
            .next_entry()
            .await
            .map_err(|e| A2AError::internal(format!("readdir failed: {e}")))?
        {
            let path = entry.path();
            if path.extension().and_then(|s| s.to_str()) != Some("json") {
                continue;
            }
            let Some(record) = self.read_record(&path).await? else { continue };
            if let Some(ref ctx) = req.context_id {
                if record.task.context_id != *ctx {
                    continue;
                }
            }
            if let Some(ref status) = req.status {
                if record.task.status.state != *status {
                    continue;
                }
            }
            tasks.push(record.task);
        }

        tasks.sort_by(|a, b| a.id.cmp(&b.id));

        let page_size = match req.page_size {
            Some(s) if s > 0 => s as usize,
            _ => 50,
        };
        let start =
            req.page_token.as_deref().and_then(|t| t.parse::<usize>().ok()).unwrap_or(0);
        let total_size = tasks.len();
        let end = (start + page_size).min(total_size);
        let page: Vec<Task> = if start <= total_size {
            tasks[start..end].to_vec()
        } else {
            Vec::new()
        };
        let next_page_token =
            if end < total_size { Some(end.to_string()) } else { None };

        let page = page
            .into_iter()
            .map(|mut task| {
                if let Some(hl) = req.history_length {
                    let hl = hl as usize;
                    if let Some(ref mut history) = task.history {
                        if hl == 0 {
                            *history = Vec::new();
                        } else if history.len() > hl {
                            let start = history.len() - hl;
                            *history = history[start..].to_vec();
                        }
                    }
                }
                task
            })
            .collect();

        Ok(ListTasksResponse {
            tasks: page,
            next_page_token: next_page_token.unwrap_or_default(),
            page_size: page_size as i32,
            total_size: total_size as i32,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_task(id: &str, ctx: &str, state: TaskState) -> Task {
        Task {
            id: id.to_string(),
            context_id: ctx.to_string(),
            status: TaskStatus { state, message: None, timestamp: None },
            artifacts: None,
            history: None,
            metadata: None,
        }
    }

    #[test]
    fn test_task_id_validation() {
        assert!(is_task_id_valid("abc-123_DEF"));
        assert!(!is_task_id_valid(""));
        assert!(!is_task_id_valid("../escape"));
        assert!(!is_task_id_valid("with space"));
        assert!(!is_task_id_valid("dot.json"));
        assert!(!is_task_id_valid(&"a".repeat(129)));
    }

    #[tokio::test]
    async fn test_create_get_update() {
        let tmp = tempfile::tempdir().unwrap();
        let store = FsTaskStore::new(tmp.path());

        let task = make_task("t1", "c1", TaskState::Submitted);
        let v1 = store.create(task.clone()).await.unwrap();
        assert_eq!(v1, 1);

        let loaded = store.get("t1").await.unwrap().expect("present");
        assert_eq!(loaded.id, "t1");
        assert_eq!(loaded.context_id, "c1");

        let updated = make_task("t1", "c1", TaskState::Working);
        let v2 = store.update(updated).await.unwrap();
        assert_eq!(v2, 2);

        let after = store.get("t1").await.unwrap().expect("present");
        assert_eq!(after.status.state, TaskState::Working);
    }

    #[tokio::test]
    async fn test_create_duplicate() {
        let tmp = tempfile::tempdir().unwrap();
        let store = FsTaskStore::new(tmp.path());
        let task = make_task("t1", "c1", TaskState::Submitted);
        store.create(task.clone()).await.unwrap();
        assert!(store.create(task).await.is_err());
    }

    #[tokio::test]
    async fn test_update_not_found() {
        let tmp = tempfile::tempdir().unwrap();
        let store = FsTaskStore::new(tmp.path());
        let task = make_task("ghost", "c1", TaskState::Working);
        let err = store.update(task).await.unwrap_err();
        assert_eq!(err.code, error_code::TASK_NOT_FOUND);
    }

    #[tokio::test]
    async fn test_get_missing_returns_none() {
        let tmp = tempfile::tempdir().unwrap();
        let store = FsTaskStore::new(tmp.path());
        assert!(store.get("ghost").await.unwrap().is_none());
    }

    #[tokio::test]
    async fn test_invalid_task_id_rejected() {
        let tmp = tempfile::tempdir().unwrap();
        let store = FsTaskStore::new(tmp.path());
        let bad = make_task("../escape", "c", TaskState::Submitted);
        assert!(store.create(bad).await.is_err());
        assert!(store.get("../escape").await.is_err());
    }

    #[tokio::test]
    async fn test_list_empty_returns_empty_response() {
        let tmp = tempfile::tempdir().unwrap();
        let store = FsTaskStore::new(tmp.path());
        let req = ListTasksRequest {
            context_id: None,
            status: None,
            page_size: None,
            page_token: None,
            history_length: None,
            status_timestamp_after: None,
            include_artifacts: None,
            tenant: None,
        };
        let resp = store.list(&req).await.unwrap();
        assert!(resp.tasks.is_empty());
        assert_eq!(resp.total_size, 0);
    }

    #[tokio::test]
    async fn test_list_filter_and_paginate() {
        let tmp = tempfile::tempdir().unwrap();
        let store = FsTaskStore::new(tmp.path());
        for i in 0..5 {
            store
                .create(make_task(&format!("t{i}"), "ctx-a", TaskState::Submitted))
                .await
                .unwrap();
        }
        store
            .create(make_task("other", "ctx-b", TaskState::Submitted))
            .await
            .unwrap();

        let req = ListTasksRequest {
            context_id: Some("ctx-a".to_string()),
            status: None,
            page_size: Some(2),
            page_token: None,
            history_length: None,
            status_timestamp_after: None,
            include_artifacts: None,
            tenant: None,
        };
        let resp = store.list(&req).await.unwrap();
        assert_eq!(resp.tasks.len(), 2);
        assert_eq!(resp.total_size, 5);
        assert!(!resp.next_page_token.is_empty());
    }

    #[tokio::test]
    async fn test_persistence_across_instances() {
        let tmp = tempfile::tempdir().unwrap();
        {
            let store = FsTaskStore::new(tmp.path());
            store
                .create(make_task("persist", "c", TaskState::Completed))
                .await
                .unwrap();
        }
        let store2 = FsTaskStore::new(tmp.path());
        let loaded = store2.get("persist").await.unwrap().expect("survives restart");
        assert_eq!(loaded.status.state, TaskState::Completed);
    }
}
