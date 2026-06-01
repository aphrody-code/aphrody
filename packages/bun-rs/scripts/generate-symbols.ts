import { readFileSync, writeFileSync } from "fs";
import { join } from "path";

const root = "/home/ubuntu/material-web";
const jsonPath = join(root, "migration/codemods/data/material-symbols-names.json");
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
