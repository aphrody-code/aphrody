// SPDX-License-Identifier: Apache-2.0
//! `aphrody-tools` — vendor-neutral tool descriptor registry.
//!
//! Aphrody integrates with three LLM providers under the whitelist policy
//! (Anthropic, Gemini, Antigravity) and one tool transport (Model Context
//! Protocol). Every one of them ships a slightly different on-wire shape for
//! "the model can call this tool", but the underlying information is the
//! same: a *name*, a *description*, an *input JSON Schema*, and operational
//! metadata (tags, dangerous flag, permission requirement). This crate is the
//! single source of truth so consumers (the router, the MCP bridge, the
//! permission engine, the skills runtime) all agree on what a tool *is*.
//!
//! ## Wire-format conversions
//!
//! | Surface                                                | Method                                      |
//! |--------------------------------------------------------|---------------------------------------------|
//! | Anthropic Messages `tools[]` (`tool_use` blocks)       | [`ToolRegistry::to_anthropic_tool_json`]    |
//! | Gemini `generateContent` `function_declarations[]`     | [`ToolRegistry::to_gemini_function_declaration`] |
//! | Model Context Protocol `tools/list` response           | [`ToolRegistry::to_mcp_tool_json`]          |
//! | Anthropic `tools[]` array → descriptors                | [`ToolRegistry::from_anthropic_tools`]      |
//! | MCP `tools/list` `result.tools[]` → descriptors        | [`ToolRegistry::from_mcp_tools`]            |
//!
//! ## Naming
//!
//! Tool names are validated against the union of the three vendor regexes
//! (cf. memory `project_aphrody_providers_3only`):
//!
//! * Anthropic: `^[a-zA-Z0-9_-]{1,64}$`
//! * Gemini   : `^[a-zA-Z_][a-zA-Z0-9_]{0,63}$`
//! * MCP      : free-form but conventionally snake_case
//!
//! We enforce the strict intersection — a leading ASCII letter, then up to 63
//! `[a-zA-Z0-9_-]` characters (max length 64). See [`is_valid_tool_name`] and
//! [`ToolsError::InvalidName`].
//!
//! ## Validation
//!
//! [`ToolRegistry::validate_args`] is *intentionally* not a full JSON Schema
//! draft 2020-12 validator (that lives in a downstream crate behind a feature
//! flag). It guarantees:
//!
//! 1. The schema declares `"type": "object"`.
//! 2. Every required field listed in `required[]` is present in `args`.
//! 3. If the schema sets `"additionalProperties": false`, no extra fields slip
//!    through.
//! 4. If a top-level property declares `"type": "string"` / `"number"` /
//!    `"integer"` / `"boolean"` / `"array"` / `"object"`, the supplied value
//!    matches.
//!
//! This minimal shape is enough to reject the 95% of malformed tool calls
//! that come from the model hallucinating field names, while staying within
//! a tight, zero-dependency check.

#![forbid(unsafe_code)]

use std::collections::HashMap;
use std::sync::OnceLock;

use regex::Regex;
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value, json};
use thiserror::Error;

// ---------------------------------------------------------------------------
// Builtin tools (read_file, write_file, edit, glob, grep, ls, web_fetch,
// memory_tool). Each module exposes a `descriptor()` returning a
// [`ToolDescriptor`] and a unit struct implementing the [`Tool`] trait.
//
// The builtin modules are gated on non-WASM targets only: filesystem and
// network primitives they rely on (`tokio::fs`, `walkdir`, `ignore`, the
// blocking parts of `reqwest`) cannot link in `wasm32-unknown-unknown`. WASM
// callers that need a tool should construct a custom one against the
// vendor-neutral [`Tool`] trait or wait for the WASI surface to stabilise.
// ---------------------------------------------------------------------------

pub mod error;
pub mod permissions;

#[cfg(not(target_arch = "wasm32"))]
pub mod builtin;

pub use crate::error::ToolError;
pub use crate::permissions::{Permission, PermissionDescriptor};

// ---------------------------------------------------------------------------
// Errors
// ---------------------------------------------------------------------------

/// Failure surface of the tool registry.
#[derive(Debug, Error)]
pub enum ToolsError {
    /// The supplied tool name is absent from the registry.
    #[error("tool not found: {0}")]
    NotFound(String),

