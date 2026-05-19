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

pub mod bun_apis;
pub mod cli_commands;
pub mod imports_ast;
pub mod node_imports;

// `dotnet` branch (see README-dotnet.md): Windows 11 + dotnet 10 + node-api-dotnet
// + WinClean-specific rules. NOT wired into the default scanner — that would
// break the frozen baselines (tests/snapshots/baseline/). The `apply_*_rules`
// functions are public API; activate via a future `--dotnet` CLI flag or by
// calling them explicitly from a custom scanner.
pub mod dotnet;
pub mod node_api_dotnet;
pub mod winclean;
pub mod windows;
