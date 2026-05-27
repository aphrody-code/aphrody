// SPDX-License-Identifier: Apache-2.0
//! `aphrody-toolcall` — the tool catalogue model for the aphrody agent engine.
//!
//! This crate is the pure, dependency-light heart of the agent's tool layer:
//!
//! - [`JsonSchema`] — a serde-compatible subset of JSON Schema describing tool
//!   inputs and outputs, with prompt-budget [`JsonSchema::compact`] passes.
//! - [`ToolDefinition`] — a named tool with input/output schemas and an
//!   [`ToolExposure`], plus adapters to the Gemini `function_declarations` and
//!   OpenAI `function` wire shapes.
//! - [`ToolExecutor`] — the async runtime contract a concrete tool implements.
//! - [`ToolRegistry`] — an in-memory catalogue that refuses duplicate names and
//!   projects exposure-filtered declaration lists for a model request.
//!
//! The schema model is re-implemented from the OpenAI Codex `tools` crate
//! (Apache-2.0). The crate is `wasm`-clean: no I/O, no time, no threads beyond
//! the [`Send`] + [`Sync`] bounds required of executors.

use std::collections::BTreeMap;
use std::sync::Arc;

use serde::Deserialize;
use serde::Serialize;
use serde_json::Value as JsonValue;
use serde_json::json;

// ---------------------------------------------------------------------------
// Errors
// ---------------------------------------------------------------------------

/// Errors surfaced by the tool registry and executors.
#[derive(Debug, thiserror::Error)]
pub enum ToolError {
    /// A tool with the same name is already registered.
    #[error("a tool named `{0}` is already registered")]
    DuplicateName(String),

    /// No tool with the requested name exists in the registry.
    #[error("no tool named `{0}` is registered")]
    NotFound(String),

    /// The arguments handed to a tool failed validation/parsing.
    #[error("invalid arguments for tool `{tool}`: {message}")]
    InvalidArguments {
        /// The tool whose arguments were rejected.
        tool: String,
        /// Human-readable reason.
        message: String,
    },

    /// The tool ran but failed during execution.
    #[error("tool `{tool}` failed: {message}")]
    Execution {
        /// The tool that failed.
        tool: String,
        /// Human-readable reason.
        message: String,
    },
}

// ---------------------------------------------------------------------------
// Exposure
// ---------------------------------------------------------------------------

/// Controls where a tool is exposed to the model.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum ToolExposure {
    /// Include this tool in the initial model-visible tool list.
    #[default]
    Direct,

    /// Register the tool for later discovery, but omit it from the initial
    /// model-visible tool list.
    Deferred,

    /// Include this tool in the model-visible list only (never in nested
    /// code-mode tool surfaces).
    DirectModelOnly,

    /// Keep the tool registered for dispatch without exposing it to the model.
    Hidden,
}

impl ToolExposure {
    /// Whether the tool appears in the initial model-visible tool list.
    #[must_use]
    pub fn is_direct(self) -> bool {
        matches!(self, Self::Direct | Self::DirectModelOnly)
    }

    /// Whether the tool should be advertised to the model in a request's
    /// declaration list. `Deferred` and `Hidden` tools are dispatch-only.
    #[must_use]
    pub fn is_model_visible(self) -> bool {
        matches!(self, Self::Direct | Self::DirectModelOnly)
    }
}

// ---------------------------------------------------------------------------
// JSON Schema subset
// ---------------------------------------------------------------------------

/// Primitive JSON Schema type names supported in tool definitions.
///
/// Mirrors the OpenAI Structured Outputs subset for the JSON Schema `type`
/// keyword: string, number, boolean, integer, object, array, and null.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum JsonSchemaPrimitiveType {
    /// `"string"`.
    String,
    /// `"number"`.
    Number,
    /// `"boolean"`.
    Boolean,
    /// `"integer"`.
    Integer,
    /// `"object"`.
    Object,
    /// `"array"`.
    Array,
    /// `"null"`.
    Null,
}

/// JSON Schema `type`: either a single type name or a union of names.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum JsonSchemaType {
    /// A single type, e.g. `"type": "string"`.
    Single(JsonSchemaPrimitiveType),
    /// A union, e.g. `"type": ["string", "null"]`.
    Multiple(Vec<JsonSchemaPrimitiveType>),
}

