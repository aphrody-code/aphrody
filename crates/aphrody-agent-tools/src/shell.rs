// SPDX-License-Identifier: Apache-2.0
//! The `shell` tool: a sandboxable, streaming command executor.

use std::collections::BTreeMap;
use std::path::PathBuf;
use std::process::Stdio;
use std::sync::Arc;
use std::time::Duration;

use aphrody_toolcall::AdditionalProperties;
use aphrody_toolcall::JsonSchema;
use aphrody_toolcall::ToolDefinition;
use aphrody_toolcall::ToolError;
use aphrody_toolcall::ToolExecutor;
use aphrody_toolcall::ToolOutput;
use serde::Deserialize;
use tokio::io::AsyncReadExt;
use tokio::process::Command;

/// Default per-command wall-clock timeout (30 seconds).
pub const DEFAULT_TIMEOUT: Duration = Duration::from_secs(30);

/// Default cap on captured combined output, in bytes (64 KiB).
pub const DEFAULT_MAX_OUTPUT_BYTES: usize = 64 * 1024;

/// A sink for streamed output chunks.
///
/// When configured on a [`ShellExecTool`], each chunk read from the child's
/// combined stdout/stderr stream is forwarded here as soon as it arrives, in
/// addition to being buffered for the final [`ToolOutput`]. The engine can use
/// this to emit
/// [`EventMsg::ExecCommandOutputDelta`](aphrody_agent_proto::EventMsg::ExecCommandOutputDelta)
/// events in real time.
pub type OutputSink = Arc<dyn Fn(String) + Send + Sync>;

/// Tunable, opt-in guardrails for [`ShellExecTool`].
///
/// All fields default to permissive-but-bounded values; nothing here blocks a
/// command from running. A request may still override the timeout via its
/// `timeout_ms` argument.
#[derive(Clone)]
pub struct ShellExecConfig {
    /// Wall-clock timeout applied when a request omits `timeout_ms`.
    pub default_timeout: Duration,
    /// Maximum number of bytes retained from the combined output. Output beyond
    /// this is dropped from the buffer (a truncation notice is appended); the
    /// streaming sink, if any, still receives every chunk.
    pub max_output_bytes: usize,
}

impl Default for ShellExecConfig {
    fn default() -> Self {
        Self {
            default_timeout: DEFAULT_TIMEOUT,
            max_output_bytes: DEFAULT_MAX_OUTPUT_BYTES,
        }
    }
}

impl std::fmt::Debug for ShellExecConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ShellExecConfig")
            .field("default_timeout", &self.default_timeout)
            .field("max_output_bytes", &self.max_output_bytes)
            .finish()
    }
}

/// Decoded arguments for the `shell` tool.
#[derive(Debug, Deserialize)]
struct ShellArgs {
    /// The command to run as an argv vector (`["git", "status"]`). The first
    /// element is the executable; remaining elements are arguments. No shell is
    /// involved, so quoting/globbing/redirection are not interpreted.
    command: Vec<String>,
    /// Working directory; defaults to the current process directory.
    #[serde(default)]
    cwd: Option<String>,
    /// Per-call timeout override, in milliseconds.
    #[serde(default)]
    timeout_ms: Option<u64>,
}

/// Runs a command and reports its exit code and (possibly truncated) output.
///
/// Commands are launched directly by their executable name — there is no
/// `sh -c` / `cmd /c` wrapper — so the argv is passed through verbatim and the
/// behavior is identical on Linux and Windows.
pub struct ShellExecTool {
    definition: ToolDefinition,
    config: ShellExecConfig,
    sink: Option<OutputSink>,
}

impl std::fmt::Debug for ShellExecTool {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ShellExecTool")
            .field("config", &self.config)
            .field("streaming", &self.sink.is_some())
            .finish()
    }
}

impl Default for ShellExecTool {
    fn default() -> Self {
        Self::new()
    }
}

impl ShellExecTool {
    /// Create a permissive shell tool with default bounds and no streaming.
    #[must_use]
    pub fn new() -> Self {
        Self::with_config(ShellExecConfig::default())
    }

    /// Create a shell tool with explicit [`ShellExecConfig`] bounds.
    #[must_use]
    pub fn with_config(config: ShellExecConfig) -> Self {
        Self {
            definition: build_definition(),
            config,
            sink: None,
        }
    }

    /// Attach a streaming [`OutputSink`] (builder style). Every output chunk is
    /// forwarded to the sink as it is read, in addition to being buffered.
    #[must_use]
    pub fn with_sink(mut self, sink: OutputSink) -> Self {
        self.sink = Some(sink);
        self
    }
}

