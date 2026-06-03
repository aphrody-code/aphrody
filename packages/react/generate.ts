/**
 * Generator for @aphrody/m3-react.
 *
 * Reads migration/scripts/md-elements.txt (119 tags), resolves each tag to its
 * real entry file + exported element class inside material-web/ by scanning the
 * `@customElement('md-x') export class Foo` pattern, and emits one wrapper file
 * per category directory using @lit/react `createComponent`.
 *
 * Run with bun:  bun run generate.ts
 */
import { readFileSync, writeFileSync, mkdirSync, readdirSync, statSync } from "node:fs";
import { join, relative, dirname } from "node:path";
import { fileURLToPath } from "node:url";

const HERE = dirname(fileURLToPath(import.meta.url));
const MW = join(HERE, "..", "material-web"); // material-web repo root
const ELEMENTS_TXT = join(HERE, "md-elements.txt");
const OUT_DIR = join(HERE, "wrappers");

// ── 1. Tag → PascalCase wrapper name (CONVENTIONS §2) ──────────────────────
const pascal = (tag: string): string =>
  tag
    .split("-")
    .map((s) => s.charAt(0).toUpperCase() + s.slice(1))
    .join("");

// ── 2. Walk material-web for every `@customElement('md-…')` entry file ──────
// Skip tests, internal/, demos, catalog. Collect tag → [{file, className}].
const SKIP_DIRS = new Set([
  "node_modules",
  "internal",
  "demo",
  "demos",
  "catalog",
  "dist-aphrody",
  "sass",
  "testing",
]);
const RE = /@customElement\(\s*['"]([a-z0-9-]+)['"]\s*\)\s*\nexport class (\w+)/g;

interface Def {
  file: string;
  className: string;
}

type EventMap = Record<string, string>;

interface Resolved {
  tag: string;
  name: string;
  file: string;
  className: string;
  events: EventMap;
}

interface Unresolved {
  tag: string;
  reason: string;
}

const defs: Map<string, Def[]> = new Map();

function walk(dir: string): void {
  for (const name of readdirSync(dir)) {
    const full = join(dir, name);
    const st = statSync(full);
    if (st.isDirectory()) {
      if (SKIP_DIRS.has(name)) continue;
      walk(full);
    } else if (name.endsWith(".ts") && !name.endsWith("_test.ts") && !name.endsWith(".d.ts")) {
      const src = readFileSync(full, "utf8");
      let m;
      RE.lastIndex = 0;
      while ((m = RE.exec(src))) {
        const [, tag, className] = m;
        if (!tag.startsWith("md-")) continue;
        const rel = relative(MW, full).replace(/\.ts$/, ".js"); // import as .js (side-effect register)
        if (!defs.has(tag)) defs.set(tag, []);
        defs.get(tag)!.push({ file: rel, className });
      }
    }
  }
}
walk(MW);

// ── 3. Canonical import path preference ────────────────────────────────────
// Tags re-exported by the three bundles must import from the bundled entry
// path. Build the set of canonical relative paths from those bundles.
const bundleImports = new Set<string>();
for (const bundle of ["all.ts", "aphrody-components.ts", "aphrody-labs.ts"]) {
  const src = readFileSync(join(MW, bundle), "utf8");
  for (const m of src.matchAll(/from ['"]\.\/([^'"]+\.js)['"]/g)) bundleImports.add(m[1]);
}

function pickDef(tag: string, candidates: Def[]): Def {
  // 1. Prefer a candidate whose path is referenced by a bundle entry.
  const bundled = candidates.find((c) => bundleImports.has(c.file));
  if (bundled) return bundled;
  // 2. Otherwise prefer a non-labs/gb path (canonical source dir).
  const nonGb = candidates.find((c) => !c.file.startsWith("labs/gb/"));
  if (nonGb) return nonGb;
  // 3. Fall back to the first (labs/gb only — e.g. md-button, md-split-button).
  return candidates[0];
}

// ── 4. Event maps per category (verified against new Event/CustomEvent in MW) ─
// Generic UI events React already types; the md elements re-dispatch the native
// `input`/`change`. Component-specific events use their real names.
// Every name below is the REAL event string verified by grepping
// `@fires` / `new (Custom)?Event('…')` in material-web/<dir>/internal/*.ts.
// Upstream components use bare names (input/change/opening/…); aphrody fork
// components namespace theirs (paginator:page, table:sort, side-sheet:opening…).
const EVENTS_BY_TAG: Record<string, EventMap> = {
  // ── fork: parity additions (event names verified against source dispatch) ──
  "md-rating": { onChange: "change", onInput: "input" },
  "md-carousel": { onChange: "carousel-change" },
  "md-scheduler": {
    onNavigate: "scheduler:navigate",
    onSelect: "scheduler:select",
    onEventClick: "scheduler:event-click",
  },
  "md-date-range-picker": {
    onInput: "input",
    onChange: "change",
    onDateRangeChange: "date-range-picker:change",
  },
  "md-date-time-picker": {
    onInput: "input",
    onChange: "change",
    onDateTimeChange: "date-time-picker:change",
  },
  "md-popover": { onOpened: "opened", onClosed: "closed" },
  "md-alert": { onClose: "close" },
  "md-avatar": { onError: "error" },
  // ── Form inputs (upstream: re-dispatched native input/change) ──────────
  "md-checkbox": { onChange: "change", onInput: "input" },
  "md-radio": { onChange: "change", onInput: "input" },
  "md-switch": { onChange: "change", onInput: "input" },
  "md-slider": { onChange: "change", onInput: "input" },
  "md-filled-text-field": { onChange: "change", onInput: "input" },
  "md-outlined-text-field": { onChange: "change", onInput: "input" },
  "md-filled-select": {
    onChange: "change",
    onInput: "input",
    onOpening: "opening",
    onOpened: "opened",
    onClosing: "closing",
    onClosed: "closed",
  },
  "md-outlined-select": {
    onChange: "change",
    onInput: "input",
    onOpening: "opening",
    onOpened: "opened",
    onClosing: "closing",
    onClosed: "closed",
  },
  // fork: search-bar fires input + search + search:open/close
  "md-search-bar": {
    onInput: "input",
    onSearch: "search",
    onSearchOpen: "search:open",
    onSearchClose: "search:close",
  },
  // fork: autocomplete fires input + autocomplete:select
  "md-autocomplete": { onInput: "input", onSelect: "autocomplete:select" },
  // ── Overlays (upstream) ────────────────────────────────────────────────
  "md-dialog": {
    onOpen: "open",
    onOpened: "opened",
    onClose: "close",
    onClosed: "closed",
    onCancel: "cancel",
  },
  "md-menu": {
    onOpening: "opening",
    onOpened: "opened",
    onClosing: "closing",
    onClosed: "closed",
  },
  "md-menu-item": { onCloseMenu: "close-menu" },
  "md-sub-menu": {
    onOpening: "opening",
    onOpened: "opened",
    onClosing: "closing",
    onClosed: "closed",
  },
  // fork: snackbar (bare opening/opened/closing/closed)
  "md-snackbar": {
    onOpening: "opening",
    onOpened: "opened",
    onClosing: "closing",
    onClosed: "closed",
  },
  // fork: sheets namespace their lifecycle events
  "md-bottom-sheet": {
    onOpening: "bottom-sheet:opening",
    onOpened: "bottom-sheet:opened",
    onClosing: "bottom-sheet:closing",
    onClosed: "bottom-sheet:closed",
  },
  "md-side-sheet": {
    onOpening: "side-sheet:opening",
    onOpened: "side-sheet:opened",
    onClosing: "side-sheet:closing",
    onClosed: "side-sheet:closed",
  },
  // labs navigation drawer
  "md-navigation-drawer": {
    onNavigationDrawerChanged: "navigation-drawer-changed",
  },
  "md-navigation-drawer-modal": {
    onNavigationDrawerChanged: "navigation-drawer-changed",
  },
  // ── Selection / navigation ─────────────────────────────────────────────
  "md-tabs": { onChange: "change" }, // upstream: bubbling 'change'
  "md-navigation-bar": { onActivated: "navigation-bar-activated" }, // labs
  "md-navigation-rail": {
    onChange: "navigation-rail:change",
    onItemActivate: "navigation-rail-item:activate",
  }, // fork
  "md-input-chip": { onRemove: "remove", onUpdateFocus: "update-focus" },
  "md-button-group": { onChange: "button-group:change" }, // fork
  "md-fab-menu": { onOpen: "fab-menu:open", onClose: "fab-menu:close" }, // fork
  "md-fab-menu-item": { onClick: "fab-menu-item:click" }, // fork
  // ── Disclosure (fork) ──────────────────────────────────────────────────
  "md-expansion-panel": { onToggle: "expansion:toggle" },
  "md-stepper": { onChange: "stepper:change" },
  // ── Pickers (fork) ─────────────────────────────────────────────────────
  "md-date-picker": { onChange: "date-picker:change" },
  "md-time-picker": { onChange: "time-picker:change" },
  // ── Data (fork) ────────────────────────────────────────────────────────
  "md-table": {
    onSort: "table:sort",
    onSelectionChange: "table:selection-change",
  },
  "md-paginator": { onPage: "paginator:page" },
  "md-tree": { onSelect: "tree:select" },
  "md-tree-item": {
    onToggle: "tree-item:toggle",
    onActivate: "tree-item:activate",
  },
  "md-virtual-scroller": { onRange: "virtual-scroll:range" },
  // ── Misc (fork) ────────────────────────────────────────────────────────
  "md-tooltip": { onShow: "tooltip-show", onHide: "tooltip-hide" },
};

// ── 5. Resolve every requested tag ─────────────────────────────────────────
const tags = readFileSync(ELEMENTS_TXT, "utf8")
  .split("\n")
  .map((s) => s.trim())
  .filter(Boolean);

const resolved: Resolved[] = [];
const unresolved: Unresolved[] = [];

for (const tag of tags) {
  const candidates = defs.get(tag);
  if (!candidates || candidates.length === 0) {
    unresolved.push({
      tag,
      reason: "no @customElement entry file found in material-web/",
    });
    continue;
  }
  const def = pickDef(tag, candidates);
  resolved.push({
    tag,
    name: pascal(tag),
    file: def.file,
    className: def.className,
    events: EVENTS_BY_TAG[tag] ?? {},
  });
}

// ── 6. Group by top-level category dir, emit one wrapper file per group ─────
const groupOf = (file: string): string => file.split("/")[0].replace(/\..*$/, "");
const groups: Map<string, Resolved[]> = new Map();
for (const r of resolved) {
  const g = groupOf(r.file);
  if (!groups.has(g)) groups.set(g, []);
  groups.get(g)!.push(r);
}

mkdirSync(OUT_DIR, { recursive: true });

const HEADER = `// AUTO-GENERATED by generate.ts — do not edit by hand. Run \`bun run generate\`.\n`;
const fmtEvents = (events: EventMap): string => {
  const keys = Object.keys(events);
  if (keys.length === 0) return "{}";
  const body = keys.map((k) => `${k}: '${events[k]}'`).join(", ");
  return `{${body}}`;
};

const indexLines = [
  HEADER,
  `import * as React from 'react';`,
  `import {createComponent} from '@lit/react';`,
  "",
];
// Group elements that share the same import file (e.g. several tags per module is rare; one file = one element here).
for (const [group, items] of [...groups.entries()].sort()) {
  const fileName = `${group}.ts`;
  const lines = [
    HEADER,
    `import * as React from 'react';`,
    `import {createComponent} from '@lit/react';`,
    "",
  ];
  // imports — alias element class to <Name>Element to avoid clashing with wrapper const.
  for (const r of items) {
    lines.push(
      `import {${r.className} as ${r.name}Element} from '@aphrody/material-web/${r.file}';`,
    );
  }
  lines.push("");
  for (const r of items) {
    lines.push(`export const ${r.name} = createComponent({`);
    lines.push(`  react: React,`);
    lines.push(`  tagName: '${r.tag}',`);
    lines.push(`  elementClass: ${r.name}Element,`);
    lines.push(`  events: ${fmtEvents(r.events)},`);
    lines.push(`});`);
    lines.push("");
  }
  writeFileSync(join(OUT_DIR, fileName), lines.join("\n"));
  indexLines.push(`export * from './wrappers/${group}.js';`.replace("./wrappers/", "./"));
}

// index.ts at package root re-exports all groups.
const idx = [HEADER];
for (const [group] of [...groups.entries()].sort()) {
  idx.push(`export * from './wrappers/${group}.js';`);
}
idx.push("");
writeFileSync(join(HERE, "index.ts"), idx.join("\n"));

// ── 7. Report ──────────────────────────────────────────────────────────────
console.log(`Tags requested: ${tags.length}`);
console.log(`Wrappers generated: ${resolved.length} across ${groups.size} files`);
if (unresolved.length) {
  console.log(`Unresolved (${unresolved.length}):`);
  for (const u of unresolved) console.log(`  - ${u.tag}: ${u.reason}`);
}
// Emit a machine-readable manifest for the README.
writeFileSync(
  join(HERE, ".manifest.json"),
  JSON.stringify(
    {
      resolved: resolved.map(({ tag, name, file, className, events }) => ({
        tag,
        name,
        file,
        className,
        events,
      })),
      unresolved,
    },
    null,
    2,
  ),
);

// Keep the README's component list in sync (it is the last section). This makes
// the wrapper count + table self-maintaining — never stale after a `generate`.
const README = join(HERE, "README.md");
const rows = resolved
  .slice()
  .sort((a, b) => a.name.localeCompare(b.name))
  .map((r) => {
    const ev = Object.keys(r.events ?? {}).length || "—";
    return `| \`${r.name}\` | \`${r.tag}\` | \`@aphrody/material-web/${r.file}\` | ${ev} |`;
  })
  .join("\n");
const listSection =
  `## Liste complète des wrappers (${resolved.length})\n\n` +
  `> Régénérée par \`generate.ts\` (\`.manifest.json\`). ${unresolved.length} non résolu(s).\n\n` +
  `| Wrapper | Tag | Import élément | # events |\n| --- | --- | --- | --- |\n${rows}\n`;
try {
  const readme = readFileSync(README, "utf8");
  const updated = readme.replace(/## Liste complète des wrappers \(\d+\)[\s\S]*$/, listSection);
  if (updated !== readme) writeFileSync(README, updated);
} catch {
  /* README optional */
}