    /// The input or output schema attached to a tool is not a valid JSON
    /// Schema fragment (missing `type`, root not an object, etc.).
    #[error("invalid schema for tool {0}")]
    InvalidSchema(String),

    /// The supplied tool name does not satisfy the cross-vendor regex.
    #[error("invalid tool name: {0}")]
    InvalidName(String),

    /// A required field declared by the input schema is missing from the
    /// supplied arguments object.
    #[error("missing required field: {0}")]
    MissingRequiredField(String),

    /// A field's runtime JSON type does not match its declared schema type.
    #[error("type mismatch on field {field}: expected {expected}, got {got}")]
    TypeMismatch {
        /// Property name that mismatched.
        field: String,
        /// Type declared in the schema (`string`, `number`, …).
        expected: String,
        /// Actual JSON type that was supplied.
        got: String,
    },

    /// A property not declared by the schema was supplied while
    /// `additionalProperties: false` was set.
    #[error("unexpected additional property: {0}")]
    AdditionalProperty(String),

    /// JSON (de)serialisation failure.
    #[error("json error: {0}")]
    Json(#[from] serde_json::Error),

    /// Two tools were registered under the same name.
    #[error("duplicate tool: {0}")]
    DuplicateTool(String),
}

/// Result alias used across the crate.
pub type ToolsResult<T> = Result<T, ToolsError>;

// ---------------------------------------------------------------------------
// Tool descriptor
// ---------------------------------------------------------------------------

/// Vendor-neutral descriptor for one callable tool.
///
/// `input_schema` is a JSON Schema draft 2020-12 fragment shaped as `{"type":
/// "object", "properties": {…}, "required": […]}`. The MCP, Anthropic and
/// Gemini shapes all map to this object once normalised.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ToolDescriptor {
    /// Stable identifier. Validated against [`is_valid_tool_name`].
    pub name: String,
    /// Human-readable, model-facing description.
    pub description: String,
    /// JSON Schema for the argument object.
    pub input_schema: Value,
    /// Optional JSON Schema for the response body. `None` means "unspecified".
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub output_schema: Option<Value>,
    /// Free-form tags used by [`ToolRegistry::find_by_tag`].
    #[serde(default)]
    pub tags: Vec<String>,
    /// `true` for tools that mutate state (filesystem writes, network POSTs,
    /// process spawning…). Surfaced through [`ToolRegistry::find_dangerous`].
    #[serde(default)]
    pub dangerous: bool,
    /// `true` when the tool must be gated by `aphrody-permissions` before
    /// the model is allowed to invoke it.
    #[serde(default)]
    pub requires_permission: bool,
}

impl ToolDescriptor {
    /// Build a descriptor in one expression. Validates the name eagerly.
    ///
    /// # Errors
    ///
    /// Returns [`ToolsError::InvalidName`] when the name fails the cross-vendor
    /// regex check.
    pub fn new(
        name: impl Into<String>,
        description: impl Into<String>,
        input_schema: Value,
    ) -> ToolsResult<Self> {
        let name = name.into();
        if !is_valid_tool_name(&name) {
            return Err(ToolsError::InvalidName(name));
        }
        Ok(Self {
            name,
            description: description.into(),
            input_schema,
            output_schema: None,
            tags: Vec::new(),
            dangerous: false,
            requires_permission: false,
        })
    }

    /// Mark as a write-class tool. Chainable.
    #[must_use]
    pub fn dangerous(mut self) -> Self {
        self.dangerous = true;
        self.requires_permission = true;
        self
    }

    /// Tag the descriptor. Chainable.
    #[must_use]
    pub fn with_tag(mut self, tag: impl Into<String>) -> Self {
        self.tags.push(tag.into());
        self
    }

    /// Attach an output schema. Chainable.
    #[must_use]
    pub fn with_output_schema(mut self, schema: Value) -> Self {
        self.output_schema = Some(schema);
        self
    }

    /// Require explicit user permission before invocation. Chainable.
    #[must_use]
    pub fn require_permission(mut self) -> Self {
        self.requires_permission = true;
        self
    }
}

// ---------------------------------------------------------------------------
// Tool trait — runtime execution surface
// ---------------------------------------------------------------------------

