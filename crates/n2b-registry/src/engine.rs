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

//! Moteur de matching registre ↔ findings.
//!
//! **État Phase 1** : squelette. La logique vit encore dans `n2b-rules`
//! (qui lit le registre via `crate::registry::{APIS, PACKAGES, ...}`).
//! Phase 2 fera passer le matching JS/TS par ici avec un `ImportGraph` AST.

use crate::schema::MatchInput;
use n2b_types::types::Finding;

/// Squelette — implémenté en Phase 2.
pub fn match_rules(_input: MatchInput<'_>) -> Vec<Finding> {
    Vec::new()
}
