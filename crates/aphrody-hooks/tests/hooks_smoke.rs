// SPDX-License-Identifier: Apache-2.0
//! End-to-end smoke tests for the aphrody-hooks dispatcher.
//!
//! Each test exercises a real subprocess (`cmd /c` on Windows, `sh -c`
//! everywhere else). The tests are deliberately short — never more than
//! a few hundred milliseconds even for the timeout scenario — so they
//! stay green on slow CI runners.

use std::collections::HashMap;

use aphrody_hooks::{
    HookCommand, HookConfig, HookDispatcher, HookError, HookEvent, HookMatcher, HookOutcome,
    HookSpec,
};

// ---------------------------------------------------------------------------
// 1. JSON parsing — settings.json compatibility shape.
// ---------------------------------------------------------------------------

#[test]
fn loads_settings_hooks_block() {
    // This mirrors the structure consumed by Code-style runtimes:
    // a top-level "hooks" map keyed by lifecycle kind.
    let raw = serde_json::json!({
        "hooks": {
            "PreToolUse": [
                {
                    "matcher": "Bash",
                    "hooks": [
                        { "type": "command", "command": "echo pre", "timeout_ms": 2000 }
                    ]
                }
            ],
            "Stop": [
                {
                    "hooks": [
                        { "type": "command", "command": "echo stop" }
                    ]
                }
            ]
        }
    });

    let disp = HookDispatcher::load_from_json(&raw).expect("load");
    let cfg = disp.config();
    assert_eq!(cfg.hooks.get("PreToolUse").unwrap().len(), 1);
    assert_eq!(cfg.hooks.get("Stop").unwrap().len(), 1);

    // The dispatcher also accepts a bare hooks map (without the
    // outer "hooks" wrapper) for embedded uses.
    let bare = serde_json::json!({
        "Notification": [
            { "hooks": [{ "type": "command", "command": "echo notify" }] }
        ]
    });
    let disp2 = HookDispatcher::load_from_json(&bare).expect("bare");
    assert_eq!(disp2.config().hooks.get("Notification").unwrap().len(), 1);
}

// ---------------------------------------------------------------------------
// 2. Glob matcher behaviour.
// ---------------------------------------------------------------------------

#[test]
fn matcher_glob_matches_tool_name() {
    let disp = build_dispatcher(vec![(
        "PreToolUse",
        vec![HookSpec {
            matcher: HookMatcher {
                event: None,
                matcher: Some("Edit*".into()),
            },
            hooks: vec![cmd_echo("matched")],
        }],
    )]);

    let event = HookEvent::PreToolUse {
        tool_name: "EditFile".into(),
        tool_input: serde_json::json!({}),
    };
    assert_eq!(disp.match_hooks(&event).len(), 1);

    let event_miss = HookEvent::PreToolUse {
        tool_name: "Bash".into(),
        tool_input: serde_json::json!({}),
    };
    assert_eq!(disp.match_hooks(&event_miss).len(), 0);
}

#[test]
fn matcher_none_matches_all_tools() {
    // Bare PreToolUse hook with no matcher should fire for every tool.
    let disp = build_dispatcher(vec![(
        "PreToolUse",
        vec![HookSpec {
            matcher: HookMatcher::default(),
            hooks: vec![cmd_echo("any")],
        }],
    )]);

    for name in ["Bash", "Edit", "WeirdTool"] {
        let ev = HookEvent::PreToolUse {
            tool_name: name.into(),
            tool_input: serde_json::json!({}),
        };
        assert_eq!(disp.match_hooks(&ev).len(), 1, "expected match for {name}");
    }
}

// ---------------------------------------------------------------------------
// 3. Subprocess execution — runs a real echo command.
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn pre_tool_use_hook_runs_command() {
    let disp = build_dispatcher(vec![(
        "PreToolUse",
        vec![HookSpec {
            matcher: HookMatcher::default(),
            hooks: vec![cmd_echo("aphrody-hooks-smoke")],
        }],
    )]);

    let ev = HookEvent::PreToolUse {
        tool_name: "Bash".into(),
        tool_input: serde_json::json!({"command": "ls"}),
    };
    let results = disp.run_hooks(&ev).await.expect("dispatch");
    assert_eq!(results.len(), 1);
    let r = &results[0];
    assert_eq!(r.exit_code, 0, "stderr={}", r.stderr);
    assert!(
        r.stdout.contains("aphrody-hooks-smoke"),
        "stdout did not contain marker: {:?}",
        r.stdout
    );
    assert_eq!(r.outcome, HookOutcome::Continue);
}

// ---------------------------------------------------------------------------
// 4. Timeout enforcement.
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn hook_timeout_returns_timeout_error() {
    // Sleep ~5s, but only allow 150ms — the dispatcher should kill
    // the wait future and return HookError::Timeout immediately.
    let sleep_cmd = if cfg!(target_os = "windows") {
        // `ping` on Windows sleeps roughly (count - 1) seconds.
        "ping -n 6 127.0.0.1 > nul".to_string()
    } else {
        "sleep 5".to_string()
    };

    let disp = build_dispatcher(vec![(
        "Stop",
        vec![HookSpec {
            matcher: HookMatcher::default(),
            hooks: vec![HookCommand {
                r#type: "command".into(),
                command: sleep_cmd,
                timeout_ms: Some(150),
            }],
        }],
    )]);

    let result = disp.run_hooks(&HookEvent::Stop).await;
    match result {
        Err(HookError::Timeout { elapsed_ms, .. }) => {
            // Should fire close to the budget, certainly well below
            // the underlying sleep duration.
            assert!(
                elapsed_ms < 3_000,
                "timeout fired too late ({elapsed_ms}ms)",
            );
        }
        other => panic!("expected HookError::Timeout, got {other:?}"),
    }
}

