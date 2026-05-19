// SPDX-License-Identifier: Apache-2.0
//! aphrody-settings — hierarchical JSON settings loader.
//!
//! Mirrors the conventions used by aphrody-permissions / aphrody-hooks /
//! aphrody-terminal-config: four ordered layers compose into one merged JSON
//! document and a single [`SettingsLoader::load`] call drives the on-disk
//! reads.
//!
//! ## Layer precedence
//!
//! Higher numbers win on conflict. Within a layer, objects are merged
//! *deeply* and arrays are *replaced* (never concatenated), which matches the
//! merge contract of [`serde_json::Value`] manipulation used elsewhere in the
//! workspace (cf. `crates/aphrody-permissions/src/lib.rs` `merge` and
//! `crates/aphrody-terminal-config/src/merge.rs`).
//!
//! ```text
//!  4. Env       (process environment, `APHRODY_FOO__BAR=baz` → {"foo":{"bar":"baz"}})
//!  3. Local     (`./.aphrody/settings.local.json` — git-ignored)
//!  2. Project   (`./.aphrody/settings.json`       — checked in)
//!  1. User      (`$HOME/.aphrody/settings.json`)
//! ```
//!
//! ## Path access
//!
//! [`LayeredSettings::get`] accepts a dotted path (`"llm.model"`) and walks
//! the merged JSON. Numeric segments index into arrays (`"servers.0.host"`).
//!
//! ## Schema validation
//!
//! [`validate_against_schema`] implements a deliberately small subset of
//! JSON Schema draft-07 (type, required, additionalProperties=false,
//! properties). For the full surface, plug in a `jsonschema` crate over the
//! merged [`serde_json::Value`] returned by [`LayeredSettings::merged`].

#![forbid(unsafe_code)]

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use thiserror::Error;

// ---------------------------------------------------------------------------
// Layer identifier
// ---------------------------------------------------------------------------

/// Ordered settings layer. Lower discriminants are overridden by higher ones
/// during [`LayeredSettings::merged`]. The numeric order is part of the public
/// contract — do not reorder variants.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SettingsLayer {
    /// `$HOME/.aphrody/settings.json` — lowest precedence.
    User = 0,
    /// `./.aphrody/settings.json` — project default checked into the repo.
    Project = 1,
    /// `./.aphrody/settings.local.json` — git-ignored per-machine overrides.
    Local = 2,
    /// Process environment (`APHRODY_FOO__BAR=baz`). Highest precedence.
    Env = 3,
}

impl SettingsLayer {
    /// Stable lowercase identifier used in CLI / JSON surfaces.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::User => "user",
            Self::Project => "project",
            Self::Local => "local",
            Self::Env => "env",
        }
    }

    /// Iterate from lowest to highest precedence.
    #[must_use]
    pub fn ordered() -> [Self; 4] {
        [Self::User, Self::Project, Self::Local, Self::Env]
    }
}

// ---------------------------------------------------------------------------
// Errors
// ---------------------------------------------------------------------------

/// Failure modes surfaced by the loader and merger.
#[derive(Debug, Error)]
pub enum SettingsError {
    /// Filesystem I/O failure (open / read).
    #[error("io error on {path}: {source}")]
    Io {
        /// Offending path.
        path: PathBuf,
        /// Underlying I/O error.
        #[source]
        source: std::io::Error,
    },
    /// JSON (de)serialisation failure.
    #[error("json error in {path}: {source}")]
    Json {
        /// Offending path (or `"<env>"` / `"<inline>"`).
        path: String,
        /// Underlying serde_json error.
        #[source]
        source: serde_json::Error,
    },
    /// A user-supplied path could not be located.
    #[error("path not found: {0}")]
    PathNotFound(String),
    /// Layer rejected because it was not a JSON object at the root.
    #[error("invalid layer {0}: root must be a JSON object")]
    InvalidLayer(String),
    /// Two layers carry incompatible JSON shapes at the same path
    /// (e.g. user has `{"a": {"b": 1}}` and env has `{"a": 5}`).
    #[error("merge conflict at `{path}`: {reason}")]
    MergeConflict {
        /// Dotted path where the conflict surfaced.
        path: String,
        /// Human-readable explanation.
        reason: String,
    },
    /// Schema validation rejected the supplied value.
    #[error("schema violation at `{path}`: {message}")]
    SchemaViolation {
        /// Dotted path of the offending value.
        path: String,
        /// Human-readable explanation.
        message: String,
    },
}