/// Whether additional object properties are allowed, and any required schema.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum AdditionalProperties {
    /// `additionalProperties: true | false`.
    Boolean(bool),
    /// `additionalProperties: { ... schema ... }`.
    Schema(Box<JsonSchema>),
}

impl From<bool> for AdditionalProperties {
    fn from(value: bool) -> Self {
        Self::Boolean(value)
    }
}

impl From<JsonSchema> for AdditionalProperties {
    fn from(value: JsonSchema) -> Self {
        Self::Schema(Box::new(value))
    }
}

/// A serde-compatible subset of JSON Schema for tool input/output shapes.
///
/// The field set is intentionally small but round-trips against real JSON
/// Schema documents produced by `schemars` and the OpenAI/Gemini tool wire
/// formats. Unknown keywords are ignored on deserialization.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct JsonSchema {
    /// A `$ref` pointer into `$defs`/`definitions`.
    #[serde(rename = "$ref", skip_serializing_if = "Option::is_none")]
    pub schema_ref: Option<String>,

    /// The `type` keyword (single or union).
    #[serde(rename = "type", skip_serializing_if = "Option::is_none")]
    pub schema_type: Option<JsonSchemaType>,

    /// Human-readable description (first to be dropped under budget pressure).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,

    /// Object properties keyed by name.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub properties: Option<BTreeMap<String, JsonSchema>>,

    /// Required property names for object schemas.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub required: Option<Vec<String>>,

    /// Element schema for array types.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub items: Option<Box<JsonSchema>>,

    /// The `enum` keyword.
    #[serde(rename = "enum", skip_serializing_if = "Option::is_none")]
    pub enum_values: Option<Vec<JsonValue>>,

    /// The `additionalProperties` keyword.
    #[serde(
        rename = "additionalProperties",
        skip_serializing_if = "Option::is_none"
    )]
    pub additional_properties: Option<AdditionalProperties>,

    /// The `anyOf` keyword.
    #[serde(rename = "anyOf", skip_serializing_if = "Option::is_none")]
    pub any_of: Option<Vec<JsonSchema>>,

    /// The `$defs` definition table (2020-12 dialect).
    #[serde(rename = "$defs", skip_serializing_if = "Option::is_none")]
    pub defs: Option<BTreeMap<String, JsonSchema>>,

    /// The legacy `definitions` table (draft-07 dialect).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub definitions: Option<BTreeMap<String, JsonSchema>>,
}

impl JsonSchema {
    /// Construct a schema with a single primitive type and optional description.
    fn typed(schema_type: JsonSchemaPrimitiveType, description: Option<String>) -> Self {
        Self {
            schema_type: Some(JsonSchemaType::Single(schema_type)),
            description,
            ..Default::default()
        }
    }

    /// A `string` schema.
    #[must_use]
    pub fn string(description: Option<String>) -> Self {
        Self::typed(JsonSchemaPrimitiveType::String, description)
    }

    /// A `number` schema.
    #[must_use]
    pub fn number(description: Option<String>) -> Self {
        Self::typed(JsonSchemaPrimitiveType::Number, description)
    }

    /// An `integer` schema.
    #[must_use]
    pub fn integer(description: Option<String>) -> Self {
        Self::typed(JsonSchemaPrimitiveType::Integer, description)
    }

    /// A `boolean` schema.
    #[must_use]
    pub fn boolean(description: Option<String>) -> Self {
        Self::typed(JsonSchemaPrimitiveType::Boolean, description)
    }

    /// A `null` schema.
    #[must_use]
    pub fn null(description: Option<String>) -> Self {
        Self::typed(JsonSchemaPrimitiveType::Null, description)
    }

    /// A `string` schema constrained to an enumeration of values.
    #[must_use]
    pub fn string_enum(values: Vec<JsonValue>, description: Option<String>) -> Self {
        Self {
            schema_type: Some(JsonSchemaType::Single(JsonSchemaPrimitiveType::String)),
            description,
            enum_values: Some(values),
            ..Default::default()
        }
    }

    /// An `array` schema with the given element schema.
    #[must_use]
    pub fn array(items: JsonSchema, description: Option<String>) -> Self {
        Self {
            schema_type: Some(JsonSchemaType::Single(JsonSchemaPrimitiveType::Array)),
            description,
            items: Some(Box::new(items)),
            ..Default::default()
        }
    }