/// Build the model-visible definition for the `shell` tool.
fn build_definition() -> ToolDefinition {
    let mut properties = BTreeMap::new();
    properties.insert(
        "command".to_string(),
        JsonSchema::array(
            JsonSchema::string(None),
            Some(
                "The command as an argv vector, e.g. [\"git\", \"status\"]. The \
                 first element is the executable; no shell is involved, so quoting, \
                 globbing, and redirection are not interpreted."
                    .to_string(),
            ),
        ),
    );
    properties.insert(
        "cwd".to_string(),
        JsonSchema::string(Some(
            "Working directory for the command. Defaults to the agent's current \
             directory."
                .to_string(),
        )),
    );
    properties.insert(
        "timeout_ms".to_string(),
        JsonSchema::integer(Some(
            "Wall-clock timeout in milliseconds. Defaults to 30000.".to_string(),
        )),
    );

    let input_schema = JsonSchema::object(
        properties,
        Some(vec!["command".to_string()]),
        Some(AdditionalProperties::Boolean(false)),
    );

    let mut output_properties = BTreeMap::new();
    output_properties.insert(
        "exit_code".to_string(),
        JsonSchema::integer(Some("Process exit code (-1 if terminated by signal).".to_string())),
    );
    output_properties.insert(
        "output".to_string(),
        JsonSchema::string(Some("Combined stdout and stderr, possibly truncated.".to_string())),
    );
    let output_schema = JsonSchema::object(
        output_properties,
        Some(vec!["exit_code".to_string(), "output".to_string()]),
        Some(AdditionalProperties::Boolean(true)),
    );

    ToolDefinition::new(
        "shell",
        "Run a command and capture its combined stdout/stderr output and exit \
         code. The command is given as an argv vector and executed directly \
         (no shell interpretation). Long output is truncated.",
        input_schema,
    )
    .with_output_schema(output_schema)
}

/// Outcome of running a child process to completion or timeout.
enum RunOutcome {
    /// The process exited; `code` is its exit status (`-1` if killed by signal).
    Exited { code: i32, output: String, truncated: bool },
    /// The process exceeded its timeout and was killed.
    TimedOut,
}

#[async_trait::async_trait]
impl ToolExecutor for ShellExecTool {
    fn definition(&self) -> &ToolDefinition {
        &self.definition
    }

    async fn handle(&self, arguments: serde_json::Value) -> Result<ToolOutput, ToolError> {
        let args: ShellArgs = serde_json::from_value(arguments).map_err(|err| {
            ToolError::InvalidArguments {
                tool: "shell".to_string(),
                message: err.to_string(),
            }
        })?;

        let Some((program, rest)) = args.command.split_first() else {
            return Ok(ToolOutput::error("shell: `command` must be a non-empty argv vector"));
        };

        // Command-safety backstop. Opt-in via APHRODY_GUARD (a no-op by default,
        // per the autonomy contract); when enabled, a provably-destructive
        // command is refused here and never spawned.
        if let Some(refusal) = forbidden_refusal(&args.command, aphrody_guard::guardrails_enabled())
        {
            tracing::warn!(
                command = ?args.command,
                "shell: refused known-destructive command (APHRODY_GUARD)"
            );
            return Ok(refusal);
        }

        let timeout = args
            .timeout_ms
            .map_or(self.config.default_timeout, Duration::from_millis);

        let mut command = Command::new(program);
        command.args(rest);
        if let Some(cwd) = args.cwd.as_ref() {
            command.current_dir(PathBuf::from(cwd));
        }
        command
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .kill_on_drop(true);

        let outcome = run_child(command, timeout, self.config.max_output_bytes, self.sink.as_ref())
            .await
            .map_err(|message| ToolError::Execution {
                tool: "shell".to_string(),
                message,
            })?;

        match outcome {
            RunOutcome::TimedOut => Ok(ToolOutput::error(format!(
                "shell: command `{}` timed out after {} ms",
                program,
                timeout.as_millis()
            ))),
            RunOutcome::Exited {
                code,
                output,
                truncated,
            } => {
                let body = if truncated {
                    format!(
                        "exit_code: {code}\n--- output (truncated to {} bytes) ---\n{output}",
                        self.config.max_output_bytes
                    )
                } else {
                    format!("exit_code: {code}\n--- output ---\n{output}")
                };
                // A non-zero exit is reported to the model but is not a tool
                // *error*: the command ran and produced a result.
                Ok(ToolOutput::ok(body))
            }
        }
    }
}

