// SPDX-License-Identifier: Apache-2.0
//! Unit tests for the tool catalogue model.
//!
//! The async `ToolExecutor` trait is exercised without pulling in a runtime
//! (`tokio`/`futures` are not dependencies). A runtime-free `async fn` that
//! never awaits real I/O completes on its first `poll`, so we drive the boxed
//! future returned by `async-trait` to completion with a no-op waker.

use std::collections::BTreeMap;
use std::future::Future;
use std::sync::Arc;
use std::task::Context;
use std::task::Poll;
use std::task::RawWaker;
use std::task::RawWakerVTable;
use std::task::Waker;

use pretty_assertions::assert_eq;
use serde_json::Value as JsonValue;
use serde_json::json;

use super::AdditionalProperties;
use super::JsonSchema;
use super::JsonSchemaPrimitiveType;
use super::JsonSchemaType;
use super::ToolDefinition;
use super::ToolError;
use super::ToolExecutor;
use super::ToolExposure;
use super::ToolOutput;
use super::ToolRegistry;

// --- minimal no-runtime future driver -------------------------------------

const NOOP_VTABLE: RawWakerVTable =
    RawWakerVTable::new(|_| RawWaker::new(std::ptr::null(), &NOOP_VTABLE), |_| {}, |_| {}, |_| {});

fn noop_waker() -> Waker {
    // SAFETY: the vtable's clone/wake/wake_by_ref/drop are all no-ops that do
    // not dereference the (null) data pointer, satisfying the Waker contract.
    unsafe { Waker::from_raw(RawWaker::new(std::ptr::null(), &NOOP_VTABLE)) }
}

/// Drive a future to completion, expecting it to be ready on the first poll.
fn block_on<F: Future>(future: F) -> F::Output {
    let mut future = Box::pin(future);
    let waker = noop_waker();
    let mut cx = Context::from_waker(&waker);
    match future.as_mut().poll(&mut cx) {
        Poll::Ready(value) => value,
        Poll::Pending => panic!("runtime-free future unexpectedly returned Pending"),
    }
}

// --- fixtures --------------------------------------------------------------

/// A deeply nested object schema used to exercise compaction.
fn big_schema() -> JsonSchema {
    let leaf = JsonSchema::string(Some("a verbose description ".repeat(40)));

    let mut level3 = BTreeMap::new();
    level3.insert("deep_a".to_string(), leaf.clone());
    level3.insert("deep_b".to_string(), leaf.clone());
    let level3_obj = JsonSchema::object(level3, Some(vec!["deep_a".to_string()]), None);

    let mut level2 = BTreeMap::new();
    level2.insert("nested".to_string(), level3_obj);
    level2.insert("other".to_string(), leaf.clone());
    let level2_obj = JsonSchema::object(level2, None, None);

    let mut level1 = BTreeMap::new();
    level1.insert("first".to_string(), level2_obj);
    level1.insert("second".to_string(), JsonSchema::array(leaf.clone(), None));

    let mut root = JsonSchema::object(level1, Some(vec!["first".to_string()]), None);
    root.description = Some("the root description ".repeat(40));

    let mut defs = BTreeMap::new();
    defs.insert("Reusable".to_string(), JsonSchema::object(BTreeMap::new(), None, None));
    root.defs = Some(defs);
    root
}

struct EchoTool {
    definition: ToolDefinition,
}

impl EchoTool {
    fn new(name: &str, exposure: ToolExposure) -> Self {
        let mut props = BTreeMap::new();
        props.insert("text".to_string(), JsonSchema::string(Some("text to echo".into())));
        let input = JsonSchema::object(props, Some(vec!["text".to_string()]), None);
        Self {
            definition: ToolDefinition::new(name, "echoes the text argument", input)
                .with_exposure(exposure),
        }
    }
}

#[async_trait::async_trait]
impl ToolExecutor for EchoTool {
    fn definition(&self) -> &ToolDefinition {
        &self.definition
    }

    async fn handle(&self, arguments: JsonValue) -> Result<ToolOutput, ToolError> {
        match arguments.get("text").and_then(JsonValue::as_str) {
            Some(text) => Ok(ToolOutput::ok(text.to_string())),
            None => Err(ToolError::InvalidArguments {
                tool: self.definition.name.clone(),
                message: "missing `text`".into(),
            }),
        }
    }
}

// --- compaction tests ------------------------------------------------------

#[test]
fn compact_brings_large_schema_under_budget() {
    let schema = big_schema();
    let original_len = serde_json::to_vec(&schema).unwrap().len();
    let budget = 200;
    assert!(original_len > budget, "fixture must exceed the budget to be meaningful");

    let compacted = schema.compact(budget);
    let serialized = serde_json::to_vec(&compacted.to_value()).unwrap();
    assert!(
        serialized.len() <= budget,
        "compacted schema ({} bytes) must fit budget ({budget} bytes)",
        serialized.len()
    );
}

#[test]
fn compact_is_idempotent() {
    let schema = big_schema();
    let budget = 200;
    let once = schema.compact(budget);
    let twice = once.compact(budget);
    assert_eq!(once, twice, "compaction must be idempotent for a fixed budget");
}

#[test]
fn compact_noop_when_already_within_budget() {
    let schema = JsonSchema::string(Some("short".into()));
    let compacted = schema.compact(10_000);
    assert_eq!(schema, compacted);
}