    /// An `object` schema with properties, optional required list, and optional
    /// `additionalProperties`.
    #[must_use]
    pub fn object(
        properties: BTreeMap<String, JsonSchema>,
        required: Option<Vec<String>>,
        additional_properties: Option<AdditionalProperties>,
    ) -> Self {
        Self {
            schema_type: Some(JsonSchemaType::Single(JsonSchemaPrimitiveType::Object)),
            properties: Some(properties),
            required,
            additional_properties,
            ..Default::default()
        }
    }

    /// Serialize the schema to a [`serde_json::Value`].
    ///
    /// The schema model only contains serializable data, so serialization
    /// cannot fail; a malformed value would indicate a bug and yields `null`.
    #[must_use]
    pub fn to_value(&self) -> JsonValue {
        serde_json::to_value(self).unwrap_or(JsonValue::Null)
    }

    /// Byte length of the compact JSON serialization of this schema.
    #[must_use]
    fn serialized_len(&self) -> usize {
        serde_json::to_vec(self).map_or(usize::MAX, |bytes| bytes.len())
    }

    /// Return a copy of this schema shrunk to fit within `budget_bytes`.
    ///
    /// Applies up to three increasingly lossy passes, stopping as soon as the
    /// serialized form fits the budget:
    ///
    /// 1. drop every `description`,
    /// 2. drop every `$defs` / `definitions` table,
    /// 3. collapse object schemas deeper than two levels into
    ///    `{"type": "object"}`.
    ///
    /// The operation is idempotent: compacting an already-compacted schema with
    /// the same budget returns an equal schema. If even the fully collapsed
    /// schema exceeds the budget, the most-compacted form is returned.
    #[must_use]
    pub fn compact(&self, budget_bytes: usize) -> JsonSchema {
        let mut current = self.clone();
        if current.serialized_len() <= budget_bytes {
            return current;
        }

        current.strip_descriptions();
        if current.serialized_len() <= budget_bytes {
            return current;
        }

        current.drop_definitions();
        if current.serialized_len() <= budget_bytes {
            return current;
        }

        current.collapse_deep_objects(0);
        current
    }

    /// Pass 1: recursively remove every `description`.
    fn strip_descriptions(&mut self) {
        self.description = None;
        if let Some(items) = self.items.as_mut() {
            items.strip_descriptions();
        }
        if let Some(properties) = self.properties.as_mut() {
            for value in properties.values_mut() {
                value.strip_descriptions();
            }
        }
        if let Some(any_of) = self.any_of.as_mut() {
            for value in any_of {
                value.strip_descriptions();
            }
        }
        if let Some(AdditionalProperties::Schema(schema)) = self.additional_properties.as_mut() {
            schema.strip_descriptions();
        }
        if let Some(defs) = self.defs.as_mut() {
            for value in defs.values_mut() {
                value.strip_descriptions();
            }
        }
        if let Some(definitions) = self.definitions.as_mut() {
            for value in definitions.values_mut() {
                value.strip_descriptions();
            }
        }
    }

    /// Pass 2: drop the local `$defs` / `definitions` tables. References to the
    /// dropped definitions are rewritten to permissive empty schemas so the
    /// resulting document stays self-consistent.
    fn drop_definitions(&mut self) {
        self.rewrite_local_refs();
        self.defs = None;
        self.definitions = None;
    }

    /// Replace any `$ref` into a local definition table with an empty schema.
    fn rewrite_local_refs(&mut self) {
        if self
            .schema_ref
            .as_deref()
            .is_some_and(is_local_definition_ref)
        {
            *self = JsonSchema::default();
            return;
        }
        if let Some(items) = self.items.as_mut() {
            items.rewrite_local_refs();
        }
        if let Some(properties) = self.properties.as_mut() {
            for value in properties.values_mut() {
                value.rewrite_local_refs();
            }
        }
        if let Some(any_of) = self.any_of.as_mut() {
            for value in any_of {
                value.rewrite_local_refs();
            }
        }
        if let Some(AdditionalProperties::Schema(schema)) = self.additional_properties.as_mut() {
            schema.rewrite_local_refs();
        }
        // Note: definitions themselves are about to be dropped, so their inner
        // refs do not need rewriting.
    }