/// A runnable tool. Implementations describe themselves via
/// [`Tool::descriptor`] and execute through [`Tool::invoke`].
///
/// The trait is intentionally `async` (via [`async_trait::async_trait`]) to
/// keep parity with the Gemini CLI TypeScript surface where every
/// `BaseToolInvocation.execute` returns a `Promise`. Implementations should
/// remain cancellation-friendly (cooperative `tokio` await points) and never
/// block the runtime worker pool for more than a handful of milliseconds at
/// a time.
#[async_trait::async_trait]
pub trait Tool: Send + Sync {
    /// Returns the vendor-neutral descriptor for this tool. Stable: the
    /// caller may cache the result across many invocations.
    fn descriptor(&self) -> ToolDescriptor;

    /// Returns the permission descriptor for this tool. Defaults to a
    /// permissive "always allow read" entry; mutating tools override this
    /// to return an `Ask` or `Deny` profile.
    fn permission(&self) -> PermissionDescriptor {
        PermissionDescriptor::allow(self.descriptor().name)
    }

    /// Execute the tool against the supplied JSON arguments.
    ///
    /// Implementations should validate inputs against
    /// [`ToolDescriptor::input_schema`] before performing any side effect,
    /// returning [`ToolError::InvalidArgs`] on mismatch.
    ///
    /// # Errors
    ///
    /// Returns a [`ToolError`] describing the failure mode (I/O, network,
    /// validation, etc.). On success returns the JSON payload to surface
    /// back to the model.
    async fn invoke(&self, args: Value) -> Result<Value, ToolError>;
}

// ---------------------------------------------------------------------------
// Registry
// ---------------------------------------------------------------------------

/// Indexed in-memory registry of [`ToolDescriptor`]s.
///
/// Backed by a [`HashMap`] for O(1) lookup by name. Tag and danger queries
/// are O(n) scans which is fine for the small tool counts (≤ ~50) we expect
/// in a single aphrody session.
#[derive(Debug, Clone, Default)]
pub struct ToolRegistry {
    tools: HashMap<String, ToolDescriptor>,
}

impl ToolRegistry {
    /// Empty registry.
    #[must_use]
    pub fn new() -> Self {
        Self {
            tools: HashMap::new(),
        }
    }

    /// Number of registered tools.
    #[must_use]
    pub fn len(&self) -> usize {
        self.tools.len()
    }

