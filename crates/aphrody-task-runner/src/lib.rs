// SPDX-License-Identifier: Apache-2.0
//! aphrody-task-runner — parallel DAG executor for aphrody background pipelines.
//!
//! This crate owns the **"when"** *and* the **"what"** of a task graph: given a
//! set of [`TaskSpec`]s (each with optional dependencies, retry budget,
//! per-task timeout and scheduling weight) and matching [`TaskAction`]
//! implementations, [`TaskRunner::run`] performs a topological pass and
//! drives each ready node concurrently up to a configurable cap, emitting a
//! [`TaskEvent`] stream that downstream consumers can serialize as
//! line-delimited JSON without any framing glue.
//!
//! ## Design goals
//!
//! 1. **Real concurrency, not cooperative.** The scheduler uses
//!    [`tokio::sync::Semaphore`] permits to fan out work across the
//!    multi-thread runtime, and waits on a `JoinSet` of node futures so
//!    that completion ordering is the natural async event order.
//! 2. **True dependency cascade.** When a node fails (and exhausts its
//!    retry budget) every transitive descendant transitions straight to
//!    `Skipped` with a structured reason — the runner never executes
//!    work that already lost its preconditions.
//! 3. **Deterministic tests.** All timing surfaces ([`TaskRunner::run`]
//!    timeouts, retry sleeps) go through [`tokio::time::sleep`] /
//!    [`tokio::time::timeout`], so the test suite uses
//!    `#[tokio::test(start_paused = true)]` for fully deterministic
//!    backoff verification without wall-clock latency.
//! 4. **Observable.** Every transition emits a [`TaskEvent`] on the
//!    consumer-supplied channel; the final [`TaskReport`] additionally
//!    snapshots every node's wall-clock window and attempt count.
//!
//! ## Cohesion with the wider workspace
//!
//! * **`aphrody-cron`** schedules ticks; this crate executes the per-tick
//!   batch as a typed DAG instead of a flat list (lets jobs declare their
//!   read-after-write ordering explicitly).
//! * **`aphrody-hooks`** dispatches user hooks with per-command timeouts
//!   via [`tokio::process::Command`] + [`tokio::time::timeout`]; the same
//!   timeout primitive is reused here for arbitrary [`TaskAction`]s.
//! * **`aphrody-retry`** provides the canonical retry vocabulary
//!   (`max_attempts`, jitter, classifier). This crate keeps a strictly
//!   simpler per-task retry knob (`max_retries`, fixed exponential
//!   back-off) so leaf actions can plug `aphrody-retry::retry()` *inside*
//!   their [`TaskAction::execute`] for HTTP-grade tuning and let the
//!   runner govern only the coarse "did the node eventually succeed?"
//!   question.
//!
//! See `tests/runner_smoke.rs` for the executable contract.

#![forbid(unsafe_code)]

use std::collections::{BTreeMap, HashMap, HashSet, VecDeque};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::time::Duration;

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use thiserror::Error;
use tokio::sync::{Semaphore, mpsc};
use tokio::task::JoinSet;
use tokio::time::{Instant, timeout};
use tracing::{debug, warn};

// ---------------------------------------------------------------------------
// TaskId / TaskOutput
// ---------------------------------------------------------------------------

/// Stable identifier for a task in the graph.
///
/// Wraps a [`String`] so the same DTO can round-trip through serde without
/// extra ceremony, and so downstream consumers can use `HashMap<TaskId, _>`
/// directly.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct TaskId(pub String);

impl TaskId {
    /// Construct a [`TaskId`] from anything string-like.
    pub fn new(s: impl Into<String>) -> Self {
        Self(s.into())
    }

    /// Borrow the underlying string slice.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for TaskId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

impl From<&str> for TaskId {
    fn from(s: &str) -> Self {
        Self(s.to_string())
    }
}

impl From<String> for TaskId {
    fn from(s: String) -> Self {
        Self(s)
    }
}

/// Free-form structured output returned by a [`TaskAction`].
pub type TaskOutput = serde_json::Value;

// ---------------------------------------------------------------------------
// TaskStatus
// ---------------------------------------------------------------------------

/// Lifecycle state of a single node.
///
/// Variants carry their failure context inline so a serialized
/// [`TaskReport`] is self-describing without a side channel.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum TaskStatus {
    /// Not yet started.
    Pending,
    /// Currently executing (possibly mid-retry).
    Running,
    /// Action returned `Ok(_)` within its retry budget.
    Completed,
    /// Action returned `Err(_)` and exhausted [`TaskSpec::max_retries`].
    Failed(String),
    /// Skipped because an upstream dependency did not reach
    /// [`TaskStatus::Completed`].
    Skipped(String),
    /// Action did not return inside [`TaskSpec::timeout_ms`].
    TimedOut,
}