#[test]
fn compact_pass1_strips_descriptions_when_that_suffices() {
    let schema = big_schema();
    let full_len = schema.serialized_len();
    let no_desc_len = {
        let mut s = schema.clone();
        s.strip_descriptions();
        s.serialized_len()
    };
    // Choose a budget reachable by stripping descriptions but not by the full
    // un-compacted schema.
    let budget = (full_len + no_desc_len) / 2;
    let compacted = schema.compact(budget);
    assert!(compacted.description.is_none());
    // The defs table should survive when pass 1 already fits.
    assert!(compacted.defs.is_some());
}

// --- declaration tests -----------------------------------------------------

#[test]
fn gemini_declaration_has_expected_shape() {
    let tool = EchoTool::new("echo", ToolExposure::Direct);
    let decl = tool.definition().to_gemini_declaration();

    assert_eq!(decl["name"], json!("echo"));
    assert_eq!(decl["description"], json!("echoes the text argument"));
    assert_eq!(decl["parameters"]["type"], json!("object"));
    assert_eq!(decl["parameters"]["properties"]["text"]["type"], json!("string"));
    assert_eq!(decl["parameters"]["required"], json!(["text"]));
    // No top-level keys beyond the three expected.
    let obj = decl.as_object().unwrap();
    assert_eq!(obj.len(), 3);
}

#[test]
fn openai_function_has_expected_shape() {
    let tool = EchoTool::new("echo", ToolExposure::Direct);
    let func = tool.definition().to_openai_function();
    assert_eq!(func["type"], json!("function"));
    assert_eq!(func["name"], json!("echo"));
    assert_eq!(func["description"], json!("echoes the text argument"));
    assert_eq!(func["parameters"]["type"], json!("object"));
}

// --- registry tests --------------------------------------------------------

#[test]
fn registry_refuses_duplicate_names() {
    let mut registry = ToolRegistry::new();
    registry
        .register(Arc::new(EchoTool::new("echo", ToolExposure::Direct)))
        .unwrap();
    let err = registry
        .register(Arc::new(EchoTool::new("echo", ToolExposure::Direct)))
        .unwrap_err();
    assert!(matches!(err, ToolError::DuplicateName(name) if name == "echo"));
    assert_eq!(registry.len(), 1);
}

#[test]
fn registry_get_finds_registered_tool() {
    let mut registry = ToolRegistry::new();
    registry
        .register(Arc::new(EchoTool::new("echo", ToolExposure::Direct)))
        .unwrap();
    assert!(registry.get("echo").is_some());
    assert!(registry.get("missing").is_none());
}

#[test]
fn registry_filters_by_exposure() {
    let mut registry = ToolRegistry::new();
    registry
        .register(Arc::new(EchoTool::new("direct", ToolExposure::Direct)))
        .unwrap();
    registry
        .register(Arc::new(EchoTool::new("deferred", ToolExposure::Deferred)))
        .unwrap();
    registry
        .register(Arc::new(EchoTool::new("hidden", ToolExposure::Hidden)))
        .unwrap();
    registry
        .register(Arc::new(EchoTool::new("model_only", ToolExposure::DirectModelOnly)))
        .unwrap();

    assert_eq!(registry.definitions().len(), 4);
    assert_eq!(registry.definitions_with_exposure(ToolExposure::Deferred).len(), 1);

    let visible: Vec<&str> = registry
        .model_visible_definitions()
        .iter()
        .map(|d| d.name.as_str())
        .collect();
    assert_eq!(visible, vec!["direct", "model_only"]);

    // gemini_declarations excludes Deferred + Hidden.
    let decls = registry.gemini_declarations();
    assert_eq!(decls.len(), 2);
    let names: Vec<&str> = decls.iter().map(|d| d["name"].as_str().unwrap()).collect();
    assert_eq!(names, vec!["direct", "model_only"]);
}

// --- executor tests (no runtime) -------------------------------------------

#[test]
fn executor_handle_returns_output() {
    let tool = EchoTool::new("echo", ToolExposure::Direct);
    let output = block_on(tool.handle(json!({ "text": "hello" }))).unwrap();
    assert_eq!(output, ToolOutput::ok("hello"));
    assert!(!output.is_error);
}

#[test]
fn executor_handle_reports_invalid_arguments() {
    let tool = EchoTool::new("echo", ToolExposure::Direct);
    let err = block_on(tool.handle(json!({}))).unwrap_err();
    assert!(matches!(err, ToolError::InvalidArguments { ref tool, .. } if tool == "echo"));
}

#[test]
fn executor_dispatch_via_registry() {
    let mut registry = ToolRegistry::new();
    registry
        .register(Arc::new(EchoTool::new("echo", ToolExposure::Direct)))
        .unwrap();
    let tool = registry.get("echo").unwrap();
    let output = block_on(tool.handle(json!({ "text": "via registry" }))).unwrap();
    assert_eq!(output, ToolOutput::ok("via registry"));
}

// --- schema round-trip + output helpers ------------------------------------

#[test]
fn output_helpers_set_error_flag() {
    assert!(!ToolOutput::ok("ok").is_error);
    assert!(ToolOutput::error("bad").is_error);
}

#[test]
fn schema_to_value_round_trips() {
    let mut props = BTreeMap::new();
    props.insert("flag".to_string(), JsonSchema::boolean(None));
    let schema = JsonSchema::object(props, None, Some(AdditionalProperties::Boolean(false)));
    let value = schema.to_value();
    let back: JsonSchema = serde_json::from_value(value).unwrap();
    assert_eq!(schema, back);
    assert_eq!(
        schema.schema_type,
        Some(JsonSchemaType::Single(JsonSchemaPrimitiveType::Object))
    );
}
