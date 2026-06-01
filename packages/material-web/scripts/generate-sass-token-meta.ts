// SPDX-License-Identifier: Apache-2.0
//
// Generates tokens/versions/**/_md-{name}-meta.scss from each _md-*.scss token
// partial. Bun-native (Bun.Glob + Bun.file/Bun.write) and self-batching — one
// process for every token file, no `xargs` per-file spawn.
import { resolve } from "node:path";

const PKG = resolve(import.meta.dir, "..");

function toMeta(content: string): string {
  const metaVars: string[] = [];
  const varRegex = /^\s*\$([\w-]+)\s*:\s*([^;]+);/gm;
  let match: RegExpExecArray | null;
  while ((match = varRegex.exec(content)) !== null) {
    const name = match[1];
    let value = match[2].trim();
    // Resolve a token reference (md-sys-color.$primary / $primary) to a CSS var().
    const refMatch = value.match(/^(?:([\w-]+)\.)?\$([\w-]+)$/);
    if (refMatch) {
      const refModule = refMatch[1] || null;
      value = `var(--${refModule ? `${refModule}-` : ""}${refMatch[2]})`;
    }
    metaVars.push(`$${name}--resolved: ${value};`);
  }
  return (
    `//\n// Copyright 2026 Google LLC\n// SPDX-License-Identifier: Apache-2.0\n//\n` +
    `// Auto-generated token metadata.\n${metaVars.join("\n")}\n`
  );
}

let count = 0;
for await (const rel of new Bun.Glob("tokens/versions/**/_md-*.scss").scan({
  cwd: PKG,
})) {
  if (rel.endsWith("-meta.scss")) continue;
  const input = resolve(PKG, rel);
  const output = input.replace(/\.scss$/, "-meta.scss");
  await Bun.write(output, toMeta(await Bun.file(input).text()));
  count++;
}
console.log(`sass token meta: ${count} files`);