    /// Pass 3: collapse any object schema nested deeper than two levels into a
    /// bare `{"type": "object"}`.
    fn collapse_deep_objects(&mut self, depth: usize) {
        if depth >= COMPACT_MAX_DEPTH && self.is_complex_object() {
            *self = JsonSchema {
                schema_type: Some(JsonSchemaType::Single(JsonSchemaPrimitiveType::Object)),
                ..Default::default()
            };
            return;
        }
        if let Some(items) = self.items.as_mut() {
            items.collapse_deep_objects(depth + 1);
        }
        if let Some(properties) = self.properties.as_mut() {
            for value in properties.values_mut() {
                value.collapse_deep_objects(depth + 1);
            }
        }
        if let Some(any_of) = self.any_of.as_mut() {
            for value in any_of {
                value.collapse_deep_objects(depth + 1);
            }
        }
        if let Some(AdditionalProperties::Schema(schema)) = self.additional_properties.as_mut() {
            schema.collapse_deep_objects(depth + 1);
        }
    }

    /// Whether this schema carries nested structure worth collapsing.
    fn is_complex_object(&self) -> bool {
        self.properties
            .as_ref()
            .is_some_and(|properties| !properties.is_empty())
            || self.items.is_some()
            || self.any_of.is_some()
            || matches!(
                self.additional_properties,
                Some(AdditionalProperties::Schema(_))
            )
    }
}

/// Maximum object nesting depth retained by the third compaction pass.
const COMPACT_MAX_DEPTH: usize = 2;

/// Whether a `$ref` points into a local `#/$defs/...` or `#/definitions/...`
/// table.
fn is_local_definition_ref(schema_ref: &str) -> bool {
    schema_ref.starts_with("#/$defs/") || schema_ref.starts_with("#/definitions/")
}

// ---------------------------------------------------------------------------
// Tool definition
// ---------------------------------------------------------------------------

/// A named tool, its description, input/output schemas, and model exposure.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ToolDefinition {
    /// The tool's unique name (the function name the model calls).
    pub name: String,
    /// Human-readable description shown to the model.
    pub description: String,
    /// Schema describing the tool's arguments.
    pub input_schema: JsonSchema,
    /// Optional schema describing the tool's structured output.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub output_schema: Option<JsonSchema>,
    /// Where the tool is exposed to the model.
    #[serde(default)]
    pub exposure: ToolExposure,
}

impl ToolDefinition {
    /// Create a `Direct`-exposure tool definition with no output schema.
    #[must_use]
    pub fn new(
        name: impl Into<String>,
        description: impl Into<String>,
        input_schema: JsonSchema,
    ) -> Self {
        Self {
            name: name.into(),
            description: description.into(),
            input_schema,
            output_schema: None,
            exposure: ToolExposure::Direct,
        }
    }

    /// Set this definition's exposure (builder style).
    #[must_use]
    pub fn with_exposure(mut self, exposure: ToolExposure) -> Self {
        self.exposure = exposure;
        self
    }

    /// Set this definition's output schema (builder style).
    #[must_use]
    pub fn with_output_schema(mut self, output_schema: JsonSchema) -> Self {
        self.output_schema = Some(output_schema);
        self
    }

    /// The Gemini `function_declarations[]` shape:
    /// `{ name, description, parameters }`, where `parameters` is the input
    /// schema compacted to fit a conservative per-tool budget.
    #[must_use]
    pub fn to_gemini_declaration(&self) -> JsonValue {
        json!({
            "name": self.name,
            "description": self.description,
            "parameters": self.input_schema.compact(GEMINI_SCHEMA_BUDGET_BYTES).to_value(),
        })
    }

    /// The OpenAI `tools[]` function shape:
    /// `{ type: "function", name, description, parameters }`.
    #[must_use]
    pub fn to_openai_function(&self) -> JsonValue {
        json!({
            "type": "function",
            "name": self.name,
            "description": self.description,
            "parameters": self.input_schema.compact(OPENAI_SCHEMA_BUDGET_BYTES).to_value(),
        })
    }
}

/// Conservative per-tool schema budget for Gemini declarations (bytes).
const GEMINI_SCHEMA_BUDGET_BYTES: usize = 8_000;
/// Conservative per-tool schema budget for OpenAI functions (bytes).
const OPENAI_SCHEMA_BUDGET_BYTES: usize = 8_000;

// ---------------------------------------------------------------------------
// Tool output + executor
// ---------------------------------------------------------------------------

/// The result a tool returns to the engine.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ToolOutput {
    /// The textual content (often JSON or markdown) the tool produced.
    pub content: String,
    /// Whether this output represents an error the model should see.
    pub is_error: bool,
}

