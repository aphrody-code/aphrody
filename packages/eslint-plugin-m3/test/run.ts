/**
 * test/run.ts — exécute oxlint réel avec le plugin m3 sur les fixtures.
 * Vérifie que CHAQUE règle se déclenche sur bad.tsx et qu'AUCUNE sur good.tsx.
 *
 * Lancer : bun test/run.ts   (depuis packages/eslint-plugin-m3/)
 */
import { spawnSync } from "node:child_process";
import { dirname, join } from "node:path";

const ROOT = join(dirname(new URL(import.meta.url).pathname), "..");
const CONFIG = join(ROOT, "test", ".oxlintrc.json");

const RULES = [
  "valid-icon-name",
  "valid-color-role",
  "no-sx-prop",
  "no-mui-import",
  "no-mui-prop-on-md",
  "prefer-icon-token",
  "require-icon-button-label",
  "no-hardcoded-color",
];

function lint(file: string): string {
  const r = spawnSync("bunx", ["oxlint", "-c", CONFIG, join(ROOT, "test", "fixtures", file)], {
    cwd: ROOT,
    encoding: "utf8",
  });
  return (r.stdout || "") + (r.stderr || "");
}

/** Règles m3 déclenchées (par leur slug) dans une sortie oxlint texte. */
function firedRules(output: string): Set<string> {
  const fired = new Set<string>();
  for (const m of output.matchAll(/m3\(([a-z-]+)\)/g)) fired.add(m[1]);
  return fired;
}

let failures = 0;

// 1) bad.tsx : chaque règle doit tirer au moins une fois.
const badOut = lint("bad.tsx");
const badFired = firedRules(badOut);
for (const rule of RULES) {
  if (badFired.has(rule)) {
    console.log(`PASS  bad.tsx déclenche m3/${rule}`);
  } else {
    failures++;
    console.log(`FAIL  bad.tsx NE déclenche PAS m3/${rule}`);
  }
}

// 2) good.tsx : aucune règle m3 ne doit tirer.
const goodFired = firedRules(lint("good.tsx"));
if (goodFired.size === 0) {
  console.log("PASS  good.tsx ne déclenche aucune règle m3");
} else {
  failures++;
  console.log(`FAIL  good.tsx déclenche : ${[...goodFired].join(", ")}`);
}

if (failures > 0) {
  console.error(`\n${failures} échec(s).`);
  process.exit(1);
}
console.log(
  `\nToutes les règles m3 vérifiées (${RULES.length}/${RULES.length} sur bad, 0 sur good).`,
);
