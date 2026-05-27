// SPDX-License-Identifier: Apache-2.0
//! Tests for [`ApplyPatchTool`]. Hermetic: real disk via `tempfile`.

use super::*;
use aphrody_toolcall::ToolExecutor;
use pretty_assertions::assert_eq;
use serde_json::json;

#[tokio::test]
async fn definition_advertises_apply_patch() {
    let tool = ApplyPatchTool::new();
    let def = tool.definition();
    assert_eq!(def.name, "apply_patch");
    assert!(def.input_schema.properties.is_some());
    let props = def.input_schema.properties.as_ref().unwrap();
    assert!(props.contains_key("patch"));
    assert!(props.contains_key("cwd"));
}

#[tokio::test]
async fn add_then_update_file_on_disk() {
    let dir = tempfile::tempdir().expect("tempdir");
    let root = dir.path().to_path_buf();
    let tool = ApplyPatchTool::new();

    // 1. Add a file.
    let add = concat!(
        "*** Begin Patch\n",
        "*** Add File: greeting.txt\n",
        "+hello\n",
        "+world\n",
        "*** End Patch",
    );
    let out = tool
        .handle(json!({ "patch": add, "cwd": root.to_str().unwrap() }))
        .await
        .expect("handle add");
    assert!(!out.is_error, "add output: {}", out.content);
    assert!(out.content.contains("greeting.txt"));

    let written = std::fs::read_to_string(root.join("greeting.txt")).expect("read added");
    assert_eq!(written, "hello\nworld\n");

    // 2. Update the file we just created.
    let update = concat!(
        "*** Begin Patch\n",
        "*** Update File: greeting.txt\n",
        "@@\n",
        "-hello\n",
        "+HELLO\n",
        " world\n",
        "*** End Patch",
    );
    let out = tool
        .handle(json!({ "patch": update, "cwd": root.to_str().unwrap() }))
        .await
        .expect("handle update");
    assert!(!out.is_error, "update output: {}", out.content);

    let updated = std::fs::read_to_string(root.join("greeting.txt")).expect("read updated");
    assert_eq!(updated, "HELLO\nworld\n");
}

#[tokio::test]
async fn malformed_patch_is_error_output() {
    let tool = ApplyPatchTool::new();
    let out = tool
        .handle(json!({ "patch": "this is not a patch" }))
        .await
        .expect("handle returns Ok with error output");
    assert!(out.is_error);
    assert!(out.content.contains("parse error"), "content: {}", out.content);
}

#[tokio::test]
async fn apply_failure_is_error_output() {
    let dir = tempfile::tempdir().expect("tempdir");
    let root = dir.path().to_path_buf();
    let tool = ApplyPatchTool::new();

    // Updating a file that does not exist must surface an apply error, not panic.
    let update = concat!(
        "*** Begin Patch\n",
        "*** Update File: missing.txt\n",
        "@@\n",
        "-old\n",
        "+new\n",
        "*** End Patch",
    );
    let out = tool
        .handle(json!({ "patch": update, "cwd": root.to_str().unwrap() }))
        .await
        .expect("handle");
    assert!(out.is_error);
    assert!(out.content.contains("apply_patch:"), "content: {}", out.content);
}

#[tokio::test]
async fn missing_patch_argument_is_invalid_arguments() {
    let tool = ApplyPatchTool::new();
    let err = tool
        .handle(json!({ "cwd": "." }))
        .await
        .expect_err("missing `patch` must be InvalidArguments");
    assert!(matches!(err, ToolError::InvalidArguments { .. }));
}
