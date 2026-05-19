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

// n2b-core — moteur métier : scanners, règles, report, run, ai, github, audit.
// Re-export n2b-types (types + schema) pour préserver l'API publique consommée
// par n2b-cli et n2b-native (compat).

pub mod audit;
pub mod llmstxt;
pub mod manifest;
pub mod report_card;
pub mod run;

pub use n2b_ai as ai;
pub use n2b_github as github;
pub use n2b_report as report;
pub use n2b_rules as rules;
pub use n2b_scanners as scanners;
pub use n2b_types::{schema, types};
pub use n2b_util as util;
