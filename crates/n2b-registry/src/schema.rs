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

//! Structs de désérialisation des `registry/*.toml` + types partagés
//! (MatchInput, ImportGraph) consommés par `engine.rs` et les scanners.
//!
//! Cf. plan/03-registre-spec.md pour la spec exhaustive de chaque champ.

use n2b_types::types::Severity;
use serde::Deserialize;

// ---------------------------------------------------------------------------
// Enums communs
// ---------------------------------------------------------------------------

/// Statut de compatibilité Bun d'un module Node — pilote la sévérité dérivée.
#[derive(Deserialize, Debug, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Compat {
    Full,
    Partial,
    Missing,
}

/// Stratégie de réécriture mécanique d'un pattern.
#[derive(Deserialize, Debug, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Rewrite {
    /// Substitution par template avec placeholders `{0}..{n}` ou `$1..$n`.
    Template,
    /// Pas mécanisable — le finding porte un `codemod_hint`.
    Manual,
    /// L'appel disparaît (ex. `dotenv.config()` → `.env` autoload natif).
    Drop,
}

/// Confiance dans la réécriture (priorité de matching, signal qualité).
#[derive(Deserialize, Debug, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Confidence {
    Low,
    Medium,
    High,
}

/// Stratégie d'un package npm dans `packages.toml`.
#[derive(Deserialize, Debug, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum PackageStrategy {
    /// La dep + ses appels disparaissent (fonctionnalité native Bun).
    Drop,
    /// Remplacement mécanique import + appels.
    Rewrite,
    /// Pas d'équivalent direct — pointer vers `@n2b/shims` ou réécriture manuelle.
    Shim,
}

/// Type de binding d'import (résolu par l'AST oxc).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BindingKind {
    Default,
    Named { imported: String },
    Namespace,
    Require,
}

/// Contexte d'application d'un global (CJS vs ESM, dynamique vs statique).
#[derive(Deserialize, Debug, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum GlobalContext {
    Esm,
    Cjs,
    Any,
    DynamicArg,
}

/// Compat avec l'enum `ReplaceKind` de l'ancien `bun_apis.rs` —
/// préservé pour la couche de compat pendant la migration Phase 1.
#[derive(Debug, Clone)]
pub enum ReplaceKind {
    None,
    Static(String),
    Template(String),
}

// ---------------------------------------------------------------------------
// Entrées du registre
// ---------------------------------------------------------------------------

/// Entrée `modules.toml` — un module `node:*`.
#[derive(Deserialize, Debug, Clone)]
pub struct ModuleEntry {
    pub id: String,
    pub module: String,
    #[serde(default = "default_compat_full")]
    pub compat: Compat,
    #[serde(default)]
    pub bun_reimpl: bool,
    #[serde(default)]
    pub missing_apis: Vec<String>,
    #[serde(default)]
    pub equivalent: String,
    #[serde(default)]
    pub severity: Option<Severity>,
    #[serde(default)]
    pub rewrite_hint: Option<String>,
    #[serde(default)]
    pub bunpp: Option<String>,
    #[serde(default)]
    pub docs: String,
}

fn default_compat_full() -> Compat {
    Compat::Full
}

/// Entrée `apis.toml` — un pattern d'appel API/méthode Node.
///
/// Champs Phase 1 : ne porte pas encore `compat`/`severity` dérivée — la
/// sévérité reste explicite (héritage de l'ancien code) pour préserver la
/// sortie octet-identique. Phase 3 ajoute la dérivation.
#[derive(Deserialize, Debug, Clone)]
pub struct ApiEntry {
    pub id: String,
    pub pattern: String,
    pub message: String,
    #[serde(default)]
    pub import_from: Option<String>,
    #[serde(default = "default_severity_warn")]
    pub severity: Severity,
    #[serde(default)]
    pub aggressive: bool,
    /// Stratégie de réécriture explicite ("none" | "static" | "template").
    /// Préserve la forme exacte de l'ancien `ReplaceKind`.
    #[serde(default = "default_replace_none")]
    pub replace: String,
    /// Cible de la réécriture quand `replace = "static"` ou `"template"`.
    #[serde(default)]
    pub replacement: Option<String>,
    /// Champs Phase 3+ — non utilisés en Phase 1.
    #[serde(default)]
    pub compat: Option<Compat>,
    #[serde(default)]
    pub rewrite: Option<Rewrite>,
    #[serde(default)]
    pub template: Option<String>,
    #[serde(default)]
    pub codemod_hint: Option<String>,
    #[serde(default)]
    pub confidence: Option<Confidence>,
    #[serde(default)]
    pub docs: String,
}