    /// True when nothing is registered.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.tools.is_empty()
    }

    /// Insert a tool. Errors when a tool with the same `name` is already
    /// registered (mirrors the precedence rule used by
    /// `aphrody-permissions`: explicit duplicate registration is a config
    /// bug, not a soft override).
    ///
    /// # Errors
    ///
    /// Returns [`ToolsError::DuplicateTool`] if the name is already taken,
    /// [`ToolsError::InvalidName`] if the descriptor's name fails validation,
    /// or [`ToolsError::InvalidSchema`] if the input schema is not an object.
    pub fn register(&mut self, tool: ToolDescriptor) -> ToolsResult<()> {
        if !is_valid_tool_name(&tool.name) {
            return Err(ToolsError::InvalidName(tool.name));
        }
        if !is_object_schema(&tool.input_schema) {
            return Err(ToolsError::InvalidSchema(tool.name));
        }
        if self.tools.contains_key(&tool.name) {
            return Err(ToolsError::DuplicateTool(tool.name));
        }
        tracing::debug!(
            target: "aphrody_tools::registry",
            name = %tool.name,
            tags = ?tool.tags,
            dangerous = tool.dangerous,
            "registered tool",
        );
        self.tools.insert(tool.name.clone(), tool);
        Ok(())
    }

    /// Bulk insert. Stops at the first failure (the registry keeps every
    /// prior insertion — no rollback) and returns the offending error.
    ///
    /// # Errors
    ///
    /// Propagates the first [`ToolsError`] emitted by [`Self::register`].
    pub fn register_many(&mut self, tools: impl IntoIterator<Item = ToolDescriptor>) -> ToolsResult<()> {
        for t in tools {
            self.register(t)?;
        }
        Ok(())
    }

    /// Borrow a descriptor by name.
    #[must_use]
    pub fn get(&self, name: &str) -> Option<&ToolDescriptor> {
        self.tools.get(name)
    }

    /// All registered descriptors, sorted by name for deterministic output.
    #[must_use]
    pub fn list(&self) -> Vec<&ToolDescriptor> {
        let mut v: Vec<&ToolDescriptor> = self.tools.values().collect();
        v.sort_by(|a, b| a.name.cmp(&b.name));
        v
    }

    /// Descriptors carrying `tag` in their [`ToolDescriptor::tags`] list.
    #[must_use]
    pub fn find_by_tag(&self, tag: &str) -> Vec<&ToolDescriptor> {
        let mut v: Vec<&ToolDescriptor> = self
            .tools
            .values()
            .filter(|t| t.tags.iter().any(|x| x == tag))
            .collect();
        v.sort_by(|a, b| a.name.cmp(&b.name));
        v
    }

    /// Descriptors flagged [`ToolDescriptor::dangerous`].
    #[must_use]
    pub fn find_dangerous(&self) -> Vec<&ToolDescriptor> {
        let mut v: Vec<&ToolDescriptor> = self.tools.values().filter(|t| t.dangerous).collect();
        v.sort_by(|a, b| a.name.cmp(&b.name));
        v
    }

    /// Descriptors that require an explicit permission prompt.
    #[must_use]
    pub fn find_permission_required(&self) -> Vec<&ToolDescriptor> {
        let mut v: Vec<&ToolDescriptor> = self
            .tools
            .values()
            .filter(|t| t.requires_permission)
            .collect();
        v.sort_by(|a, b| a.name.cmp(&b.name));
        v
    }

    /// Minimal JSON-Schema-ish validation. See the crate-level docs for the
    /// exact subset honoured.
    ///
    /// # Errors
    ///
    /// Returns one of the validation [`ToolsError`] variants on failure:
    /// [`ToolsError::NotFound`], [`ToolsError::InvalidSchema`],
    /// [`ToolsError::MissingRequiredField`], [`ToolsError::AdditionalProperty`],
    /// or [`ToolsError::TypeMismatch`].
    pub fn validate_args(&self, name: &str, args: &Value) -> ToolsResult<()> {
        let tool = self
            .tools
            .get(name)
            .ok_or_else(|| ToolsError::NotFound(name.to_string()))?;
        let schema = &tool.input_schema;
        if !is_object_schema(schema) {
            return Err(ToolsError::InvalidSchema(name.to_string()));
        }
        let Some(args_obj) = args.as_object() else {
            return Err(ToolsError::TypeMismatch {
                field: "<root>".to_string(),
                expected: "object".to_string(),
                got: json_type_name(args).to_string(),
            });
        };
        // Required fields
        if let Some(required) = schema.get("required").and_then(Value::as_array) {
            for r in required {
                if let Some(field) = r.as_str() {
                    if !args_obj.contains_key(field) {
                        return Err(ToolsError::MissingRequiredField(field.to_string()));
                    }
                }
            }
        }
        // additionalProperties: false
        let additional_allowed = schema
            .get("additionalProperties")
            .and_then(Value::as_bool)
            .unwrap_or(true);
        let properties: Option<&Map<String, Value>> =
            schema.get("properties").and_then(Value::as_object);
        if !additional_allowed {
            if let Some(props) = properties {
                for key in args_obj.keys() {
                    if !props.contains_key(key) {
                        return Err(ToolsError::AdditionalProperty(key.clone()));
                    }
                }
            }
        }
        // Per-property type check (top-level only).
        if let Some(props) = properties {
            for (field, sub_schema) in props {
                let Some(supplied) = args_obj.get(field) else {
                    continue;
                };
                if let Some(expected_ty) =
                    sub_schema.get("type").and_then(Value::as_str)
                {
                    if !json_type_matches(supplied, expected_ty) {
                        return Err(ToolsError::TypeMismatch {
                            field: field.clone(),
                            expected: expected_ty.to_string(),
                            got: json_type_name(supplied).to_string(),
                        });
                    }
                }
            }
        }
        Ok(())
    }

    // -----------------------------------------------------------------------
    // Vendor exporters
    // -----------------------------------------------------------------------

    /// Project a descriptor into the Anthropic Messages `tools[]` shape.
    ///
    /// Output shape:
    /// ```json
    /// {
    ///   "name":         "<name>",
    ///   "description":  "<desc>",
    ///   "input_schema": {…}
    /// }
    /// ```
    ///
    /// # Errors
    ///
    /// Returns [`ToolsError::NotFound`] when the name is absent.
    pub fn to_anthropic_tool_json(&self, name: &str) -> ToolsResult<Value> {
        let tool = self
            .tools
            .get(name)
            .ok_or_else(|| ToolsError::NotFound(name.to_string()))?;
        Ok(json!({
            "name": tool.name,
            "description": tool.description,
            "input_schema": tool.input_schema,
        }))
    }

    /// Project into the Gemini `function_declarations[]` shape.
    ///
    /// Gemini renames `input_schema` → `parameters` and tolerates the same
    /// subset of JSON Schema (object root with `type`, `properties`,
    /// `required`).
    ///
    /// # Errors
    ///
    /// Returns [`ToolsError::NotFound`] when the name is absent.
    pub fn to_gemini_function_declaration(&self, name: &str) -> ToolsResult<Value> {
        let tool = self
            .tools
            .get(name)
            .ok_or_else(|| ToolsError::NotFound(name.to_string()))?;
        Ok(json!({
            "name": tool.name,
            "description": tool.description,
            "parameters": tool.input_schema,
        }))
    }

    /// Project into the Model Context Protocol `tools/list` shape (a single
    /// element of `result.tools[]`).
    ///
    /// Output shape:
    /// ```json
    /// {
    ///   "name":        "<name>",
    ///   "description": "<desc>",
    ///   "inputSchema": {…}
    /// }
    /// ```
    ///
    /// # Errors
    ///
    /// Returns [`ToolsError::NotFound`] when the name is absent.
    pub fn to_mcp_tool_json(&self, name: &str) -> ToolsResult<Value> {
        let tool = self
            .tools
            .get(name)
            .ok_or_else(|| ToolsError::NotFound(name.to_string()))?;
        Ok(json!({
            "name": tool.name,
            "description": tool.description,
            "inputSchema": tool.input_schema,
        }))
    }

    // -----------------------------------------------------------------------
    // Vendor importers (free functions — no `self` needed)
    // -----------------------------------------------------------------------

    /// Parse an Anthropic `tools[]` JSON array into vendor-neutral
    /// descriptors.
    ///
    /// # Errors
    ///
    /// Returns [`ToolsError::InvalidSchema`] when the supplied value is not
    /// an array of objects, or when an entry is missing a required field.
    /// [`ToolsError::InvalidName`] surfaces when a parsed name fails
    /// validation.
    pub fn from_anthropic_tools(value: &Value) -> ToolsResult<Vec<ToolDescriptor>> {
        let arr = value
            .as_array()
            .ok_or_else(|| ToolsError::InvalidSchema("expected an array".to_string()))?;
        arr.iter()
            .map(|entry| {
                let name = entry
                    .get("name")
                    .and_then(Value::as_str)
                    .ok_or_else(|| ToolsError::InvalidSchema("missing name".to_string()))?
                    .to_string();
                let description = entry
                    .get("description")
                    .and_then(Value::as_str)
                    .unwrap_or("")
                    .to_string();
                let input_schema = entry
                    .get("input_schema")
                    .cloned()
                    .ok_or_else(|| ToolsError::InvalidSchema("missing input_schema".to_string()))?;
                ToolDescriptor::new(name, description, input_schema)
            })
            .collect()
    }

    /// Parse a Model Context Protocol `tools/list` response into vendor-
    /// neutral descriptors.
    ///
    /// Accepts either the raw `result` object (`{"tools": […]}`) or the bare
    /// array (`[…]`).
    ///
    /// # Errors
    ///
    /// Same surface as [`Self::from_anthropic_tools`].
    pub fn from_mcp_tools(value: &Value) -> ToolsResult<Vec<ToolDescriptor>> {
        let arr = if let Some(tools) = value.get("tools").and_then(Value::as_array) {
            tools
        } else {
            value
                .as_array()
                .ok_or_else(|| ToolsError::InvalidSchema("expected tools[] array".to_string()))?
        };
        arr.iter()
            .map(|entry| {
                let name = entry
                    .get("name")
                    .and_then(Value::as_str)
                    .ok_or_else(|| ToolsError::InvalidSchema("missing name".to_string()))?
                    .to_string();
                let description = entry
                    .get("description")
                    .and_then(Value::as_str)
                    .unwrap_or("")
                    .to_string();
                let input_schema = entry
                    .get("inputSchema")
                    .cloned()
                    .ok_or_else(|| ToolsError::InvalidSchema("missing inputSchema".to_string()))?;
                ToolDescriptor::new(name, description, input_schema)
            })
            .collect()
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Strict tool-name regex: leading ASCII letter, then `[a-zA-Z0-9_-]{0,63}`.
/// Capped at 64 chars total (Anthropic's hard limit, also fits Gemini's 64
/// chars limit and the MCP convention).
fn name_regex() -> &'static Regex {
    static R: OnceLock<Regex> = OnceLock::new();
    R.get_or_init(|| Regex::new(r"^[a-zA-Z][a-zA-Z0-9_-]{0,63}$").expect("name regex compiles"))
}

