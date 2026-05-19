// SPDX-License-Identifier: Apache-2.0
//! End-to-end smoke tests for `aphrody-task-runner`.

use std::sync::Arc;
use std::sync::atomic::{AtomicU32, AtomicUsize, Ordering};
use std::time::Duration;

use aphrody_task_runner::{
    TaskAction, TaskActionError, TaskContext, TaskError, TaskEvent, TaskGraph, TaskOutput,
    TaskRunner, TaskSpec, TaskStatus,
};
use async_trait::async_trait;
use tokio::sync::{Mutex, mpsc};

// ---------------------------------------------------------------------------
// Test doubles
// ---------------------------------------------------------------------------

struct OkAction {
    log: Arc<Mutex<Vec<String>>>,
    label: String,
}

#[async_trait]
impl TaskAction for OkAction {
    async fn execute(&self, _ctx: &TaskContext) -> Result<TaskOutput, TaskActionError> {
        self.log.lock().await.push(self.label.clone());
        Ok(serde_json::json!({ "label": self.label }))
    }
}

struct AlwaysFailAction {
    message: String,
    attempts: Arc<AtomicU32>,
}

#[async_trait]
impl TaskAction for AlwaysFailAction {
    async fn execute(&self, _ctx: &TaskContext) -> Result<TaskOutput, TaskActionError> {
        self.attempts.fetch_add(1, Ordering::SeqCst);
        Err(TaskActionError::new(self.message.clone()))
    }
}

struct FlakyAction {
    fail_until: u32,
    attempts: Arc<AtomicU32>,
}

#[async_trait]
impl TaskAction for FlakyAction {
    async fn execute(&self, ctx: &TaskContext) -> Result<TaskOutput, TaskActionError> {
        let prior = self.attempts.fetch_add(1, Ordering::SeqCst);
        if prior < self.fail_until {
            return Err(TaskActionError::new(format!(
                "fail on attempt {}",
                ctx.attempt
            )));
        }
        Ok(serde_json::json!({ "ok": true, "attempt": ctx.attempt }))
    }
}

struct SleepyAction {
    sleep: Duration,
}

#[async_trait]
impl TaskAction for SleepyAction {
    async fn execute(&self, _ctx: &TaskContext) -> Result<TaskOutput, TaskActionError> {
        tokio::time::sleep(self.sleep).await;
        Ok(serde_json::Value::Null)
    }
}

struct ConcurrencyAction {
    sleep: Duration,
    live: Arc<AtomicUsize>,
    peak: Arc<AtomicUsize>,
}

#[async_trait]
impl TaskAction for ConcurrencyAction {
    async fn execute(&self, _ctx: &TaskContext) -> Result<TaskOutput, TaskActionError> {
        let now = self.live.fetch_add(1, Ordering::SeqCst) + 1;
        self.peak.fetch_max(now, Ordering::SeqCst);
        tokio::time::sleep(self.sleep).await;
        self.live.fetch_sub(1, Ordering::SeqCst);
        Ok(serde_json::Value::Null)
    }
}

// ---------------------------------------------------------------------------
// 1. topological order — simple chain
// ---------------------------------------------------------------------------

#[test]
fn topo_order_simple_chain() {
    let mut g = TaskGraph::new();
    g.add(
        TaskSpec::leaf("a", "a"),
        Arc::new(OkAction {
            log: Arc::new(Mutex::new(Vec::new())),
            label: "a".into(),
        }),
    )
    .unwrap();
    g.add(
        TaskSpec::leaf("b", "b").with_deps(["a"]),
        Arc::new(OkAction {
            log: Arc::new(Mutex::new(Vec::new())),
            label: "b".into(),
        }),
    )
    .unwrap();
    g.add(
        TaskSpec::leaf("c", "c").with_deps(["b"]),
        Arc::new(OkAction {
            log: Arc::new(Mutex::new(Vec::new())),
            label: "c".into(),
        }),
    )
    .unwrap();

    let order: Vec<String> = g
        .topo_order()
        .unwrap()
        .into_iter()
        .map(|id| id.as_str().to_string())
        .collect();
    assert_eq!(order, vec!["a".to_string(), "b".to_string(), "c".to_string()]);
}

// ---------------------------------------------------------------------------
// 2. cycle detection
// ---------------------------------------------------------------------------