impl TaskStatus {
    /// `true` when the task successfully ran to completion.
    #[must_use]
    pub fn is_success(&self) -> bool {
        matches!(self, Self::Completed)
    }

    /// `true` when the task is in a terminal failure shape (`Failed`,
    /// `Skipped` or `TimedOut`).
    #[must_use]
    pub fn is_terminal_failure(&self) -> bool {
        matches!(self, Self::Failed(_) | Self::Skipped(_) | Self::TimedOut)
    }
}

// ---------------------------------------------------------------------------
// TaskSpec
// ---------------------------------------------------------------------------

/// Declarative description of a node in the task graph.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskSpec {
    /// Stable identifier referenced by other tasks' [`Self::dependencies`].
    pub id: TaskId,
    /// Human-readable label surfaced in events and the final report.
    pub description: String,
    /// IDs that must reach [`TaskStatus::Completed`] before this task can
    /// be scheduled.
    #[serde(default)]
    pub dependencies: Vec<TaskId>,
    /// Optional per-task wall-clock budget (milliseconds).
    #[serde(default)]
    pub timeout_ms: Option<u64>,
    /// Maximum number of retries on `Err(_)` (so `max_retries = 2` means
    /// up to 3 attempts in total).
    #[serde(default)]
    pub max_retries: u32,
    /// Scheduling weight, higher values run first when several tasks
    /// become ready in the same tick.
    #[serde(default = "default_weight")]
    pub weight: u32,
}

fn default_weight() -> u32 {
    1
}

impl TaskSpec {
    /// Convenience constructor for a leaf task with no dependencies.
    pub fn leaf(id: impl Into<TaskId>, description: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            description: description.into(),
            dependencies: Vec::new(),
            timeout_ms: None,
            max_retries: 0,
            weight: 1,
        }
    }

    /// Builder helper: attach a list of dependency ids.
    #[must_use]
    pub fn with_deps<I, T>(mut self, deps: I) -> Self
    where
        I: IntoIterator<Item = T>,
        T: Into<TaskId>,
    {
        self.dependencies = deps.into_iter().map(Into::into).collect();
        self
    }

    /// Builder helper: set timeout.
    #[must_use]
    pub fn with_timeout_ms(mut self, ms: u64) -> Self {
        self.timeout_ms = Some(ms);
        self
    }

    /// Builder helper: set retry budget.
    #[must_use]
    pub fn with_max_retries(mut self, n: u32) -> Self {
        self.max_retries = n;
        self
    }

    /// Builder helper: set scheduling weight.
    #[must_use]
    pub fn with_weight(mut self, w: u32) -> Self {
        self.weight = w;
        self
    }
}

// ---------------------------------------------------------------------------
// TaskContext — passed to TaskAction::execute
// ---------------------------------------------------------------------------

/// Cooperative cancellation token, AtomicBool-backed so we avoid pulling
/// in `tokio-util` for a single bit of shared state.
#[derive(Debug, Clone, Default)]
pub struct CancelToken {
    flag: Arc<AtomicBool>,
}

impl CancelToken {
    /// Construct a fresh, un-cancelled token.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Signal cancellation to any holder polling [`Self::is_cancelled`].
    pub fn cancel(&self) {
        self.flag.store(true, Ordering::SeqCst);
    }

    /// `true` after the first call to [`Self::cancel`].
    #[must_use]
    pub fn is_cancelled(&self) -> bool {
        self.flag.load(Ordering::SeqCst)
    }
}

/// Per-attempt context handed to every [`TaskAction::execute`] call.
#[derive(Debug, Clone)]
pub struct TaskContext {
    /// Identifier of the task being executed.
    pub task_id: TaskId,
    /// Current attempt number (`1` for the initial call, `2` for the
    /// first retry, …).
    pub attempt: u32,
    /// Cooperative cancellation flag. Long-running actions should poll
    /// [`CancelToken::is_cancelled`] periodically.
    pub cancel_token: CancelToken,
}