/// True when `name` satisfies the cross-vendor naming regex.
#[must_use]
pub fn is_valid_tool_name(name: &str) -> bool {
    name_regex().is_match(name)
}

/// True when `schema` is a JSON Schema fragment whose root is `type: object`.
fn is_object_schema(schema: &Value) -> bool {
    schema
        .get("type")
        .and_then(Value::as_str)
        .is_some_and(|t| t == "object")
}

/// Friendly name for the JSON kind of `v` (for `TypeMismatch` diagnostics).
fn json_type_name(v: &Value) -> &'static str {
    match v {
        Value::Null => "null",
        Value::Bool(_) => "boolean",
        Value::Number(n) => {
            if n.is_i64() || n.is_u64() {
                "integer"
            } else {
                "number"
            }
        }
        Value::String(_) => "string",
        Value::Array(_) => "array",
        Value::Object(_) => "object",
    }
}

/// True when `v`'s JSON kind matches the schema's `expected` type name. The
/// rules mirror JSON Schema draft 2020-12: `"number"` accepts both integers
/// and floats, `"integer"` is strict.
fn json_type_matches(v: &Value, expected: &str) -> bool {
    match expected {
        "string" => v.is_string(),
        "number" => v.is_number(),
        "integer" => v.is_i64() || v.is_u64(),
        "boolean" => v.is_boolean(),
        "array" => v.is_array(),
        "object" => v.is_object(),
        "null" => v.is_null(),
        _ => true, // unknown type token → don't reject (forward-compat)
    }
}