#[test]
fn topo_order_detects_cycle() {
    let log = Arc::new(Mutex::new(Vec::new()));
    let mut g = TaskGraph::new();
    g.add(
        TaskSpec::leaf("a", "a").with_deps(["b"]),
        Arc::new(OkAction { log: log.clone(), label: "a".into() }),
    )
    .unwrap();
    g.add(
        TaskSpec::leaf("b", "b").with_deps(["a"]),
        Arc::new(OkAction { log: log.clone(), label: "b".into() }),
    )
    .unwrap();
    let err = g.topo_order().unwrap_err();
    assert!(matches!(err, TaskError::CycleDetected(_)));
}

// ---------------------------------------------------------------------------
// 3. missing dependency rejected
// ---------------------------------------------------------------------------

#[test]
fn missing_dependency_rejected() {
    let log = Arc::new(Mutex::new(Vec::new()));
    let mut g = TaskGraph::new();
    g.add(
        TaskSpec::leaf("a", "a").with_deps(["ghost"]),
        Arc::new(OkAction { log: log.clone(), label: "a".into() }),
    )
    .unwrap();
    let err = g.validate().unwrap_err();
    assert!(matches!(err, TaskError::MissingDependency { .. }));
}

// ---------------------------------------------------------------------------
// 4. run executes all independent tasks
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn run_executes_all_independent_tasks() {
    let log = Arc::new(Mutex::new(Vec::new()));
    let mut g = TaskGraph::new();
    for name in ["alpha", "beta", "gamma"] {
        g.add(
            TaskSpec::leaf(name, name),
            Arc::new(OkAction { log: log.clone(), label: name.into() }),
        )
        .unwrap();
    }
    let runner = TaskRunner::new(3);
    let report = runner.run(g).await.unwrap();
    assert_eq!(report.succeeded, 3);
    assert_eq!(report.failed, 0);
    assert_eq!(report.skipped, 0);
    let mut entries = log.lock().await.clone();
    entries.sort();
    assert_eq!(entries, vec!["alpha".to_string(), "beta".into(), "gamma".into()]);
    for node in report.nodes.values() {
        assert!(matches!(node.status, TaskStatus::Completed));
        assert_eq!(node.attempts, 1);
    }
}

// ---------------------------------------------------------------------------
// 5. dependency order is honoured at runtime
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn run_respects_dependency_order() {
    let log = Arc::new(Mutex::new(Vec::new()));
    let mut g = TaskGraph::new();
    g.add(
        TaskSpec::leaf("a", "first"),
        Arc::new(OkAction { log: log.clone(), label: "a".into() }),
    )
    .unwrap();
    g.add(
        TaskSpec::leaf("b", "second").with_deps(["a"]),
        Arc::new(OkAction { log: log.clone(), label: "b".into() }),
    )
    .unwrap();
    let runner = TaskRunner::new(4);
    let report = runner.run(g).await.unwrap();
    assert!(report.all_succeeded());
    let entries = log.lock().await.clone();
    assert_eq!(entries.first().map(String::as_str), Some("a"));
    assert_eq!(entries.last().map(String::as_str), Some("b"));
}

// ---------------------------------------------------------------------------
// 6. retry succeeds after N failures
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn retry_succeeds_after_n_failures() {
    let attempts = Arc::new(AtomicU32::new(0));
    let action = Arc::new(FlakyAction {
        fail_until: 2,
        attempts: attempts.clone(),
    });
    let spec = TaskSpec::leaf("flaky", "flaky").with_max_retries(3);
    let mut g = TaskGraph::new();
    g.add(spec, action).unwrap();
    let runner = TaskRunner {
        max_concurrency: 1,
        default_timeout: None,
        default_retries: 0,
        retry_backoff: Duration::from_millis(0),
    };
    let report = runner.run(g).await.unwrap();
    assert_eq!(attempts.load(Ordering::SeqCst), 3);
    let node = report.nodes.values().next().unwrap();
    assert!(matches!(node.status, TaskStatus::Completed));
    assert_eq!(node.attempts, 3);
}