// ---------------------------------------------------------------------------
// TaskAction trait
// ---------------------------------------------------------------------------

/// Error returned by a [`TaskAction`]. Stringly-typed on purpose so the
/// runner can serialize it into [`TaskStatus::Failed`].
#[derive(Debug, Error)]
#[error("task action failed: {message}")]
pub struct TaskActionError {
    /// Human-readable failure reason, surfaced verbatim in the report.
    pub message: String,
}

impl TaskActionError {
    /// Construct from anything that can render as a [`String`].
    pub fn new(message: impl Into<String>) -> Self {
        Self { message: message.into() }
    }
}

impl From<&str> for TaskActionError {
    fn from(s: &str) -> Self {
        Self::new(s)
    }
}

impl From<String> for TaskActionError {
    fn from(s: String) -> Self {
        Self::new(s)
    }
}

/// Asynchronous unit of work driven by [`TaskRunner`].
#[async_trait]
pub trait TaskAction: Send + Sync {
    /// Execute one attempt of the task.
    ///
    /// Returning `Ok` transitions the node to [`TaskStatus::Completed`].
    /// Returning `Err` triggers a retry if [`TaskSpec::max_retries`] still
    /// allows it; otherwise the node transitions to
    /// [`TaskStatus::Failed`] and the failure cascades to descendants.
    async fn execute(&self, ctx: &TaskContext) -> Result<TaskOutput, TaskActionError>;
}

// ---------------------------------------------------------------------------
// TaskNode — internal scheduler view
// ---------------------------------------------------------------------------

/// Bookkeeping snapshot for a single graph node.
///
/// Exposed through [`TaskReport::nodes`] for post-mortem analysis.
#[derive(Debug, Clone)]
pub struct TaskNode {
    /// Original declarative spec.
    pub spec: TaskSpec,
    /// Terminal status after [`TaskRunner::run`] returns.
    pub status: TaskStatus,
    /// Wall-clock instant of the first scheduled attempt.
    pub started_at: Option<DateTime<Utc>>,
    /// Wall-clock instant at which the terminal status was assigned.
    pub finished_at: Option<DateTime<Utc>>,
    /// Number of attempts actually performed (1 + retries).
    pub attempts: u32,
    /// JSON output of the final successful attempt, if any.
    pub output: Option<TaskOutput>,
}

impl TaskNode {
    fn fresh(spec: TaskSpec) -> Self {
        Self {
            spec,
            status: TaskStatus::Pending,
            started_at: None,
            finished_at: None,
            attempts: 0,
            output: None,
        }
    }
}

// ---------------------------------------------------------------------------
// TaskGraph
// ---------------------------------------------------------------------------

/// Mutable builder representing a DAG of tasks paired with their actions.
pub struct TaskGraph {
    pub(crate) nodes: HashMap<TaskId, TaskNode>,
    pub(crate) actions: HashMap<TaskId, Arc<dyn TaskAction + Send + Sync>>,
}

impl std::fmt::Debug for TaskGraph {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TaskGraph")
            .field("nodes", &self.nodes.keys().collect::<Vec<_>>())
            .finish()
    }
}

impl Default for TaskGraph {
    fn default() -> Self {
        Self::new()
    }
}

impl TaskGraph {
    /// Construct an empty graph.
    #[must_use]
    pub fn new() -> Self {
        Self { nodes: HashMap::new(), actions: HashMap::new() }
    }

    /// Insert a new task into the graph. Returns
    /// [`TaskError::DuplicateTask`] if the id collides with an existing
    /// entry.
    pub fn add(
        &mut self,
        spec: TaskSpec,
        action: Arc<dyn TaskAction + Send + Sync>,
    ) -> Result<(), TaskError> {
        if self.nodes.contains_key(&spec.id) {
            return Err(TaskError::DuplicateTask(spec.id.0.clone()));
        }
        let id = spec.id.clone();
        self.actions.insert(id.clone(), action);
        self.nodes.insert(id, TaskNode::fresh(spec));
        Ok(())
    }

    /// Number of nodes in the graph.
    #[must_use]
    pub fn len(&self) -> usize {
        self.nodes.len()
    }