/// Spawn `command`, pump its combined output, and enforce `timeout`.
///
/// Returns `Err(String)` only when the process cannot be spawned or an I/O
/// error occurs while reading; a non-zero exit or a timeout are normal
/// [`RunOutcome`]s.
async fn run_child(
    mut command: Command,
    timeout: Duration,
    max_output_bytes: usize,
    sink: Option<&OutputSink>,
) -> Result<RunOutcome, String> {
    let mut child = command
        .spawn()
        .map_err(|err| format!("failed to spawn process: {err}"))?;

    let mut stdout = child
        .stdout
        .take()
        .ok_or_else(|| "failed to capture stdout".to_string())?;
    let mut stderr = child
        .stderr
        .take()
        .ok_or_else(|| "failed to capture stderr".to_string())?;

    let mut buffer = String::new();
    let mut truncated = false;

    // Read both pipes concurrently with the wait, under a single deadline.
    let pump = async {
        let mut out_buf = [0u8; 8192];
        let mut err_buf = [0u8; 8192];
        let mut out_open = true;
        let mut err_open = true;

        loop {
            tokio::select! {
                read = stdout.read(&mut out_buf), if out_open => {
                    match read {
                        Ok(0) => out_open = false,
                        Ok(n) => push_chunk(&out_buf[..n], &mut buffer, &mut truncated, max_output_bytes, sink),
                        Err(err) => return Err(format!("error reading stdout: {err}")),
                    }
                }
                read = stderr.read(&mut err_buf), if err_open => {
                    match read {
                        Ok(0) => err_open = false,
                        Ok(n) => push_chunk(&err_buf[..n], &mut buffer, &mut truncated, max_output_bytes, sink),
                        Err(err) => return Err(format!("error reading stderr: {err}")),
                    }
                }
                else => break,
            }
        }
        Ok(())
    };

    let status = tokio::select! {
        result = async {
            pump.await?;
            child.wait().await.map_err(|err| format!("error waiting for process: {err}"))
        } => result,
        () = tokio::time::sleep(timeout) => {
            let _ = child.start_kill();
            let _ = child.wait().await;
            return Ok(RunOutcome::TimedOut);
        }
    }?;

    let code = status.code().unwrap_or(-1);
    Ok(RunOutcome::Exited {
        code,
        output: buffer,
        truncated,
    })
}

/// Append a raw byte chunk to `buffer` (lossy UTF-8), respecting the byte cap,
/// and forward it to the streaming sink if present.
fn push_chunk(
    bytes: &[u8],
    buffer: &mut String,
    truncated: &mut bool,
    max_output_bytes: usize,
    sink: Option<&OutputSink>,
) {
    let text = String::from_utf8_lossy(bytes);
    if let Some(sink) = sink {
        sink(text.clone().into_owned());
    }
    if buffer.len() >= max_output_bytes {
        *truncated = true;
        return;
    }
    let room = max_output_bytes - buffer.len();
    if text.len() <= room {
        buffer.push_str(&text);
    } else {
        // Append a UTF-8-safe prefix that fits the remaining room.
        let mut end = room;
        while end > 0 && !text.is_char_boundary(end) {
            end -= 1;
        }
        buffer.push_str(&text[..end]);
        *truncated = true;
    }
}

/// Decide whether `command` must be refused *before* spawning, per the
/// command-safety guardrail.
///
/// When `guard_enabled` is `true` (i.e. `APHRODY_GUARD` is opted in) and
/// `aphrody-guard` proves the command **known-destructive**
/// ([`Decision::Forbidden`](aphrody_guard::Decision::Forbidden) — `rm -rf /`,
/// `git push --force`, `dd`, a fork bomb, …), this returns a refusal
/// [`ToolOutput`] and the caller must not spawn it: such a command is never
/// auto-run. `Allow` and `Prompt` commands return `None` and still run, so
/// autonomy is preserved — only the destructive backstop is hard. With the
/// guard off (the default) this is always `None`.
///
/// Kept as a pure, env-free function (the env is read once by the caller) so it
/// is deterministically testable without mutating process-global state.
fn forbidden_refusal(command: &[String], guard_enabled: bool) -> Option<ToolOutput> {
    if !guard_enabled {
        return None;
    }
    if aphrody_guard::classify_command(command) != aphrody_guard::Decision::Forbidden {
        return None;
    }
    let program = command.first().map_or("<empty>", String::as_str);
    Some(ToolOutput::error(format!(
        "shell: refused to run `{program}` — aphrody-guard command-safety classified it as \
         known-destructive and APHRODY_GUARD is enabled, so it is never auto-run."
    )))
}

#[cfg(test)]
#[path = "shell_tests.rs"]
mod tests;
