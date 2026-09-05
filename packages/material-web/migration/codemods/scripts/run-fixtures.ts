/**
 * scripts/run-fixtures.ts — test rapide des transforms sur les fixtures.
 *
 * Pour chaque `<name>.input.tsx` de `__testfixtures__/`, applique l'orchestrateur
 * et compare au `<name>.output.tsx` attendu. Sortie non-zero si un écart.
 *
 * Lancer : `bun run scripts/run-fixtures.ts`  (depuis `migration/codemods/`)
 * Régénérer les attendus : `bun run scripts/run-fixtures.ts --update`
 */
import { execFileSync } from "node:child_process";
import { copyFileSync, readFileSync, writeFileSync, mkdtempSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";

// CommonJS (package.json type=commonjs) : __dirname est défini. Évite
// import.meta.url qui imposerait un module ESM au typecheck.
const ROOT = join(__dirname, "..");
const FIXTURES = join(ROOT, "__testfixtures__");
const transformPath = (t: string) => join(ROOT, "transforms", `${t}.ts`);
const update = process.argv.includes("--update");

// Chaque fixture est associée à son transform (défaut : orchestrateur). Les
// icônes ont un transform dédié (icons.ts) car hors périmètre de l'orchestrateur.
const cases: { name: string; transform: string }[] = [
  { name: "button", transform: "orchestrator" },
  { name: "textfield", transform: "orchestrator" },
  { name: "checkbox", transform: "orchestrator" },
  { name: "dialog", transform: "orchestrator" },
  { name: "icons", transform: "icons" },
];

function runTransform(inputPath: string, transform: string): string {
  const tmp = mkdtempSync(join(tmpdir(), "m3cm-"));
  const work = join(tmp, "f.tsx");
  copyFileSync(inputPath, work);
  execFileSync(
    "bunx",
    ["jscodeshift", "-t", transformPath(transform), "--parser=tsx", "--extensions=tsx", work],
    { cwd: ROOT, stdio: "ignore" },
  );
  return readFileSync(work, "utf8");
}

/**
 * Normalise un source TSX via oxfmt (style canonique du dépôt : tabs/quotes,
 * wrapping). Rend la comparaison fixtures stable face au hook PostToolUse
 * (oxfmt) qui reformaterait sinon les `.output.tsx` versionnés à chaque édition.
 * Les deux côtés (`actual` brut recast + `expected` versionné) passent par oxfmt
 * avant comparaison → on teste la sémantique du codemod, pas le style d'impression.
 */
function format(source: string): string {
  return execFileSync("bunx", ["oxfmt", "--stdin-filepath=f.tsx"], {
    cwd: ROOT,
    input: source,
    encoding: "utf8",
  });
}

let failures = 0;
for (const { name, transform } of cases) {
  const input = join(FIXTURES, `${name}.input.tsx`);
  const expectedPath = join(FIXTURES, `${name}.output.tsx`);
  const actual = format(runTransform(input, transform));
  if (update) {
    writeFileSync(expectedPath, actual);
    console.log(`UPDATED ${name}.output.tsx`);
    continue;
  }
  const expected = format(readFileSync(expectedPath, "utf8"));
  if (actual === expected) {
    console.log(`PASS  ${name}`);
  } else {
    failures++;
    console.log(`FAIL  ${name}`);
    // petit diff ligne à ligne
    const a = actual.split("\n");
    const e = expected.split("\n");
    const n = Math.max(a.length, e.length);
    for (let i = 0; i < n; i++) {
      if (a[i] !== e[i]) {
        console.log(
          `  L${i + 1}\n   attendu: ${JSON.stringify(e[i])}\n   obtenu : ${JSON.stringify(a[i])}`,
        );
      }
    }
  }
}

if (!update && failures > 0) {
  console.error(`\n${failures} fixture(s) en échec.`);
  process.exit(1);
}
console.log(update ? "\nFixtures régénérées." : "\nTous les fixtures passent.");