/// Convenience alias.
pub type Result<T> = std::result::Result<T, SettingsError>;

// ---------------------------------------------------------------------------
// LayeredSettings — the merged document.
// ---------------------------------------------------------------------------

/// Holds one [`serde_json::Value`] per [`SettingsLayer`]. Each layer is
/// always a JSON object (enforced at insertion time via [`Self::set_layer`]).
#[derive(Debug, Clone, Default)]
pub struct LayeredSettings {
    layers: BTreeMap<SettingsLayer, Value>,
}

impl LayeredSettings {
    /// Empty settings, no layer populated.
    #[must_use]
    pub fn new() -> Self {
        Self { layers: BTreeMap::new() }
    }

    /// Builder: set the user layer.
    ///
    /// # Errors
    ///
    /// Returns [`SettingsError::InvalidLayer`] if `value` is not an object.
    pub fn with_user(mut self, value: Value) -> Result<Self> {
        self.set_layer(SettingsLayer::User, value)?;
        Ok(self)
    }

    /// Builder: set the project layer.
    ///
    /// # Errors
    ///
    /// Returns [`SettingsError::InvalidLayer`] if `value` is not an object.
    pub fn with_project(mut self, value: Value) -> Result<Self> {
        self.set_layer(SettingsLayer::Project, value)?;
        Ok(self)
    }

    /// Builder: set the local layer.
    ///
    /// # Errors
    ///
    /// Returns [`SettingsError::InvalidLayer`] if `value` is not an object.
    pub fn with_local(mut self, value: Value) -> Result<Self> {
        self.set_layer(SettingsLayer::Local, value)?;
        Ok(self)
    }

    /// Builder: set the env layer.
    ///
    /// # Errors
    ///
    /// Returns [`SettingsError::InvalidLayer`] if `value` is not an object.
    pub fn with_env(mut self, value: Value) -> Result<Self> {
        self.set_layer(SettingsLayer::Env, value)?;
        Ok(self)
    }

    /// Replace one layer entirely. The root of `value` must be a JSON object.
    ///
    /// # Errors
    ///
    /// Returns [`SettingsError::InvalidLayer`] if `value` is not an object.
    pub fn set_layer(&mut self, layer: SettingsLayer, value: Value) -> Result<()> {
        if !value.is_object() {
            return Err(SettingsError::InvalidLayer(layer.as_str().to_string()));
        }
        self.layers.insert(layer, value);
        Ok(())
    }

    /// Drop a layer (becomes equivalent to "never set").
    pub fn clear_layer(&mut self, layer: SettingsLayer) {
        self.layers.remove(&layer);
    }

    /// Borrow the raw value for a specific layer (without merging).
    #[must_use]
    pub fn layer(&self, layer: SettingsLayer) -> Option<&Value> {
        self.layers.get(&layer)
    }

    /// Layers currently populated (in ascending precedence order).
    pub fn layers(&self) -> impl Iterator<Item = (SettingsLayer, &Value)> {
        // BTreeMap iterates in key order; SettingsLayer's discriminants encode
        // precedence, so this is already user → project → local → env.
        self.layers.iter().map(|(k, v)| (*k, v))
    }

    /// Compute the deep-merged document.
    ///
    /// Merge contract:
    ///   * Objects merge recursively (later wins on overlapping keys).
    ///   * Arrays are *replaced* whole (never concatenated).
    ///   * Scalars take the highest-precedence non-null value.
    #[must_use]
    pub fn merged(&self) -> Value {
        let mut acc = Value::Object(Map::new());
        // BTreeMap iteration order is by key, which is precedence-ordered.
        for (_layer, value) in &self.layers {
            deep_merge(&mut acc, value.clone());
        }
        acc
    }

    /// Walk the merged document along a dotted path (`"a.b.0.c"`).
    /// Returns the borrowed value if every segment resolves.
    ///
    /// Numeric segments index into arrays; non-numeric segments index into
    /// objects. A leading `.` or trailing `.` is rejected (empty segment).
    #[must_use]
    pub fn get(&self, path: &str) -> Option<Value> {
        let merged = self.merged();
        navigate(&merged, path).cloned()
    }
}