    /// `true` when the graph contains no nodes.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.nodes.is_empty()
    }

    /// Validate every dependency edge points to a known node and that
    /// the graph is acyclic.
    pub fn validate(&self) -> Result<(), TaskError> {
        for node in self.nodes.values() {
            for dep in &node.spec.dependencies {
                if !self.nodes.contains_key(dep) {
                    return Err(TaskError::MissingDependency {
                        task: node.spec.id.0.clone(),
                        dep: dep.0.clone(),
                    });
                }
            }
        }
        let _ = self.topo_order()?;
        Ok(())
    }

    /// Kahn's algorithm: produce a topological ordering of node IDs, or
    /// [`TaskError::CycleDetected`] if the graph has a cycle.
    pub fn topo_order(&self) -> Result<Vec<TaskId>, TaskError> {
        // in-degree per node
        let mut in_degree: HashMap<TaskId, usize> =
            self.nodes.keys().map(|k| (k.clone(), 0_usize)).collect();
        // forward adjacency (dep -> dependents)
        let mut adj: HashMap<TaskId, Vec<TaskId>> = HashMap::new();

        for node in self.nodes.values() {
            for dep in &node.spec.dependencies {
                if !self.nodes.contains_key(dep) {
                    return Err(TaskError::MissingDependency {
                        task: node.spec.id.0.clone(),
                        dep: dep.0.clone(),
                    });
                }
                adj.entry(dep.clone()).or_default().push(node.spec.id.clone());
                *in_degree.entry(node.spec.id.clone()).or_insert(0) += 1;
            }
        }

        // sort starting frontier deterministically by id for reproducible
        // tie-breaking in tests and logs.
        let mut frontier: Vec<TaskId> = in_degree
            .iter()
            .filter(|(_, d)| **d == 0)
            .map(|(id, _)| id.clone())
            .collect();
        frontier.sort_by(|a, b| a.0.cmp(&b.0));

        let mut queue: VecDeque<TaskId> = frontier.into_iter().collect();
        let mut ordered: Vec<TaskId> = Vec::with_capacity(self.nodes.len());

        while let Some(next) = queue.pop_front() {
            ordered.push(next.clone());
            if let Some(children) = adj.get(&next) {
                let mut sorted_children = children.clone();
                sorted_children.sort_by(|a, b| a.0.cmp(&b.0));
                for child in sorted_children {
                    let entry = in_degree.get_mut(&child).expect("known node");
                    *entry -= 1;
                    if *entry == 0 {
                        queue.push_back(child);
                    }
                }
            }
        }

        if ordered.len() != self.nodes.len() {
            // Collect surviving nodes to give a helpful error message.
            let remaining: Vec<String> = in_degree
                .iter()
                .filter(|(_, d)| **d > 0)
                .map(|(id, _)| id.0.clone())
                .collect();
            return Err(TaskError::CycleDetected(remaining.join(",")));
        }
        Ok(ordered)
    }
}

// ---------------------------------------------------------------------------
// TaskRunner
// ---------------------------------------------------------------------------

/// Top-level scheduler configuration.
#[derive(Debug, Clone)]
pub struct TaskRunner {
    /// Maximum number of nodes allowed to run simultaneously.
    pub max_concurrency: usize,
    /// Fallback timeout for nodes whose [`TaskSpec::timeout_ms`] is `None`.
    /// `None` here means "no implicit timeout".
    pub default_timeout: Option<Duration>,
    /// Fallback retry budget when [`TaskSpec::max_retries`] is zero.
    pub default_retries: u32,
    /// Base back-off applied between retries (then multiplied by attempt
    /// index for a coarse exponential ramp).
    pub retry_backoff: Duration,
}

impl Default for TaskRunner {
    fn default() -> Self {
        Self {
            max_concurrency: 4,
            default_timeout: None,
            default_retries: 0,
            retry_backoff: Duration::from_millis(50),
        }
    }
}

impl TaskRunner {
    /// Construct a runner with explicit knobs.
    #[must_use]
    pub fn new(max_concurrency: usize) -> Self {
        Self {
            max_concurrency: max_concurrency.max(1),
            ..Self::default()
        }
    }

    /// Run `graph` and discard the live event stream. Returns the final
    /// [`TaskReport`] once every node has reached a terminal status.
    pub async fn run(&self, graph: TaskGraph) -> Result<TaskReport, TaskError> {
        let (tx, mut rx) = mpsc::channel::<TaskEvent>(256);
        // Drain events so the channel never back-pressures the runner.
        let drain = tokio::spawn(async move { while rx.recv().await.is_some() {} });
        let report = self.run_with_events(graph, tx).await?;
        let _ = drain.await;
        Ok(report)
    }

