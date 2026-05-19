// Copyright 2026 Yohan Pierre
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     https://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Severity {
    Info,
    Warn,
    Error,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Mode {
    Check,
    Fix,
    Aggressive,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Report {
    Text,
    Json,
    Jsonl,
    Markdown,
    Sarif,
}

#[derive(Debug, Clone, Serialize)]
pub struct Finding {
    pub file: String,
    pub line: u32,
    pub col: u32,
    #[serde(rename = "ruleId")]
    pub rule_id: String,
    pub severity: Severity,
    pub message: String,
    pub original: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub replacement: Option<String>,
    pub autofix: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub aggressive: Option<bool>,
    /// Phase 3+ : statut de compat Bun du module hôte. Présent uniquement sur
    /// les findings `imports/node-*` et (Phase 4+) `api/node-*`. Optionnel —
    /// rétro-compat schema_version=2.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub compat: Option<CompatInfo>,
}

/// Métadonnée de compat attachée à un Finding (Phase 3+). Mirror du
/// `Compat` du schéma JSON v2 — pas généré pour préserver la simplicité
/// d'API runtime (les types générés sont consommés au boundary I/O).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CompatInfo {
    pub status: CompatStatus,
    pub module: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub missing_apis: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub equivalent: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bunpp: Option<String>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum CompatStatus {
    Full,
    Partial,
    Missing,
}

#[derive(Debug, Clone)]
pub struct FileFix {
    pub file: String,
    pub before: String,
    pub after: String,
    pub findings: Vec<Finding>,
}

#[derive(Debug, Clone)]
pub struct RunOptions {
    pub root: std::path::PathBuf,
    pub mode: Mode,
    pub report: Report,
    pub quiet: bool,
    pub ignore: Vec<String>,
    /// Mode agent : pas de couleurs, logs sur stderr uniquement, stdout réservé au payload.
    #[allow(dead_code)]
    pub agent: bool,
    /// Dry-run : applique les transformations en mémoire mais n'écrit rien
    /// sur le disque. Utilisé par `n2b patch --self`.
    pub dry_run: bool,
}

#[derive(Default)]
pub struct MakeFindingOpts {
    pub severity: Option<Severity>,
    pub autofix: Option<bool>,
    pub aggressive: Option<bool>,
    pub compat: Option<CompatInfo>,
}
