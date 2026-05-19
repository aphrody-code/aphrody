// SPDX-License-Identifier: Apache-2.0
//! Integration smoke suite for `aphrody-tools`.
//!
//! Covers the registry lifecycle (`register`/`get`/`list`/queries), the
//! validation engine (required fields, additional properties, type checks),
//! and round-trips through the Anthropic/Gemini/MCP wire shapes.

use aphrody_tools::{ToolDescriptor, ToolRegistry, ToolsError};
use serde_json::json;

fn read_tool() -> ToolDescriptor {
    ToolDescriptor::new(
        "read",
        "Read a UTF-8 file from disk and return its contents.",
        json!({
            "type": "object",
            "properties": {
                "path": {"type": "string"},
                "limit": {"type": "integer"}
            },
            "required": ["path"],
            "additionalProperties": false
        }),
    )
    .expect("valid name")
    .with_tag("fs")
    .with_tag("read")
}

fn write_tool() -> ToolDescriptor {
    ToolDescriptor::new(
        "write-file",
        "Write bytes to a path. Overwrites existing files.",
        json!({
            "type": "object",
            "properties": {
                "path": {"type": "string"},
                "contents": {"type": "string"}
            },
            "required": ["path", "contents"]
        }),
    )
    .expect("valid name")
    .with_tag("fs")
    .dangerous()
}

fn bash_tool() -> ToolDescriptor {
    ToolDescriptor::new(
        "Bash",
        "Run a shell command.",
        json!({
            "type": "object",
            "properties": {
                "command": {"type": "string"}
            },
            "required": ["command"]
        }),
    )
    .expect("valid name")
    .with_tag("system")
    .dangerous()
}

// ---------------------------------------------------------------------------
// Registry lifecycle
// ---------------------------------------------------------------------------

#[test]
fn register_and_get_tool() {
    let mut reg = ToolRegistry::new();
    reg.register(read_tool()).expect("register ok");
    let got = reg.get("read").expect("found");
    assert_eq!(got.name, "read");
    assert_eq!(reg.len(), 1);
    assert!(!reg.is_empty());
}

#[test]
fn register_duplicate_returns_error() {
    let mut reg = ToolRegistry::new();
    reg.register(read_tool()).expect("first ok");
    let err = reg.register(read_tool()).expect_err("second must fail");
    assert!(matches!(err, ToolsError::DuplicateTool(ref name) if name == "read"));
}

#[test]
fn register_many_inserts_all() {
    let mut reg = ToolRegistry::new();
    reg.register_many(vec![read_tool(), write_tool(), bash_tool()])
        .expect("bulk ok");
    assert_eq!(reg.len(), 3);
    let names: Vec<&str> = reg.list().iter().map(|t| t.name.as_str()).collect();
    assert_eq!(names, vec!["Bash", "read", "write-file"]);
}

#[test]
fn find_by_tag_filters() {
    let mut reg = ToolRegistry::new();
    reg.register_many(vec![read_tool(), write_tool(), bash_tool()])
        .expect("bulk ok");
    let fs_tools: Vec<&str> = reg.find_by_tag("fs").iter().map(|t| t.name.as_str()).collect();
    assert_eq!(fs_tools, vec!["read", "write-file"]);
    let sys_tools: Vec<&str> = reg
        .find_by_tag("system")
        .iter()
        .map(|t| t.name.as_str())
        .collect();
    assert_eq!(sys_tools, vec!["Bash"]);
    assert!(reg.find_by_tag("missing").is_empty());
}

#[test]
fn find_dangerous_returns_writes() {
    let mut reg = ToolRegistry::new();
    reg.register_many(vec![read_tool(), write_tool(), bash_tool()])
        .expect("bulk ok");
    let dangerous: Vec<&str> = reg
        .find_dangerous()
        .iter()
        .map(|t| t.name.as_str())
        .collect();
    assert_eq!(dangerous, vec!["Bash", "write-file"]);
    // dangerous() also flips requires_permission on.
    let perm_required: Vec<&str> = reg
        .find_permission_required()
        .iter()
        .map(|t| t.name.as_str())
        .collect();
    assert_eq!(perm_required, vec!["Bash", "write-file"]);
}

#[test]
fn invalid_tool_name_rejected() {
    let err = ToolDescriptor::new("1bad-name", "x", json!({"type": "object"}))
        .expect_err("must fail");
    assert!(matches!(err, ToolsError::InvalidName(ref n) if n == "1bad-name"));
}

#[test]
fn register_with_non_object_schema_errors() {
    let mut reg = ToolRegistry::new();
    let bogus = ToolDescriptor::new("foo", "x", json!({"type": "string"})).expect("name ok");
    let err = reg.register(bogus).expect_err("schema must be object");
    assert!(matches!(err, ToolsError::InvalidSchema(ref n) if n == "foo"));
}

// ---------------------------------------------------------------------------
// validate_args
// ---------------------------------------------------------------------------

#[test]
fn validate_args_accepts_valid_object() {
    let mut reg = ToolRegistry::new();
    reg.register(read_tool()).unwrap();
    reg.validate_args("read", &json!({"path": "/tmp/x"})).expect("ok");
    reg.validate_args("read", &json!({"path": "/tmp/x", "limit": 100}))
        .expect("ok");
}

#[test]
fn validate_args_rejects_missing_required() {
    let mut reg = ToolRegistry::new();
    reg.register(read_tool()).unwrap();
    let err = reg
        .validate_args("read", &json!({"limit": 100}))
        .expect_err("missing path");
    assert!(matches!(err, ToolsError::MissingRequiredField(ref f) if f == "path"));
}