    /// Run `graph`, streaming every transition to `events`.
    ///
    /// The runner does not close `events`; consumers should drop the
    /// receiver after [`TaskEvent::RunFinished`] arrives.
    pub async fn run_with_events(
        &self,
        graph: TaskGraph,
        events: mpsc::Sender<TaskEvent>,
    ) -> Result<TaskReport, TaskError> {
        graph.validate()?;
        let order = graph.topo_order()?;
        let TaskGraph { mut nodes, actions } = graph;
        let total_tasks = nodes.len() as u32;
        let cancel = CancelToken::new();

        let _ = events
            .send(TaskEvent::RunStarted { total_tasks, ts: Utc::now() })
            .await;

        let run_started = Instant::now();
        let semaphore = Arc::new(Semaphore::new(self.max_concurrency.max(1)));
        let live_counter = Arc::new(AtomicU64::new(0));
        let mut join: JoinSet<NodeResult> = JoinSet::new();
        let mut running: HashSet<TaskId> = HashSet::new();
        let mut remaining: HashSet<TaskId> = order.iter().cloned().collect();

        loop {
            // Cascade: any node whose dependencies are no longer reachable
            // must be skipped before we attempt to schedule new work.
            self.cascade_skips(&mut nodes, &mut remaining, &events).await;
            if remaining.is_empty() && running.is_empty() {
                break;
            }

            // Schedule ready nodes (Pending with all deps Completed).
            let ready: Vec<TaskId> = {
                let mut candidates: Vec<TaskId> = remaining
                    .iter()
                    .filter(|id| !running.contains(id))
                    .filter(|id| {
                        let node = &nodes[*id];
                        matches!(node.status, TaskStatus::Pending)
                            && node
                                .spec
                                .dependencies
                                .iter()
                                .all(|d| matches!(nodes.get(d).map(|n| &n.status), Some(TaskStatus::Completed)))
                    })
                    .cloned()
                    .collect();
                // Higher weight runs first; ties broken by id for
                // determinism in tests.
                candidates.sort_by(|a, b| {
                    let wa = nodes[a].spec.weight;
                    let wb = nodes[b].spec.weight;
                    wb.cmp(&wa).then_with(|| a.0.cmp(&b.0))
                });
                candidates
            };

            for id in ready {
                let action = actions.get(&id).cloned().ok_or_else(|| {
                    TaskError::TaskActionFailed(format!(
                        "internal: action missing for task '{}'",
                        id.0
                    ))
                })?;
                let spec = nodes[&id].spec.clone();
                let semaphore = semaphore.clone();
                let cancel = cancel.clone();
                let live_counter = live_counter.clone();
                let events_tx = events.clone();
                let default_timeout = self.default_timeout;
                let default_retries = self.default_retries;
                let retry_backoff = self.retry_backoff;
                let id_clone = id.clone();

                {
                    let node = nodes.get_mut(&id).expect("node present");
                    node.status = TaskStatus::Running;
                    node.started_at = Some(Utc::now());
                }
                running.insert(id.clone());
                remaining.remove(&id);

                join.spawn(async move {
                    let permit = semaphore.acquire_owned().await.expect("semaphore not closed");
                    let live = live_counter.fetch_add(1, Ordering::SeqCst) + 1;
                    debug!(task = %id_clone, live, "task acquired permit");
                    let outcome = execute_task_with_retries(
                        spec.clone(),
                        action,
                        cancel,
                        default_timeout,
                        default_retries,
                        retry_backoff,
                        events_tx.clone(),
                    )
                    .await;
                    live_counter.fetch_sub(1, Ordering::SeqCst);
                    drop(permit);
                    NodeResult { id: id_clone, outcome }
                });
            }

            // Wait for at least one running task to terminate before
            // trying to schedule again (otherwise the loop spins).
            if running.is_empty() {
                // No ready node but cascade left some `remaining` — shouldn't
                // happen because cascade_skips empties anything unreachable.
                break;
            }

            if let Some(finished) = join.join_next().await {
                let NodeResult { id, outcome } =
                    finished.map_err(|e| TaskError::TaskActionFailed(e.to_string()))?;
                running.remove(&id);
                let node = nodes.get_mut(&id).expect("node present");
                let now = Utc::now();
                node.finished_at = Some(now);
                node.attempts = outcome.attempts;
                let duration_ms = node
                    .started_at
                    .map(|s| now.signed_duration_since(s).num_milliseconds().max(0) as u64)
                    .unwrap_or(0);
                match outcome.kind {
                    OutcomeKind::Completed(out) => {
                        node.status = TaskStatus::Completed;
                        node.output = Some(out);
                    }
                    OutcomeKind::Failed(msg) => {
                        node.status = TaskStatus::Failed(msg);
                    }
                    OutcomeKind::TimedOut => {
                        node.status = TaskStatus::TimedOut;
                    }
                }
                let _ = events
                    .send(TaskEvent::TaskFinished {
                        id: id.clone(),
                        status: node.status.clone(),
                        duration_ms,
                        ts: now,
                    })
                    .await;
            }
        }

        let total_duration_ms = run_started.elapsed().as_millis() as u64;
        let mut succeeded = 0_u32;
        let mut failed = 0_u32;
        let mut skipped = 0_u32;
        for node in nodes.values() {
            match &node.status {
                TaskStatus::Completed => succeeded += 1,
                TaskStatus::Failed(_) | TaskStatus::TimedOut => failed += 1,
                TaskStatus::Skipped(_) => skipped += 1,
                TaskStatus::Pending | TaskStatus::Running => {
                    // Defensive: any leftover Pending/Running counts as
                    // failure for the summary.
                    failed += 1;
                }
            }
        }

        let _ = events
            .send(TaskEvent::RunFinished {
                completed: succeeded,
                failed,
                skipped,
                ts: Utc::now(),
            })
            .await;

        Ok(TaskReport {
            nodes,
            total_duration_ms,
            succeeded,
            failed,
            skipped,
        })
    }

