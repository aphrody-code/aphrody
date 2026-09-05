/**
 * scripts/export-map-json.ts — exporte le mapping MUI -> M3 en JSON unique,
 * lisible par machine (codemods, skill de migration, outillage tiers).
 *
 * SOURCE DE VÉRITÉ : `lib/mapping.ts` (composants/props/slots/gaps) + `lib/
 * icon-names.ts` (règles icônes) + `data/mui-icon-exceptions.json`. Régénérer
 * après toute modif du mapping :
 *   bun run scripts/export-map-json.ts
 * Sortie : `migration/mui-m3-map.json`.
 */
import { readFileSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import {
  GAP_COMPONENTS,
  LAYOUT_COMPONENTS,
  M3_PKG,
  MUI_PKGS,
  PROP_DROP_WITH_TODO,
  RENAME_CHILDREN,
  SIMPLE_COMPONENTS,
  SLOTTED_CHILDREN,
  VARIANT_COMPONENTS,
} from "../lib/mapping";

const ROOT = join(__dirname, "..");
const MIGRATION = join(ROOT, "..");

const exceptions = JSON.parse(readFileSync(join(ROOT, "data", "mui-icon-exceptions.json"), "utf8"));
const symbolNames: string[] = JSON.parse(
  readFileSync(join(ROOT, "data", "material-symbols-names.json"), "utf8"),
);

const map = {
  $schema: "https://aphrody-code.github.io/material-web/schemas/mui-m3-map.json",
  meta: {
    description:
      "Mapping canonique MUI (@mui/material@9 + @mui/icons-material) -> material-web (@aphrody/m3-react + Material Symbols). Généré depuis migration/codemods/lib/.",
    targetPackage: M3_PKG,
    muiPackages: MUI_PKGS,
    generatedBy: "migration/codemods/scripts/export-map-json.ts",
    docs: ["00-CONVENTIONS.md", "11-material-symbols.md"],
  },
  // Composants pilotés par la prop `variant` (Button, TextField, Select…).
  variantComponents: VARIANT_COMPONENTS,
  // Mapping 1:1 (composant MUI -> wrapper React unique).
  simpleComponents: SIMPLE_COMPONENTS,
  // Sous-composants enfants renommés directement.
  renameChildren: RENAME_CHILDREN,
  // Sous-composants -> contenu slotté (slot=…).
  slottedChildren: SLOTTED_CHILDREN,
  // Layout MUI -> <div> + Tailwind (ou wrapper Md* si défini).
  layoutComponents: LAYOUT_COMPONENTS,
  // Gaps réels (pas d'élément md -> transition/tokens/primitive).
  gapComponents: GAP_COMPONENTS,
  // Props retirées avec TODO (pas d'équivalent direct md).
  propsDroppedWithTodo: [...PROP_DROP_WITH_TODO].sort(),
  // Règles de migration des icônes @mui/icons-material -> Material Symbols.
  icons: {
    strategy:
      "PascalCase MUI -> snake_case Material Symbols (déterministe, ~96% validé). Suffixe Outlined/Rounded/Sharp -> style ; TwoTone -> Outlined. Validation contre material-symbols-names.json.",
    target: "md-icon (texte enfant = nom de glyphe Material Symbols)",
    styles: ["Outlined", "Rounded", "Sharp"],
    defaultStyle: "Outlined",
    axes: {
      FILL: { range: [0, 1], default: 0, token: "--md-icon-fill" },
      wght: { range: [100, 700], default: 400, token: "--md-icon-wght" },
      GRAD: { range: [-50, 200], default: 0, token: "--md-icon-grad" },
      opsz: { range: [20, 48], default: 24, token: "--md-icon-opsz" },
    },
    validSymbolCount: symbolNames.length,
    brandsKeptAsSvg: exceptions.brands,
    remap: exceptions.remap,
  },
  // Conventions d'événements (MUI -> events DOM natifs / namespacés).
  events: {
    onChange:
      "MUI onChange(e, value) -> events DOM natifs input/change ; lire e.target.value (le 2e paramètre disparaît).",
    namespaced: "Composants fork : events namespacés (table:sort, stepper:change, tree:select…).",
    renamedProps: {
      "Switch.checked": "selected",
      "Drawer.open": "opened",
      "Tooltip.title": "text",
      "Tabs.value": "active-tab-index",
      "LinearProgress(0..100)": "value(0..1)",
    },
  },
};

const out = join(MIGRATION, "mui-m3-map.json");
writeFileSync(out, JSON.stringify(map, null, 2) + "\n");

const componentCount =
  Object.keys(VARIANT_COMPONENTS).length +
  Object.keys(SIMPLE_COMPONENTS).length +
  Object.keys(RENAME_CHILDREN).length +
  Object.keys(LAYOUT_COMPONENTS).length +
  Object.keys(SLOTTED_CHILDREN).length;
console.log(
  `mui-m3-map.json écrit : ${componentCount} composants mappés, ${Object.keys(GAP_COMPONENTS).length} gaps, ${symbolNames.length} glyphes Material Symbols.`,
);
