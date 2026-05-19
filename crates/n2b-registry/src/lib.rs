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

//! n2b-registry — registre data-driven des règles de migration Node→Bun.
//!
//! Source unique de vérité pour les règles : 5 fichiers `.toml` embarqués via
//! `include_str!`, chargés et validés au premier accès via `once_cell::Lazy`.
//! Cf. plan/03-registre-spec.md pour la spec complète.

pub mod engine;
pub mod registry;
pub mod schema;

pub use registry::{APIS, CLI, GLOBALS, MODULES, PACKAGES};
pub use schema::{
    ApiEntry, BindingKind, CliEntry, Compat, Confidence, GlobalContext, GlobalEntry, ImportBinding,
    ImportGraph, MatchInput, ModuleEntry, PackageEntry, PackageStrategy, ReplaceKind, Rewrite,
    derive_severity,
};