// ---------------------------------------------------------------------------
// Deep-merge helper.
// ---------------------------------------------------------------------------

/// In-place deep-merge `src` into `dst`. `src` wins on every leaf.
///
/// Contract:
///   * Two objects → recursive key-by-key merge.
///   * Two arrays → `src` replaces `dst` whole.
///   * Mismatched types or any non-object pair → `src` replaces `dst`.
///   * `src == Null` → kept as null (caller can pre-filter to skip nulls).
fn deep_merge(dst: &mut Value, src: Value) {
    match (dst, src) {
        (Value::Object(dst_map), Value::Object(src_map)) => {
            for (k, v) in src_map {
                match dst_map.get_mut(&k) {
                    Some(existing) => deep_merge(existing, v),
                    None => {
                        dst_map.insert(k, v);
                    }
                }
            }
        }
        (slot, other) => {
            *slot = other;
        }
    }
}

/// Walk a dotted path through a JSON value. Numeric segments index arrays.
fn navigate<'a>(root: &'a Value, path: &str) -> Option<&'a Value> {
    if path.is_empty() {
        return Some(root);
    }
    let mut cursor = root;
    for segment in path.split('.') {
        if segment.is_empty() {
            return None;
        }
        cursor = match cursor {
            Value::Object(map) => map.get(segment)?,
            Value::Array(items) => {
                let idx: usize = segment.parse().ok()?;
                items.get(idx)?
            }
            _ => return None,
        };
    }
    Some(cursor)
}

// ---------------------------------------------------------------------------
// Env loader
// ---------------------------------------------------------------------------

/// Parse environment variables matching `prefix` into a nested JSON object.
///
/// Key contract:
///   * Strip `prefix` from each `KEY=VALUE` pair.
///   * Split the remainder on `__` (double underscore) → nested object path.
///   * Lower-case every segment for ergonomic merging with checked-in
///     camelCase / snake_case JSON.
///   * Values are parsed as JSON if they look like one (`true`, `false`,
///     a number, a quoted string, an array, an object). Otherwise they are
///     kept as raw strings.
///
/// Example: `APHRODY_LLM__MODEL=gpt-4o` with prefix `APHRODY_` →
/// `{"llm": {"model": "gpt-4o"}}`.
#[must_use]
pub fn env_to_json<I>(vars: I, prefix: &str) -> Value
where
    I: IntoIterator<Item = (String, String)>,
{
    let mut root = Map::new();
    for (key, raw_value) in vars {
        let Some(stripped) = key.strip_prefix(prefix) else {
            continue;
        };
        if stripped.is_empty() {
            continue;
        }
        let segments: Vec<String> = stripped
            .split("__")
            .filter(|s| !s.is_empty())
            .map(|s| s.to_ascii_lowercase())
            .collect();
        if segments.is_empty() {
            continue;
        }
        let parsed_value: Value = serde_json::from_str(&raw_value)
            .unwrap_or(Value::String(raw_value));
        insert_nested(&mut root, &segments, parsed_value);
    }
    Value::Object(root)
}

/// Recursive helper for [`env_to_json`].
fn insert_nested(target: &mut Map<String, Value>, path: &[String], value: Value) {
    let (head, tail) = match path.split_first() {
        Some(pair) => pair,
        None => return,
    };
    if tail.is_empty() {
        target.insert(head.clone(), value);
        return;
    }
    let entry = target.entry(head.clone()).or_insert_with(|| Value::Object(Map::new()));
    if !entry.is_object() {
        // Existing scalar at this path; promote to object so we can nest.
        *entry = Value::Object(Map::new());
    }
    if let Value::Object(child) = entry {
        insert_nested(child, tail, value);
    }
}

// ---------------------------------------------------------------------------
// Loader
// ---------------------------------------------------------------------------

/// Configurable, async settings loader.
///
/// Build one with [`Self::new`], chain `with_*_path` / `with_env_prefix`,
/// then call [`Self::load`]. Missing files are silently skipped — pre-flight
/// existence checks aren't required.
#[derive(Debug, Clone, Default)]
pub struct SettingsLoader {
    user_path: Option<PathBuf>,
    project_path: Option<PathBuf>,
    local_path: Option<PathBuf>,
    env_prefix: Option<String>,
}

