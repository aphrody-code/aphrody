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

// path.ts — shims for the `__dirname` / `__filename` / `fileURLToPath`
// patterns that n2b flags as `api/fileURLToPath` and `api/__dirname`.
//
// Bun exposes `import.meta.dir` and `import.meta.path` natively which
// are canonical, but legacy code often writes `fileURLToPath(import.meta.url)`
// — this module provides Bun-native one-liners that make the migration
// mechanical.

import { dirname, resolve } from "node:path";

/**
 * Return the directory of the file that owns `importMeta`. Replaces
 * `path.dirname(fileURLToPath(import.meta.url))`.
 * Bun-native equivalent: `import.meta.dir`.
 */
export function dirOf(importMeta: ImportMeta): string {
  // Bun has import.meta.dir, Node does not. Fall back to parsing import.meta.url
  // when dir is absent (non-Bun runtimes, edge cases).
  const dir = (importMeta as ImportMeta & { dir?: string }).dir;
  if (typeof dir === "string") return dir;
  const url = importMeta.url;
  if (url.startsWith("file://")) return dirname(url.slice("file://".length));
  return dirname(url);
}

/**
 * Return the absolute path of the file that owns `importMeta`. Replaces
 * `fileURLToPath(import.meta.url)`.
 * Bun-native equivalent: `import.meta.path`.
 */
export function fileOf(importMeta: ImportMeta): string {
  const path = (importMeta as ImportMeta & { path?: string }).path;
  if (typeof path === "string") return path;
  const url = importMeta.url;
  if (url.startsWith("file://")) return url.slice("file://".length);
  return url;
}

/**
 * Resolve a path relative to the file owning `importMeta`. Replaces
 * `path.resolve(path.dirname(fileURLToPath(import.meta.url)), …)`.
 */
export function relativeTo(importMeta: ImportMeta, ...segments: string[]): string {
  return resolve(dirOf(importMeta), ...segments);
}
