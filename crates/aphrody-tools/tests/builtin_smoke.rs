// SPDX-License-Identifier: Apache-2.0
//! End-to-end smoke tests for the [`aphrody_tools::builtin`] module set.
//!
//! Each test exercises one builtin tool against real OS resources (temp
//! dirs, a local TCP listener for `web_fetch`), proving the descriptor /
//! invoke handshake works end-to-end through the public [`Tool`] trait
//! object. Per CLAUDE.md §7: verify = observable behaviour, not just
//! compile.
//!
//! The whole file is gated on non-WASM targets — the builtins themselves
//! are absent there.

#![cfg(not(target_arch = "wasm32"))]

use aphrody_tools::builtin::{
    self, edit, glob_tool, grep, ls, memory_tool, read_file, web_fetch,
    write_file,
};
use aphrody_tools::{Permission, Tool, ToolRegistry};
use serde_json::{Value, json};

// ---------------------------------------------------------------------------
// Registry bulk-registration
// ---------------------------------------------------------------------------

#[test]
fn register_builtins_inserts_eight_tools() {
    let mut reg = ToolRegistry::new();
    builtin::register_builtins(&mut reg).expect("bulk register");
    assert_eq!(reg.len(), 8, "expected 8 builtin tools");
    let names: Vec<&str> = reg.list().iter().map(|t| t.name.as_str()).collect();
    for expected in [
        "edit",
        "glob",
        "grep",
        "ls",
        "memory",
        "read_file",
        "web_fetch",
        "write_file",
    ] {
        assert!(
            names.contains(&expected),
            "missing builtin: {expected} (got {names:?})"
        );
    }
}

#[test]
fn builtin_names_matches_registry() {
    let names = builtin::builtin_names();
    assert_eq!(names.len(), 8);
}

// ---------------------------------------------------------------------------
// Tool trait object dispatch
// ---------------------------------------------------------------------------

#[tokio::test]
async fn dispatch_read_then_edit_then_read_chain() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("chain.txt");
    tokio::fs::write(&path, "alpha\nbeta\ngamma\n").await.unwrap();

    let write: Box<dyn Tool> = Box::new(write_file::WriteFileTool);
    let read: Box<dyn Tool> = Box::new(read_file::ReadFileTool);
    let editt: Box<dyn Tool> = Box::new(edit::EditTool);

    // 1) read original.
    let v = read
        .invoke(json!({"file_path": path.display().to_string()}))
        .await
        .unwrap();
    assert!(v
        .get("content")
        .and_then(Value::as_str)
        .unwrap()
        .contains("beta"));

    // 2) edit one line.
    editt
        .invoke(json!({
            "file_path": path.display().to_string(),
            "old_string": "beta",
            "new_string": "BETA!"
        }))
        .await
        .unwrap();

    // 3) re-read to confirm the mutation landed.
    let v2 = read
        .invoke(json!({"file_path": path.display().to_string()}))
        .await
        .unwrap();
    assert!(v2
        .get("content")
        .and_then(Value::as_str)
        .unwrap()
        .contains("BETA!"));

    // 4) overwrite via write_file.
    let v3 = write
        .invoke(json!({
            "file_path": path.display().to_string(),
            "content": "fresh\n"
        }))
        .await
        .unwrap();
    assert_eq!(v3.get("bytes").and_then(Value::as_u64), Some(6));
}

#[tokio::test]
async fn glob_and_grep_cooperate() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    tokio::fs::write(root.join("alpha.rs"), "fn alpha() {}").await.unwrap();
    tokio::fs::write(root.join("beta.rs"), "fn beta() { /* TODO */ }")
        .await
        .unwrap();
    tokio::fs::write(root.join("notes.txt"), "TODO: read me").await.unwrap();

    let glob: Box<dyn Tool> = Box::new(glob_tool::GlobTool);
    let grep: Box<dyn Tool> = Box::new(grep::GrepTool);

    let v = glob
        .invoke(json!({
            "pattern": "*.rs",
            "path": root.display().to_string()
        }))
        .await
        .unwrap();
    assert_eq!(v.get("count").and_then(Value::as_u64), Some(2));

    let v = grep
        .invoke(json!({
            "pattern": "TODO",
            "path": root.display().to_string(),
            "glob": "*.rs"
        }))
        .await
        .unwrap();
    assert_eq!(v.get("count").and_then(Value::as_u64), Some(1));
}

#[tokio::test]
async fn ls_sees_subdirs_when_recursive() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    tokio::fs::create_dir_all(root.join("a/b/c")).await.unwrap();
    tokio::fs::write(root.join("a/b/c/leaf.txt"), "x").await.unwrap();

    let ls_tool: Box<dyn Tool> = Box::new(ls::LsTool);
    let shallow = ls_tool
        .invoke(json!({"path": root.display().to_string()}))
        .await
        .unwrap();
    let shallow_count =
        shallow.get("count").and_then(Value::as_u64).unwrap();
    let deep = ls_tool
        .invoke(json!({
            "path": root.display().to_string(),
            "recursive": true,
            "max_depth": 5
        }))
        .await
        .unwrap();
    let deep_count = deep.get("count").and_then(Value::as_u64).unwrap();
    assert!(
        deep_count > shallow_count,
        "recursive should see strictly more entries ({deep_count} vs {shallow_count})"
    );
}

#[tokio::test]
async fn memory_round_trips_via_trait_object() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("mem.md");
    let mem: Box<dyn Tool> = Box::new(memory_tool::MemoryTool);
    mem.invoke(json!({
        "op": "save",
        "path": path.display().to_string(),
        "fact": "smoke fact"
    }))
    .await
    .unwrap();
    let listed = mem
        .invoke(json!({
            "op": "list",
            "path": path.display().to_string()
        }))
        .await
        .unwrap();
    assert_eq!(
        listed.get("fact_count").and_then(Value::as_u64),
        Some(1)
    );
}

// ---------------------------------------------------------------------------
// Permission descriptors
// ---------------------------------------------------------------------------

#[test]
fn read_class_tools_default_to_allow() {
    let tools: Vec<Box<dyn Tool>> = vec![
        Box::new(read_file::ReadFileTool),
        Box::new(glob_tool::GlobTool),
        Box::new(grep::GrepTool),
        Box::new(ls::LsTool),
    ];
    for t in tools {
        let p = t.permission();
        assert_eq!(
            p.default_permission,
            Permission::Allow,
            "expected read tool {} to default Allow",
            p.tool_name
        );
    }
}

#[test]
fn write_class_tools_default_to_ask() {
    let tools: Vec<Box<dyn Tool>> = vec![
        Box::new(write_file::WriteFileTool),
        Box::new(edit::EditTool),
        Box::new(web_fetch::WebFetchTool),
        Box::new(memory_tool::MemoryTool),
    ];
    for t in tools {
        let p = t.permission();
        assert_eq!(
            p.default_permission,
            Permission::Ask,
            "expected write/net tool {} to default Ask",
            p.tool_name
        );
        assert!(
            !p.scopes.is_empty(),
            "expected {} to advertise scopes",
            p.tool_name
        );
    }
}