    /// Walk pending nodes and skip any whose dependency chain is no
    /// longer satisfiable (some ancestor is in a terminal failure
    /// state). Emits `TaskFinished` events for each skipped node so
    /// downstream consumers see a complete trace.
    async fn cascade_skips(
        &self,
        nodes: &mut HashMap<TaskId, TaskNode>,
        remaining: &mut HashSet<TaskId>,
        events: &mpsc::Sender<TaskEvent>,
    ) {
        loop {
            let mut to_skip: Vec<(TaskId, String)> = Vec::new();
            for id in remaining.iter() {
                let node = &nodes[id];
                if !matches!(node.status, TaskStatus::Pending) {
                    continue;
                }
                for dep in &node.spec.dependencies {
                    if let Some(dep_node) = nodes.get(dep) {
                        if dep_node.status.is_terminal_failure() {
                            to_skip.push((id.clone(), format!("upstream failed: {dep}")));
                            break;
                        }
                    }
                }
            }
            if to_skip.is_empty() {
                return;
            }
            for (id, reason) in to_skip {
                let now = Utc::now();
                let node = nodes.get_mut(&id).expect("known");
                node.status = TaskStatus::Skipped(reason.clone());
                node.finished_at = Some(now);
                remaining.remove(&id);
                let _ = events
                    .send(TaskEvent::TaskFinished {
                        id,
                        status: TaskStatus::Skipped(reason),
                        duration_ms: 0,
                        ts: now,
                    })
                    .await;
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Internal execution helpers
// ---------------------------------------------------------------------------

struct NodeResult {
    id: TaskId,
    outcome: AttemptOutcome,
}

struct AttemptOutcome {
    attempts: u32,
    kind: OutcomeKind,
}

enum OutcomeKind {
    Completed(TaskOutput),
    Failed(String),
    TimedOut,
}

async fn execute_task_with_retries(
    spec: TaskSpec,
    action: Arc<dyn TaskAction + Send + Sync>,
    cancel: CancelToken,
    default_timeout: Option<Duration>,
    default_retries: u32,
    retry_backoff: Duration,
    events_tx: mpsc::Sender<TaskEvent>,
) -> AttemptOutcome {
    let max_retries = if spec.max_retries > 0 {
        spec.max_retries
    } else {
        default_retries
    };
    let attempt_budget = max_retries.saturating_add(1);
    let timeout_dur = spec
        .timeout_ms
        .map(Duration::from_millis)
        .or(default_timeout);

    let mut last_err: Option<String> = None;
    let mut last_timed_out = false;
    let mut attempts_done = 0_u32;
    for attempt in 1..=attempt_budget {
        attempts_done = attempt;
        let ctx = TaskContext {
            task_id: spec.id.clone(),
            attempt,
            cancel_token: cancel.clone(),
        };
        let _ = events_tx
            .send(TaskEvent::TaskStarted {
                id: spec.id.clone(),
                attempt,
                ts: Utc::now(),
            })
            .await;
        let fut = action.execute(&ctx);
        let result = match timeout_dur {
            Some(d) => match timeout(d, fut).await {
                Ok(inner) => inner.map_err(|e| AttemptFail::Action(e.message)),
                Err(_) => Err(AttemptFail::Timeout),
            },
            None => fut.await.map_err(|e| AttemptFail::Action(e.message)),
        };
        match result {
            Ok(out) => {
                return AttemptOutcome {
                    attempts: attempt,
                    kind: OutcomeKind::Completed(out),
                };
            }
            Err(AttemptFail::Timeout) => {
                warn!(task = %spec.id, attempt, "task attempt timed out");
                last_timed_out = true;
                last_err = Some(format!("attempt {attempt} timed out"));
                // Timeouts do not retry: the work is presumed unrecoverable
                // within the requested envelope.
                break;
            }
            Err(AttemptFail::Action(msg)) => {
                let _ = events_tx
                    .send(TaskEvent::TaskProgress {
                        id: spec.id.clone(),
                        message: format!("attempt {attempt} failed: {msg}"),
                        ts: Utc::now(),
                    })
                    .await;
                last_timed_out = false;
                last_err = Some(msg);
                if attempt < attempt_budget {
                    let backoff = retry_backoff
                        .checked_mul(attempt)
                        .unwrap_or(retry_backoff);
                    if !backoff.is_zero() {
                        tokio::time::sleep(backoff).await;
                    }
                }
            }
        }
    }

    if last_timed_out {
        AttemptOutcome {
            attempts: attempts_done,
            kind: OutcomeKind::TimedOut,
        }
    } else {
        AttemptOutcome {
            attempts: attempts_done,
            kind: OutcomeKind::Failed(
                last_err.unwrap_or_else(|| "task failed without context".to_string()),
            ),
        }
    }
}

enum AttemptFail {
    Action(String),
    Timeout,
}

// ---------------------------------------------------------------------------
// Event stream
// ---------------------------------------------------------------------------

/// Live transition emitted while the runner makes progress.
///
/// The serde representation is **internally tagged** on `event` so a
/// downstream consumer can mux several stream kinds onto a single NDJSON
/// channel without losing structural typing.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "event", rename_all = "snake_case")]
pub enum TaskEvent {
    /// The runner has begun executing the graph.
    RunStarted {
        /// Total number of nodes the runner will attempt.
        total_tasks: u32,
        /// Wall-clock instant the event was emitted.
        ts: DateTime<Utc>,
    },
    /// A specific task attempt is starting.
    TaskStarted {
        /// Task identifier.
        id: TaskId,
        /// 1-based attempt counter (`1` for first try).
        attempt: u32,
        /// Wall-clock instant of the start.
        ts: DateTime<Utc>,
    },
    /// Free-form progress message emitted by the runner mid-attempt
    /// (e.g. retry decision, cancellation acknowledgement).
    TaskProgress {
        /// Task identifier.
        id: TaskId,
        /// Human-readable progress line.
        message: String,
        /// Wall-clock instant of the emission.
        ts: DateTime<Utc>,
    },
    /// A task reached its terminal state.
    TaskFinished {
        /// Task identifier.
        id: TaskId,
        /// Terminal status.
        status: TaskStatus,
        /// Total elapsed time from first start to terminal status (ms).
        duration_ms: u64,
        /// Wall-clock instant of completion.
        ts: DateTime<Utc>,
    },
    /// Every node in the graph reached a terminal status.
    RunFinished {
        /// Successfully completed nodes.
        completed: u32,
        /// Failed or timed-out nodes.
        failed: u32,
        /// Nodes skipped because of upstream failures.
        skipped: u32,
        /// Wall-clock instant of completion.
        ts: DateTime<Utc>,
    },
}

// ---------------------------------------------------------------------------
// TaskReport
// ---------------------------------------------------------------------------

/// Final outcome of [`TaskRunner::run`].
#[derive(Debug, Clone)]
pub struct TaskReport {
    /// Per-task bookkeeping snapshot.
    pub nodes: HashMap<TaskId, TaskNode>,
    /// Wall-clock duration of [`TaskRunner::run`] in milliseconds.
    pub total_duration_ms: u64,
    /// Count of nodes that reached [`TaskStatus::Completed`].
    pub succeeded: u32,
    /// Count of nodes that reached [`TaskStatus::Failed`] or
    /// [`TaskStatus::TimedOut`].
    pub failed: u32,
    /// Count of nodes that reached [`TaskStatus::Skipped`].
    pub skipped: u32,
}

impl TaskReport {
    /// `true` when every node completed successfully.
    #[must_use]
    pub fn all_succeeded(&self) -> bool {
        self.failed == 0 && self.skipped == 0 && self.succeeded == self.nodes.len() as u32
    }

    /// Render the per-node snapshot sorted by id for stable output.
    #[must_use]
    pub fn ordered_nodes(&self) -> BTreeMap<&TaskId, &TaskNode> {
        self.nodes.iter().collect()
    }
}

// ---------------------------------------------------------------------------
// TaskError
// ---------------------------------------------------------------------------

/// All recoverable failures surfaced by the runner.
#[derive(Debug, Error)]
pub enum TaskError {
    /// I/O error (reserved for future filesystem-backed graph loaders).
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    /// JSON (de)serialization failure.
    #[error("json error: {0}")]
    Json(#[from] serde_json::Error),
    /// The graph contains a cycle (Kahn's algorithm could not consume
    /// every node).
    #[error("cycle detected involving: {0}")]
    CycleDetected(String),
    /// A task references a dependency id that is not present in the
    /// graph.
    #[error("task '{task}' references missing dependency '{dep}'")]
    MissingDependency {
        /// Task that has the dangling edge.
        task: String,
        /// Missing dependency id.
        dep: String,
    },
    /// Attempted to insert two tasks with the same id.
    #[error("duplicate task id: {0}")]
    DuplicateTask(String),
    /// An action returned an unrecoverable error (or a join handle
    /// panicked).
    #[error("task action failed: {0}")]
    TaskActionFailed(String),
    /// A task exceeded its allotted timeout (reserved; per-task
    /// timeouts surface through [`TaskStatus::TimedOut`]).
    #[error("task timed out: {0}")]
    TimedOut(String),
    /// The runner was cancelled before completing.
    #[error("runner cancelled: {0}")]
    Cancelled(String),
}

// ---------------------------------------------------------------------------
// Unit tests (high-density coverage of the deterministic surface)
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    struct NoopAction;

    #[async_trait]
    impl TaskAction for NoopAction {
        async fn execute(&self, _ctx: &TaskContext) -> Result<TaskOutput, TaskActionError> {
            Ok(serde_json::Value::Null)
        }
    }

    fn arc_noop() -> Arc<dyn TaskAction + Send + Sync> {
        Arc::new(NoopAction)
    }

    #[test]
    fn topo_order_empty_graph_returns_empty_vec() {
        let g = TaskGraph::new();
        assert!(g.topo_order().unwrap().is_empty());
    }

    #[test]
    fn topo_order_independent_nodes_sorted_by_id() {
        let mut g = TaskGraph::new();
        g.add(TaskSpec::leaf("b", "b"), arc_noop()).unwrap();
        g.add(TaskSpec::leaf("a", "a"), arc_noop()).unwrap();
        let order = g.topo_order().unwrap();
        assert_eq!(order[0].as_str(), "a");
        assert_eq!(order[1].as_str(), "b");
    }

    #[test]
    fn add_duplicate_task_fails() {
        let mut g = TaskGraph::new();
        g.add(TaskSpec::leaf("a", "a"), arc_noop()).unwrap();
        let err = g.add(TaskSpec::leaf("a", "a"), arc_noop()).unwrap_err();
        assert!(matches!(err, TaskError::DuplicateTask(_)));
    }

    #[test]
    fn cancel_token_round_trip() {
        let t = CancelToken::new();
        assert!(!t.is_cancelled());
        t.cancel();
        assert!(t.is_cancelled());
    }
}