impl SettingsLoader {
    /// Empty loader; nothing will be read until paths / env prefix are set.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the user-layer path (typically `$HOME/.aphrody/settings.json`).
    #[must_use]
    pub fn with_user_path<P: Into<PathBuf>>(mut self, path: P) -> Self {
        self.user_path = Some(path.into());
        self
    }

    /// Set the project-layer path (typically `./.aphrody/settings.json`).
    #[must_use]
    pub fn with_project_path<P: Into<PathBuf>>(mut self, path: P) -> Self {
        self.project_path = Some(path.into());
        self
    }

    /// Set the local-layer path (typically `./.aphrody/settings.local.json`).
    #[must_use]
    pub fn with_local_path<P: Into<PathBuf>>(mut self, path: P) -> Self {
        self.local_path = Some(path.into());
        self
    }

    /// Set the environment-variable prefix (typically `"APHRODY_"`).
    #[must_use]
    pub fn with_env_prefix<S: Into<String>>(mut self, prefix: S) -> Self {
        self.env_prefix = Some(prefix.into());
        self
    }

    /// Read every configured layer and return the assembled [`LayeredSettings`].
    ///
    /// Missing files are skipped silently. Parse failures and non-object roots
    /// surface as [`SettingsError`].
    ///
    /// # Errors
    ///
    /// Returns any [`SettingsError`] surfaced by the disk reads, JSON parse,
    /// or root-must-be-object validation.
    pub async fn load(&self) -> Result<LayeredSettings> {
        let mut settings = LayeredSettings::new();
        if let Some(path) = &self.user_path {
            if let Some(value) = read_json_if_exists(path).await? {
                settings.set_layer(SettingsLayer::User, value)?;
            }
        }
        if let Some(path) = &self.project_path {
            if let Some(value) = read_json_if_exists(path).await? {
                settings.set_layer(SettingsLayer::Project, value)?;
            }
        }
        if let Some(path) = &self.local_path {
            if let Some(value) = read_json_if_exists(path).await? {
                settings.set_layer(SettingsLayer::Local, value)?;
            }
        }
        if let Some(prefix) = &self.env_prefix {
            let env_value = env_to_json(std::env::vars(), prefix);
            if let Value::Object(map) = &env_value {
                if !map.is_empty() {
                    settings.set_layer(SettingsLayer::Env, env_value)?;
                }
            }
        }
        Ok(settings)
    }
}

async fn read_json_if_exists(path: &Path) -> Result<Option<Value>> {
    let bytes = match tokio::fs::read(path).await {
        Ok(bytes) => bytes,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(err) => {
            return Err(SettingsError::Io { path: path.to_path_buf(), source: err });
        }
    };
    let value: Value = serde_json::from_slice(&bytes).map_err(|err| SettingsError::Json {
        path: path.display().to_string(),
        source: err,
    })?;
    Ok(Some(value))
}

// ---------------------------------------------------------------------------
// Minimal JSON Schema validator
// ---------------------------------------------------------------------------

/// Validate `settings` against a *minimum-viable* JSON Schema document.
///
/// Supported keywords (deliberately tiny):
///   * `type` — `"object"`, `"array"`, `"string"`, `"number"`, `"integer"`,
///     `"boolean"`, `"null"`. Accepts either a string or an array of strings.
///   * `required` — array of strings (object only).
///   * `additionalProperties` — `false` rejects unknown keys (object only).
///   * `properties` — map of `{key → sub-schema}`, recursively validated.
///   * `items` — sub-schema applied to every array element.
///
/// Anything else in `schema` is ignored. For comprehensive draft-07 / 2020-12
/// validation, plug in the `jsonschema` crate on top of [`LayeredSettings::merged`].
///
/// # Errors
///
/// Returns [`SettingsError::SchemaViolation`] on the first violation.
pub fn validate_against_schema(settings: &Value, schema: &Value) -> Result<()> {
    validate_at("", settings, schema)
}