// ---------------------------------------------------------------------------
// Inline unit tests for the private helpers.
// ---------------------------------------------------------------------------

#[cfg(test)]
mod inline {
    use super::*;

    #[test]
    fn name_regex_accepts_kebab_and_snake() {
        assert!(is_valid_tool_name("read"));
        assert!(is_valid_tool_name("read-file"));
        assert!(is_valid_tool_name("read_file"));
        assert!(is_valid_tool_name("Bash"));
        assert!(is_valid_tool_name("X"));
        assert!(is_valid_tool_name(&"a".repeat(64)));
    }

    #[test]
    fn name_regex_rejects_invalid() {
        assert!(!is_valid_tool_name(""));
        assert!(!is_valid_tool_name("1read"));
        assert!(!is_valid_tool_name("-read"));
        assert!(!is_valid_tool_name("read file"));
        assert!(!is_valid_tool_name("read.file"));
        assert!(!is_valid_tool_name(&"a".repeat(65)));
    }

    #[test]
    fn json_type_matches_integer_vs_number() {
        assert!(json_type_matches(&json!(5), "integer"));
        assert!(json_type_matches(&json!(5), "number"));
        assert!(json_type_matches(&json!(5.1), "number"));
        assert!(!json_type_matches(&json!(5.1), "integer"));
        assert!(json_type_matches(&json!("s"), "string"));
        assert!(!json_type_matches(&json!("s"), "integer"));
    }

    #[test]
    fn is_object_schema_works() {
        assert!(is_object_schema(&json!({"type": "object"})));
        assert!(!is_object_schema(&json!({"type": "string"})));
        assert!(!is_object_schema(&json!({})));
    }
}
