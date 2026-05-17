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

// fs.ts — Bun-native wrappers for the fs patterns n2b flags most often
// (`api/fs-readFileSync`, `api/fs-writeFileSync`, `api/fs-existsSync`).
//
// Bun.file() is async-by-default ; these helpers expose sync-looking APIs
// backed by Bun.file's lazy loading, and async helpers that are 2-3× faster
// than node:fs on hot paths. Every helper short-circuits to a Bun.file()
// call — no node:fs import, no Node polyfill.

/**
 * Read a file as UTF-8 text, async. Replaces `fs.readFile(path, "utf8")`
 * and `fs.readFileSync(path, "utf8")` when you can afford async.
 */
export async function readText(path: string): Promise<string> {
  return await Bun.file(path).text();
}

/**
 * Read a file as parsed JSON, async. Replaces `JSON.parse(await readFile(...))`.
 */
export async function readJson<T = unknown>(path: string): Promise<T> {
  return (await Bun.file(path).json()) as T;
}

/**
 * Read a file as ArrayBuffer, async. Faster than fs.readFile for binary.
 */
export async function readBytes(path: string): Promise<ArrayBuffer> {
  return await Bun.file(path).arrayBuffer();
}

/**
 * Write text to a file, async. Replaces `fs.writeFileSync(path, data, "utf8")`.
 */
export async function writeText(path: string, content: string): Promise<number> {
  return await Bun.write(path, content);
}

/**
 * Write JSON to a file, async (pretty-printed with 2-space indent).
 */
export async function writeJson(path: string, value: unknown, indent = 2): Promise<number> {
  return await Bun.write(path, JSON.stringify(value, null, indent) + "\n");
}

/**
 * Check file existence. Replaces `fs.existsSync(path)`.
 * Bun.file() is lazy — this is a single stat() syscall.
 */
export async function exists(path: string): Promise<boolean> {
  return await Bun.file(path).exists();
}

/**
 * Return file size in bytes, or `null` if the file does not exist.
 */
export async function size(path: string): Promise<number | null> {
  const f = Bun.file(path);
  return (await f.exists()) ? f.size : null;
}