impl ToolOutput {
    /// A successful output.
    #[must_use]
    pub fn ok(content: impl Into<String>) -> Self {
        Self {
            content: content.into(),
            is_error: false,
        }
    }

    /// An error output (still returned to the model as a tool result).
    #[must_use]
    pub fn error(content: impl Into<String>) -> Self {
        Self {
            content: content.into(),
            is_error: true,
        }
    }
}

/// The async runtime contract a concrete tool implements.
///
/// Implementations tie the model-visible [`ToolDefinition`] to executable
/// behavior. The bounds `Send + Sync` let registries hold `Arc<dyn ToolExecutor>`
/// and dispatch across async tasks.
#[async_trait::async_trait]
pub trait ToolExecutor: Send + Sync {
    /// The definition advertised to the model and used for routing.
    fn definition(&self) -> &ToolDefinition;

    /// Execute the tool against decoded JSON `arguments`.
    async fn handle(&self, arguments: JsonValue) -> Result<ToolOutput, ToolError>;
}

// ---------------------------------------------------------------------------
// Registry
// ---------------------------------------------------------------------------

/// An in-memory catalogue of tools keyed by name.
#[derive(Clone, Default)]
pub struct ToolRegistry {
    tools: BTreeMap<String, Arc<dyn ToolExecutor>>,
}

impl ToolRegistry {
    /// Create an empty registry.
    #[must_use]
    pub fn new() -> Self {
        Self {
            tools: BTreeMap::new(),
        }
    }

    /// Register a tool, rejecting a name already present.
    ///
    /// # Errors
    /// Returns [`ToolError::DuplicateName`] if a tool with the same name is
    /// already registered.
    pub fn register(&mut self, tool: Arc<dyn ToolExecutor>) -> Result<(), ToolError> {
        let name = tool.definition().name.clone();
        if self.tools.contains_key(&name) {
            return Err(ToolError::DuplicateName(name));
        }
        self.tools.insert(name, tool);
        Ok(())
    }

    /// Look up a registered tool by name.
    #[must_use]
    pub fn get(&self, name: &str) -> Option<Arc<dyn ToolExecutor>> {
        self.tools.get(name).cloned()
    }

    /// Number of registered tools.
    #[must_use]
    pub fn len(&self) -> usize {
        self.tools.len()
    }

    /// Whether the registry holds no tools.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.tools.is_empty()
    }

    /// Every registered tool definition, in name order.
    #[must_use]
    pub fn definitions(&self) -> Vec<&ToolDefinition> {
        self.tools
            .values()
            .map(|tool| tool.definition())
            .collect()
    }

    /// Tool definitions whose exposure equals `exposure`.
    #[must_use]
    pub fn definitions_with_exposure(&self, exposure: ToolExposure) -> Vec<&ToolDefinition> {
        self.tools
            .values()
            .map(|tool| tool.definition())
            .filter(|definition| definition.exposure == exposure)
            .collect()
    }

    /// Tool definitions visible to the model (excludes `Deferred` / `Hidden`).
    #[must_use]
    pub fn model_visible_definitions(&self) -> Vec<&ToolDefinition> {
        self.tools
            .values()
            .map(|tool| tool.definition())
            .filter(|definition| definition.exposure.is_model_visible())
            .collect()
    }

    /// Gemini `function_declarations` for model-visible tools only
    /// (excludes `Deferred` / `Hidden`).
    #[must_use]
    pub fn gemini_declarations(&self) -> Vec<JsonValue> {
        self.tools
            .values()
            .map(|tool| tool.definition())
            .filter(|definition| definition.exposure.is_model_visible())
            .map(ToolDefinition::to_gemini_declaration)
            .collect()
    }

    /// OpenAI `tools[]` function entries for model-visible tools only.
    #[must_use]
    pub fn openai_functions(&self) -> Vec<JsonValue> {
        self.tools
            .values()
            .map(|tool| tool.definition())
            .filter(|definition| definition.exposure.is_model_visible())
            .map(ToolDefinition::to_openai_function)
            .collect()
    }
}

impl std::fmt::Debug for ToolRegistry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ToolRegistry")
            .field("tools", &self.tools.keys().collect::<Vec<_>>())
            .finish()
    }
}

#[cfg(test)]
#[path = "lib_tests.rs"]
mod tests;
