import { readFileSync, writeFileSync } from "node:fs";
import { join } from "node:path";

// Resolve the repo root relative to this script
// (packages/bun-rs/scripts/ -> repo root). material-web was merged in-tree on
// 2026-06-01, so both the input JSON and the output Rust file live under packages/.
const root = join(import.meta.dir, "../../..");
const jsonPath = join(root, "packages/material-web/migration/codemods/data/material-symbols-names.json");
const rustPath = join(root, "packages/bun-rs/src/symbols.rs");

console.log("Reading symbols JSON from:", jsonPath);
const symbols: string[] = JSON.parse(readFileSync(jsonPath, "utf-8"));

// Sort alphabetically to ensure binary_search in Rust works correctly
symbols.sort();

console.log(`Loaded ${symbols.length} symbol names. Generating Rust file...`);

let content = `// SPDX-License-Identifier: Apache-2.0
// AUTO-GENERATED from material-symbols-names.json. Do not edit manually.

pub const MATERIAL_SYMBOLS: &[&str] = &[
`;

for (const name of symbols) {
  content += `\t"${name}",\n`;
}

content += "];\n";

writeFileSync(rustPath, content, "utf-8");
console.log("Wrote Rust symbols file to:", rustPath);