#[test]
fn validate_args_rejects_additional_property() {
    let mut reg = ToolRegistry::new();
    reg.register(read_tool()).unwrap();
    let err = reg
        .validate_args("read", &json!({"path": "/tmp", "unknown": 1}))
        .expect_err("unknown property");
    assert!(matches!(err, ToolsError::AdditionalProperty(ref f) if f == "unknown"));
}

#[test]
fn validate_args_rejects_type_mismatch() {
    let mut reg = ToolRegistry::new();
    reg.register(read_tool()).unwrap();
    let err = reg
        .validate_args("read", &json!({"path": 42}))
        .expect_err("type mismatch");
    assert!(matches!(
        err,
        ToolsError::TypeMismatch { ref field, .. } if field == "path"
    ));
}

#[test]
fn validate_args_unknown_tool_errors() {
    let reg = ToolRegistry::new();
    let err = reg
        .validate_args("nope", &json!({}))
        .expect_err("not found");
    assert!(matches!(err, ToolsError::NotFound(ref n) if n == "nope"));
}

// ---------------------------------------------------------------------------
// Vendor exporters
// ---------------------------------------------------------------------------

#[test]
fn to_anthropic_tool_json_has_correct_shape() {
    let mut reg = ToolRegistry::new();
    reg.register(read_tool()).unwrap();
    let v = reg.to_anthropic_tool_json("read").expect("export");
    let obj = v.as_object().expect("object");
    assert_eq!(obj.get("name").and_then(|v| v.as_str()), Some("read"));
    assert!(obj.get("description").is_some());
    assert!(obj.get("input_schema").is_some());
    // Anthropic keeps the underscore naming convention.
    assert!(obj.get("parameters").is_none());
    assert!(obj.get("inputSchema").is_none());
}

#[test]
fn to_gemini_function_declaration_has_correct_shape() {
    let mut reg = ToolRegistry::new();
    reg.register(read_tool()).unwrap();
    let v = reg.to_gemini_function_declaration("read").expect("export");
    let obj = v.as_object().expect("object");
    assert_eq!(obj.get("name").and_then(|v| v.as_str()), Some("read"));
    assert!(obj.get("parameters").is_some());
    assert!(obj.get("input_schema").is_none());
    assert!(obj.get("inputSchema").is_none());
}

#[test]
fn to_mcp_tool_json_has_correct_shape() {
    let mut reg = ToolRegistry::new();
    reg.register(read_tool()).unwrap();
    let v = reg.to_mcp_tool_json("read").expect("export");
    let obj = v.as_object().expect("object");
    assert_eq!(obj.get("name").and_then(|v| v.as_str()), Some("read"));
    assert!(obj.get("inputSchema").is_some());
    // MCP uses camelCase.
    assert!(obj.get("input_schema").is_none());
    assert!(obj.get("parameters").is_none());
}

#[test]
fn export_unknown_tool_errors() {
    let reg = ToolRegistry::new();
    assert!(matches!(
        reg.to_anthropic_tool_json("missing"),
        Err(ToolsError::NotFound(_))
    ));
    assert!(matches!(
        reg.to_gemini_function_declaration("missing"),
        Err(ToolsError::NotFound(_))
    ));
    assert!(matches!(
        reg.to_mcp_tool_json("missing"),
        Err(ToolsError::NotFound(_))
    ));
}

// ---------------------------------------------------------------------------
// Vendor importers
// ---------------------------------------------------------------------------

#[test]
fn from_anthropic_tools_roundtrip() {
    let mut reg = ToolRegistry::new();
    reg.register_many(vec![read_tool(), write_tool()]).unwrap();
    let exported = json!([
        reg.to_anthropic_tool_json("read").unwrap(),
        reg.to_anthropic_tool_json("write-file").unwrap(),
    ]);
    let parsed = ToolRegistry::from_anthropic_tools(&exported).expect("parse");
    assert_eq!(parsed.len(), 2);
    let names: Vec<&str> = parsed.iter().map(|t| t.name.as_str()).collect();
    assert!(names.contains(&"read"));
    assert!(names.contains(&"write-file"));
}

#[test]
fn from_mcp_tools_accepts_wrapped_and_bare_arrays() {
    let mut reg = ToolRegistry::new();
    reg.register(read_tool()).unwrap();
    let one = reg.to_mcp_tool_json("read").unwrap();

    // Bare array form
    let bare = json!([one.clone()]);
    let parsed1 = ToolRegistry::from_mcp_tools(&bare).expect("parse bare");
    assert_eq!(parsed1.len(), 1);
    assert_eq!(parsed1[0].name, "read");

    // Wrapped `result.tools` form (per MCP `tools/list` schema)
    let wrapped = json!({"tools": [one]});
    let parsed2 = ToolRegistry::from_mcp_tools(&wrapped).expect("parse wrapped");
    assert_eq!(parsed2.len(), 1);
    assert_eq!(parsed2[0].name, "read");
}

#[test]
fn from_anthropic_tools_rejects_missing_input_schema() {
    let bad = json!([{"name": "foo", "description": "x"}]);
    let err = ToolRegistry::from_anthropic_tools(&bad).expect_err("missing schema");
    assert!(matches!(err, ToolsError::InvalidSchema(_)));
}

#[test]
fn from_mcp_tools_rejects_invalid_name() {
    let bad = json!({"tools": [{"name": "1bad", "description": "x", "inputSchema": {"type": "object"}}]});
    let err = ToolRegistry::from_mcp_tools(&bad).expect_err("invalid name");
    assert!(matches!(err, ToolsError::InvalidName(_)));
}
