// SPDX-License-Identifier: Apache-2.0
//
// Wraps every compiled `*.css` into a `*.cssresult.ts` Lit `CSSResult` module.
// Bun-native (Bun.Glob + Bun.file/Bun.write) and self-batching: one process for
// all stylesheets instead of `xargs -L1 bun …` (one Bun spawn per file).
import { resolve } from "node:path";

const PKG = resolve(import.meta.dir, "..");
const YEAR = new Date().getFullYear();
const SUFFIX = ".cssresult";

function toModule(cssPath: string, css: string): string {
  // Strip the sourceMappingURL comment since the CSS is embedded.
  const body = css.replace(/\/\*#\s*sourceMappingURL=[^*]+\*\//, "");
  return (
    `/**\n * @license\n * Copyright ${YEAR} Google LLC\n` +
    ` * SPDX-License-Identifier: Apache-2.0\n */\n` +
    `// Generated stylesheet for ${cssPath}.\n` +
    `import {css} from 'lit';\n` +
    `export const styles = css\`${body}\`;\n` +
    `export default styles.styleSheet!;\n`
  );
}

let count = 0;
for await (const rel of new Bun.Glob("**/*.css").scan({ cwd: PKG })) {
  if (rel.includes("node_modules")) continue;
  const cssPath = resolve(PKG, rel);
  const out = cssPath.replace(/\.css$/, `${SUFFIX}.ts`);
  await Bun.write(out, toModule(`./${rel}`, await Bun.file(cssPath).text()));
  count++;
}
console.log(`css-to-ts: ${count} stylesheets`);
