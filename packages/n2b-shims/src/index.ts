// Copyright 2026 aphrody-code
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

// shims — Bun-native implementations for the Node.js patterns n2b flags.
// Import by namespace (`import { env, fs, path, shell } from "@n2b/core/shims"`)
// or by scoped subpath (`import { readText } from "@n2b/core/shims/fs"`).

export * as env from "./env";
export * as fs from "./fs";
export * as path from "./path";
export * as shell from "./shell";

// Re-export EnvError for callers that want to instanceof-check.
export { EnvError } from "./env";