// ---------------------------------------------------------------------------
// 5. Block decision parsed from stdout JSON.
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn block_outcome_when_stdout_contains_block_marker() {
    // Drop a script on disk that emits a block decision payload on
    // stdout. Doing it via a temporary file sidesteps the platform
    // shell's quoting rules (cmd.exe `^"` vs sh `'` escaping) and
    // mirrors how real hook scripts get installed under
    // `~/.config/aphrody/hooks/`.
    let dir = tempfile::tempdir().expect("tempdir");
    let payload = r#"{"decision":"block","reason":"policy_violation"}"#;
    let (script_path, command) = if cfg!(target_os = "windows") {
        // The payload file is read via `more`, which is a shell builtin
        // shipped with every Windows install that does not stumble over
        // dotted temp directory names the way `type` does on some MUI
        // locales. Streaming with `<` also avoids cmd.exe's `/s /c`
        // double-quote parsing rules.
        let stdout_file = dir.path().join("out.txt");
        std::fs::write(&stdout_file, format!("{payload}\n")).expect("write");
        // cmd.exe's `more <` does not tolerate quoted paths; the temp
        // file path generated by `tempfile::tempdir` never contains
        // whitespace (form `.tmpXXXXXXXX/out.txt`), so an unquoted
        // form is safe here.
        let cmdline = format!("more < {}", stdout_file.display());
        (stdout_file, cmdline)
    } else {
        let script = dir.path().join("block.sh");
        std::fs::write(&script, format!("#!/bin/sh\nprintf '%s' '{payload}'\n")).expect("write");
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut perm = std::fs::metadata(&script).unwrap().permissions();
            perm.set_mode(0o755);
            std::fs::set_permissions(&script, perm).expect("chmod");
        }
        let cmdline = script.display().to_string();
        (script, cmdline)
    };
    let _keep = script_path; // bind keeps the tempdir reference alive

    let disp = build_dispatcher(vec![(
        "PreToolUse",
        vec![HookSpec {
            matcher: HookMatcher::default(),
            hooks: vec![HookCommand {
                r#type: "command".into(),
                command,
                timeout_ms: Some(2_000),
            }],
        }],
    )]);

    let ev = HookEvent::PreToolUse {
        tool_name: "Bash".into(),
        tool_input: serde_json::json!({"command": "rm -rf /"}),
    };
    let results = disp.run_hooks(&ev).await.expect("dispatch");
    assert_eq!(results.len(), 1);
    let r = &results[0];
    assert_eq!(r.exit_code, 0, "stderr={}", r.stderr);
    assert_eq!(
        r.outcome,
        HookOutcome::Block {
            reason: "policy_violation".into()
        },
        "stdout was {:?}",
        r.stdout
    );
}

// ---------------------------------------------------------------------------
// 6. Stdin payload visibility — the hook should receive the JSON event.
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn hook_receives_event_payload_on_stdin() {
    // Install a tiny script that echoes stdin verbatim, so we can
    // verify the dispatcher pipes the JSON event payload through.
    let dir = tempfile::tempdir().expect("tempdir");
    let (script, command) = if cfg!(target_os = "windows") {
        // `findstr /r ".*"` reads stdin and prints every line that
        // matches "anything" — effectively a portable `cat` shipped
        // with every Windows build. Using a builtin avoids the
        // cmd.exe `/s /c` quoting maze around tempdir paths.
        let placeholder = dir.path().join("placeholder.txt");
        std::fs::write(&placeholder, "ok").expect("write placeholder");
        (placeholder, "findstr /r \".*\"".to_string())
    } else {
        let script = dir.path().join("cat.sh");
        std::fs::write(&script, "#!/bin/sh\ncat\n").expect("write");
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut perm = std::fs::metadata(&script).unwrap().permissions();
            perm.set_mode(0o755);
            std::fs::set_permissions(&script, perm).expect("chmod");
        }
        let cmdline = script.display().to_string();
        (script, cmdline)
    };
    let _keep = script;

    let disp = build_dispatcher(vec![(
        "UserPromptSubmit",
        vec![HookSpec {
            matcher: HookMatcher::default(),
            hooks: vec![HookCommand {
                r#type: "command".into(),
                command,
                timeout_ms: Some(5_000),
            }],
        }],
    )]);

    let ev = HookEvent::UserPromptSubmit {
        prompt: "hello-aphrody".into(),
    };
    let results = disp.run_hooks(&ev).await.expect("dispatch");
    assert_eq!(results.len(), 1);
    let r = &results[0];
    assert_eq!(r.exit_code, 0, "stderr={}", r.stderr);
    assert!(
        r.stdout.contains("hello-aphrody"),
        "stdout missing payload marker: {:?}",
        r.stdout
    );
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn cmd_echo(marker: &str) -> HookCommand {
    HookCommand {
        r#type: "command".into(),
        command: format!("echo {marker}"),
        timeout_ms: Some(2_000),
    }
}

fn build_dispatcher(entries: Vec<(&str, Vec<HookSpec>)>) -> HookDispatcher {
    let mut hooks: HashMap<String, Vec<HookSpec>> = HashMap::new();
    for (k, v) in entries {
        hooks.insert(k.to_string(), v);
    }
    HookDispatcher::new(HookConfig { hooks }).expect("dispatcher")
}