// ---------------------------------------------------------------------------
// 7. failure cascades to dependent tasks
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn failure_cascades_to_dependent_tasks() {
    let attempts = Arc::new(AtomicU32::new(0));
    let mut g = TaskGraph::new();
    g.add(
        TaskSpec::leaf("a", "fails"),
        Arc::new(AlwaysFailAction {
            message: "boom".into(),
            attempts: attempts.clone(),
        }),
    )
    .unwrap();
    g.add(
        TaskSpec::leaf("b", "downstream").with_deps(["a"]),
        Arc::new(OkAction {
            log: Arc::new(Mutex::new(Vec::new())),
            label: "b".into(),
        }),
    )
    .unwrap();
    let runner = TaskRunner {
        max_concurrency: 2,
        default_timeout: None,
        default_retries: 0,
        retry_backoff: Duration::from_millis(0),
    };
    let report = runner.run(g).await.unwrap();
    let a = report.nodes.get(&"a".into()).unwrap();
    let b = report.nodes.get(&"b".into()).unwrap();
    assert!(matches!(a.status, TaskStatus::Failed(_)));
    match &b.status {
        TaskStatus::Skipped(reason) => assert!(reason.contains("a")),
        other => panic!("expected Skipped, got {other:?}"),
    }
    assert_eq!(report.failed, 1);
    assert_eq!(report.skipped, 1);
}

// ---------------------------------------------------------------------------
// 8. timeout kills long running task
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn timeout_kills_long_running_task() {
    let action = Arc::new(SleepyAction { sleep: Duration::from_secs(5) });
    let spec = TaskSpec::leaf("slow", "slow").with_timeout_ms(50);
    let mut g = TaskGraph::new();
    g.add(spec, action).unwrap();
    let runner = TaskRunner::new(1);
    let report = runner.run(g).await.unwrap();
    let node = report.nodes.values().next().unwrap();
    assert!(matches!(node.status, TaskStatus::TimedOut));
    assert_eq!(report.failed, 1);
}

// ---------------------------------------------------------------------------
// 9. concurrency limit respected (semaphore-driven)
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 8)]
async fn concurrency_limit_respected() {
    let live = Arc::new(AtomicUsize::new(0));
    let peak = Arc::new(AtomicUsize::new(0));
    let mut g = TaskGraph::new();
    for i in 0..5 {
        g.add(
            TaskSpec::leaf(format!("t{i}"), format!("t{i}")),
            Arc::new(ConcurrencyAction {
                sleep: Duration::from_millis(50),
                live: live.clone(),
                peak: peak.clone(),
            }),
        )
        .unwrap();
    }
    let runner = TaskRunner::new(2);
    let report = runner.run(g).await.unwrap();
    assert!(report.all_succeeded());
    let peak_value = peak.load(Ordering::SeqCst);
    assert!(peak_value <= 2, "peak concurrency was {peak_value}, expected <= 2");
    assert!(peak_value >= 1, "peak concurrency should be at least 1, got {peak_value}");
}

// ---------------------------------------------------------------------------
// 10. live event stream is emitted in order
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn event_stream_emits_run_started_and_finished() {
    let log = Arc::new(Mutex::new(Vec::new()));
    let mut g = TaskGraph::new();
    g.add(
        TaskSpec::leaf("only", "only"),
        Arc::new(OkAction { log: log.clone(), label: "only".into() }),
    )
    .unwrap();
    let runner = TaskRunner::new(1);
    let (tx, mut rx) = mpsc::channel::<TaskEvent>(32);
    let report = runner.run_with_events(g, tx).await.unwrap();
    drop(report);

    let mut kinds: Vec<String> = Vec::new();
    while let Some(ev) = rx.recv().await {
        match ev {
            TaskEvent::RunStarted { .. } => kinds.push("run_started".into()),
            TaskEvent::TaskStarted { .. } => kinds.push("task_started".into()),
            TaskEvent::TaskFinished { .. } => kinds.push("task_finished".into()),
            TaskEvent::TaskProgress { .. } => kinds.push("task_progress".into()),
            TaskEvent::RunFinished { .. } => {
                kinds.push("run_finished".into());
                break;
            }
        }
    }
    assert_eq!(kinds.first().map(String::as_str), Some("run_started"));
    assert_eq!(kinds.last().map(String::as_str), Some("run_finished"));
    assert!(kinds.iter().any(|k| k == "task_started"));
    assert!(kinds.iter().any(|k| k == "task_finished"));
}