fn default_severity_warn() -> Severity {
    Severity::Warn
}

fn default_replace_none() -> String {
    "none".to_string()
}

/// Entrée `packages.toml` — une dep npm avec sa stratégie de remplacement.
#[derive(Deserialize, Debug, Clone)]
pub struct PackageEntry {
    pub id: String,
    pub package: String,
    pub replacement: String,
    pub note: String,
    #[serde(default)]
    pub aggressive: bool,
    #[serde(default)]
    pub strategy: Option<PackageStrategy>,
    #[serde(default)]
    pub target: Option<String>,
    #[serde(default)]
    pub apis: Vec<String>,
    #[serde(default)]
    pub docs: String,
}

/// Entrée `cli.toml` — une commande npm/pnpm/yarn → bun.
#[derive(Deserialize, Debug, Clone)]
pub struct CliEntry {
    pub id: String,
    pub pattern: String,
    pub replace: String,
    pub message: String,
    #[serde(default = "default_true")]
    pub respect_comments: bool,
    #[serde(default)]
    pub docs: String,
}

fn default_true() -> bool {
    true
}

/// Entrée `globals.toml` — un global Node (`__dirname`, `require`, …).
#[derive(Deserialize, Debug, Clone)]
pub struct GlobalEntry {
    pub id: String,
    pub symbol: String,
    pub bun: String,
    pub context: GlobalContext,
    pub rewrite: Rewrite,
    #[serde(default)]
    pub template: Option<String>,
    #[serde(default)]
    pub codemod_hint: Option<String>,
    #[serde(default = "default_severity_warn")]
    pub severity: Severity,
    #[serde(default)]
    pub docs: String,
}

// ---------------------------------------------------------------------------
// Wrappers de fichiers (pour la désérialisation toml)
// ---------------------------------------------------------------------------

#[derive(Deserialize, Debug)]
pub(crate) struct ApisFile {
    #[serde(default)]
    pub apis: Vec<ApiEntry>,
}

#[derive(Deserialize, Debug)]
pub(crate) struct PackagesFile {
    #[serde(default)]
    pub packages: Vec<PackageEntry>,
}

#[derive(Deserialize, Debug)]
pub(crate) struct ModulesFile {
    #[serde(default)]
    pub modules: Vec<ModuleEntry>,
}

#[derive(Deserialize, Debug)]
pub(crate) struct CliFile {
    #[serde(default)]
    pub cli: Vec<CliEntry>,
}

#[derive(Deserialize, Debug)]
pub(crate) struct GlobalsFile {
    #[serde(default)]
    pub globals: Vec<GlobalEntry>,
}

// ---------------------------------------------------------------------------
// MatchInput — interface scanner ↔ engine
// ---------------------------------------------------------------------------

/// Binding local résolu par l'AST oxc (Phase 2).
#[derive(Debug, Clone)]
pub struct ImportBinding {
    pub specifier: String,
    pub kind: BindingKind,
}

/// Graphe d'imports d'un fichier — résout PS1 (matching nominal sans contexte).
///
/// Phase 1 : squelette vide, `resolves` retourne toujours `false` —
/// `bun_apis.rs` continue à utiliser le matching textuel. Phase 2 le peuple.
#[derive(Debug, Default, Clone)]
pub struct ImportGraph {
    pub bindings: std::collections::HashMap<String, ImportBinding>,
}

impl ImportGraph {
    pub fn new() -> Self {
        Self::default()
    }

    /// Le symbole `name` provient-il d'un import du specifier `from` ?
    /// Phase 1 : non implémenté — toujours `false`.
    pub fn resolves(&self, name: &str, from: &str) -> bool {
        match self.bindings.get(name) {
            Some(b) => b.specifier == from,
            None => false,
        }
    }
}

/// Entrée du moteur de matching — JS/TS via AST, reste via texte brut.
pub enum MatchInput<'a> {
    Ast {
        source: &'a str,
        imports: &'a ImportGraph,
    },
    Text {
        source: &'a str,
    },
}

// ---------------------------------------------------------------------------
// Helpers de dérivation
// ---------------------------------------------------------------------------

/// Sévérité dérivée du statut de compat d'un module hôte.
/// Utilisé en Phase 3+ pour piloter `Finding.severity` depuis le registre.
pub fn derive_severity(compat: Compat) -> Severity {
    match compat {
        Compat::Full => Severity::Info,
        Compat::Partial => Severity::Warn,
        Compat::Missing => Severity::Error,
    }
}