fn validate_at(path: &str, value: &Value, schema: &Value) -> Result<()> {
    // type check
    if let Some(type_field) = schema.get("type") {
        check_type(path, value, type_field)?;
    }
    // object-specific
    if value.is_object() {
        let obj = value.as_object().expect("checked");
        if let Some(required) = schema.get("required").and_then(|v| v.as_array()) {
            for req in required {
                let Some(key) = req.as_str() else {
                    continue;
                };
                if !obj.contains_key(key) {
                    return Err(SettingsError::SchemaViolation {
                        path: join_path(path, key),
                        message: format!("required property `{key}` missing"),
                    });
                }
            }
        }
        if schema.get("additionalProperties") == Some(&Value::Bool(false)) {
            if let Some(props) = schema.get("properties").and_then(|v| v.as_object()) {
                for key in obj.keys() {
                    if !props.contains_key(key) {
                        return Err(SettingsError::SchemaViolation {
                            path: join_path(path, key),
                            message: format!("additional property `{key}` not allowed"),
                        });
                    }
                }
            }
        }
        if let Some(props) = schema.get("properties").and_then(|v| v.as_object()) {
            for (key, sub_schema) in props {
                if let Some(child) = obj.get(key) {
                    validate_at(&join_path(path, key), child, sub_schema)?;
                }
            }
        }
    }
    // array-specific
    if value.is_array() {
        if let Some(items_schema) = schema.get("items") {
            for (idx, item) in value.as_array().expect("checked").iter().enumerate() {
                validate_at(&join_path(path, &idx.to_string()), item, items_schema)?;
            }
        }
    }
    Ok(())
}

fn check_type(path: &str, value: &Value, type_field: &Value) -> Result<()> {
    let accepted: Vec<&str> = match type_field {
        Value::String(s) => vec![s.as_str()],
        Value::Array(arr) => arr.iter().filter_map(|v| v.as_str()).collect(),
        _ => return Ok(()),
    };
    let actual = json_type(value);
    let ok = accepted.iter().any(|t| {
        *t == actual
            || (*t == "number" && actual == "integer")
            || (*t == "integer" && actual == "integer")
    });
    if !ok {
        return Err(SettingsError::SchemaViolation {
            path: if path.is_empty() { "<root>".into() } else { path.into() },
            message: format!(
                "expected type {} but got `{}`",
                accepted.join(" | "),
                actual,
            ),
        });
    }
    Ok(())
}

fn json_type(value: &Value) -> &'static str {
    match value {
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

fn join_path(prefix: &str, segment: &str) -> String {
    if prefix.is_empty() {
        segment.to_string()
    } else {
        format!("{prefix}.{segment}")
    }
}

// ---------------------------------------------------------------------------
// Inline unit tests (small helpers only — see tests/settings_smoke.rs for the
// end-to-end coverage).
// ---------------------------------------------------------------------------

#[cfg(test)]
mod inline {
    use super::*;
    use serde_json::json;

    #[test]
    fn deep_merge_overlapping_scalar_later_wins() {
        let mut dst = json!({"a": 1, "b": 2});
        deep_merge(&mut dst, json!({"b": 99, "c": 3}));
        assert_eq!(dst, json!({"a": 1, "b": 99, "c": 3}));
    }

    #[test]
    fn deep_merge_array_replaces_not_concats() {
        let mut dst = json!({"xs": [1, 2, 3]});
        deep_merge(&mut dst, json!({"xs": [9]}));
        assert_eq!(dst, json!({"xs": [9]}));
    }

    #[test]
    fn navigate_object_path() {
        let v = json!({"a": {"b": {"c": "ok"}}});
        assert_eq!(navigate(&v, "a.b.c"), Some(&Value::String("ok".into())));
    }

    #[test]
    fn navigate_array_index() {
        let v = json!({"xs": [10, 20, 30]});
        assert_eq!(navigate(&v, "xs.1"), Some(&Value::from(20)));
    }

    #[test]
    fn env_to_json_nests_via_double_underscore() {
        let env = [
            ("APHRODY_FOO__BAR".to_string(), "baz".to_string()),
            ("APHRODY_FOO__QUX".to_string(), "42".to_string()),
            ("UNRELATED".to_string(), "x".to_string()),
        ];
        let v = env_to_json(env, "APHRODY_");
        assert_eq!(v, json!({"foo": {"bar": "baz", "qux": 42}}));
    }

    #[test]
    fn layer_ordered_is_user_project_local_env() {
        assert_eq!(
            SettingsLayer::ordered(),
            [
                SettingsLayer::User,
                SettingsLayer::Project,
                SettingsLayer::Local,
                SettingsLayer::Env,
            ]
        );
    }
}
