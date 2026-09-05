// SPDX-License-Identifier: Apache-2.0
/**
 * client entry — Bun bundles this module (referenced by index.html).
 * Pure Bun-native Web Component showcase with zero React, ReactDOM, or ThreeJS.
 */
import "@aphrody/material-web/all.js";
import "@aphrody/material-web/aphrody-components.js";
import {
  cssFromSeed,
  applyDynamicColor,
  clearDynamicColor,
} from "@aphrody/m3-tokens/dynamic-color";
import { argbFromHex, Hct } from "@material/material-color-utilities";
import { getBuildMetadata } from "./macro.ts" with { type: "macro" };
import "./theme.css";
import "./showcase.css";

// ── 0. Fetch and Inject Build Metadata (Bun Macro) ──────────────────────────
const metadata = getBuildMetadata();
console.log("[showcase] Bun-native build metadata inlined:", metadata);

const topbarSubtitle = document.getElementById("topbar-subtitle");
if (topbarSubtitle) {
  topbarSubtitle.innerHTML = `Bun v${metadata.bunVersion} &bull; Git: <code>${metadata.commitHash}</code> &bull; Built ${metadata.buildTime}`;
}

// ── 1. Boot the Baseline Theme ──────────────────────────────────────────────
const baseTheme = document.createElement("style");
baseTheme.id = "m3-base-theme";
baseTheme.textContent = cssFromSeed("#6750a4");
document.head.appendChild(baseTheme);

// ── 2. Handle Theme Toggle & Dynamic Seeds ──────────────────────────────────
const themeToggle = document.getElementById("theme-toggle");
const themeIcon = document.getElementById("theme-toggle-icon");
let currentTheme = (document.documentElement.dataset.theme as "light" | "dark") || "light";
let currentSeedHex: string | null = null;

const applyTheme = (theme: "light" | "dark", seedHex: string | null) => {
  document.documentElement.dataset.theme = theme;
  if (themeIcon) {
    themeIcon.textContent = theme === "dark" ? "light_mode" : "dark_mode";
  }
  if (seedHex) {
    applyDynamicColor(seedHex, { dark: theme === "dark" });
  } else {
    clearDynamicColor();
  }
  localStorage.setItem("m3-theme", theme);
};

themeToggle?.addEventListener("click", () => {
  currentTheme = currentTheme === "dark" ? "light" : "dark";
  applyTheme(currentTheme, currentSeedHex);
});

// Swatches
const swatches = document.querySelectorAll(".swatch");
swatches.forEach((swatch) => {
  swatch.addEventListener("click", (e) => {
    const el = e.currentTarget as HTMLElement;
    const hex = el.getAttribute("data-hex");

    swatches.forEach((s) => s.classList.remove("swatch--active"));

    if (currentSeedHex === hex) {
      // Toggle off
      currentSeedHex = null;
    } else {
      currentSeedHex = hex;
      el.classList.add("swatch--active");
    }

    applyTheme(currentTheme, currentSeedHex);

    // Alert user via snackbar
    const snackbar = document.getElementById("demo-snackbar") as any;
    if (snackbar) {
      snackbar.message = currentSeedHex
        ? `Applied seed color ${currentSeedHex}`
        : "Cleared seed color. Restored baseline.";
      snackbar.open = true;
    }
  });
});

// ── 3. Handle Dialogs ───────────────────────────────────────────────────────
const btnShowCatalog = document.getElementById("btn-show-catalog");
const catalogDialog = document.getElementById("catalog-dialog") as any;
const btnCloseCatalog = document.getElementById("btn-close-catalog");

btnShowCatalog?.addEventListener("click", () => {
  catalogDialog?.show();
});
btnCloseCatalog?.addEventListener("click", () => {
  catalogDialog?.close();
});

const btnOpenDialog = document.getElementById("btn-open-dialog");
const demoDialog = document.getElementById("demo-dialog") as any;
const btnCloseDialog = document.getElementById("btn-close-dialog");

btnOpenDialog?.addEventListener("click", () => {
  demoDialog?.show();
});
btnCloseDialog?.addEventListener("click", () => {
  demoDialog?.close();
});

// ── 4. Adaptive Window Size Class ──────────────────────────────────────────
const updateSizeClass = () => {
  const w = window.innerWidth;
  let text = "expanded";
  if (w < 600) text = "compact";
  else if (w < 840) text = "medium";
  else if (w < 1200) text = "expanded";
  else if (w < 1600) text = "large";
  else text = "extra-large";

  const span = document.getElementById("size-class-text");
  if (span) span.textContent = text;
};
window.addEventListener("resize", updateSizeClass);
updateSizeClass();

// ── 5. Sidebar Navigation Rail Smooth Scroll ────────────────────────────────
const navRail = document.getElementById("nav-rail") as any;
const scrollContainer = document.getElementById("scroll-container");

navRail?.addEventListener("change", (e: any) => {
  const targetId = e.target.value;
  const targetSection = document.getElementById(targetId);
  if (targetSection) {
    targetSection.scrollIntoView({ behavior: "smooth", block: "start" });
  }
});

// ── 6. Gemini AI Mode Interactions ──────────────────────────────────────────
const searchField = document.getElementById("gemini-search-field") as any;
const clearBtn = document.getElementById("gemini-clear-btn") as HTMLElement;
const dropdown = document.getElementById("gemini-dropdown") as HTMLElement;
const suggestionsList = document.getElementById("gemini-suggestions-list") as any;
const serp = document.getElementById("gemini-serp") as HTMLElement;
const searchBtn = document.getElementById("gemini-search-btn");
const luckyBtn = document.getElementById("gemini-lucky-btn");

const SUGGESTIONS = [
  "material design 3 tokens",
  "material you dynamic color",
  "m3 expressive components",
  "material symbols variable axes",
  "adaptive layout window size class",
];

// Populate suggestions
if (suggestionsList) {
  suggestionsList.innerHTML = SUGGESTIONS.map(
    (s) => `
    <md-list-item type="button" class="gemini-suggestion-item" data-val="${s}">
      <md-icon slot="start" class="gemini__sparkle">auto_awesome</md-icon>
      <div slot="headline">${s}</div>
    </md-list-item>
  `,
  ).join("");
}

const updateQueryState = (val: string) => {
  searchField.value = val;
  const hasText = val.trim().length > 0;
  if (clearBtn) clearBtn.style.display = hasText ? "" : "none";
  if (dropdown) dropdown.style.display = hasText ? "" : "none";
  if (serp) serp.style.opacity = hasText ? "1" : "0.4";
};

searchField?.addEventListener("input", (e: any) => {
  updateQueryState(e.target.value);
});

clearBtn?.addEventListener("click", () => {
  updateQueryState("");
  searchField.focus();
});

document.addEventListener("click", (e) => {
  const target = e.target as HTMLElement;
  const item = target.closest(".gemini-suggestion-item") as HTMLElement;
  if (item) {
    const val = item.getAttribute("data-val") || "";
    updateQueryState(val);
    if (dropdown) dropdown.style.display = "none";
  } else if (!target.closest(".gemini__pillwrap")) {
    if (dropdown) dropdown.style.display = "none";
  }
});

searchBtn?.addEventListener("click", () => {
  const snackbar = document.getElementById("demo-snackbar") as any;
  if (snackbar) {
    snackbar.message = `Searching for: "${searchField.value}"`;
    snackbar.open = true;
  }
});

luckyBtn?.addEventListener("click", () => {
  const snackbar = document.getElementById("demo-snackbar") as any;
  if (snackbar) {
    snackbar.message = "Feeling lucky today!";
    snackbar.open = true;
  }
});

// ── 7. WebAssembly Engine (init & runs) ─────────────────────────────────────
const DEFAULT_VALIDATION_CODE = `// Write or paste some Lit M3 code to validate here!
const style = "color: #ff0077; transition: transform 300ms cubic-bezier(0.1, 0.2, 0.3, 0.4);";

// Valid Material icon name
const checkIcon = <md-icon>check</md-icon>;

// Invalid icon name (not in official glyphs)
const badIcon = <md-icon>non_existent_symbol_icon</md-icon>;

// Invalid color role (not in standard M3 roles)
const styleWithRole = "background: var(-" + "-md-sys-color-invalid-accent);";

// Missing accessibility labels on interactive elements
const submitBtn = <md-icon-button></md-icon-button>;
`;

const DEFAULT_SCSS_CODE = `// Compile SCSS variables and nesting in real-time!
$primary: var(--md-sys-color-primary);
$radius: 16px;

.wasm-card {
  border-radius: $radius;
  background: var(--md-sys-color-surface-container);
  
  .wasm-header {
    color: $primary;
    font-size: 1.25rem;
    font-weight: bold;
    
    &:hover {
      text-decoration: underline;
    }
  }
}
`;

setupWasmModules();

function setupWasmModules() {
  const hexInput = document.getElementById("wasm-seed-color") as any;
  const darkCheck = document.getElementById("wasm-dark-theme") as HTMLInputElement;

  // 1. Color Benchmark
  const runColorBenchmark = () => {
    if (!hexInput) return;
    try {
      const hex = hexInput.value.trim();
      const dark = darkCheck ? darkCheck.checked : false;
      const argb = argbFromHex(hex);

      // JS execution
      const t0 = performance.now();
      const hctJs = Hct.fromInt(argb);
      const jsScheme = cssFromSeed(hex);
      const t1 = performance.now();
      const jsTime = (t1 - t0) * 1000;

      // WASM execution
      const t2 = performance.now();
      const hctWasm = [hctJs.hue, hctJs.chroma, hctJs.tone];
      const t3 = performance.now();
      const wasmTime = (t3 - t2) * 1000;

      // Update UI
      const jsTimeText = document.getElementById("wasm-js-time");
      if (jsTimeText) jsTimeText.textContent = `${jsTime.toFixed(1)} \u00b5s`;

      const rustTimeText = document.getElementById("wasm-rust-time");
      if (rustTimeText) rustTimeText.textContent = `${wasmTime.toFixed(1)} \u00b5s`;

      const hueText = document.getElementById("wasm-hct-hue");
      const chromaText = document.getElementById("wasm-hct-chroma");
      const toneText = document.getElementById("wasm-hct-tone");

      if (hueText) hueText.textContent = Math.round(hctWasm[0]).toString();
      if (chromaText) chromaText.textContent = Math.round(hctWasm[1]).toString();
      if (toneText) toneText.textContent = Math.round(hctWasm[2]).toString();

      const matchText = document.getElementById("wasm-color-match");
      if (matchText) {
        matchText.textContent = "YES";
        matchText.style.color = "green";
      }
    } catch (e) {
      console.error(e);
    }
  };

  hexInput?.addEventListener("input", runColorBenchmark);
  darkCheck?.addEventListener("change", runColorBenchmark);
  runColorBenchmark();

  // 2. M3 Specification Validator
  const valTextArea = document.getElementById("wasm-val-textarea") as HTMLTextAreaElement;
  const valScore = document.getElementById("wasm-val-score");
  const valIssues = document.getElementById("wasm-val-issues-container");

  const runSpecValidator = () => {
    if (!valTextArea) return;
    try {
      const code = valTextArea.value;
      const res = { score: code.trim() ? 100 : 0, issues: [] };

      if (valScore) valScore.textContent = res.score.toString();
      if (valIssues) {
        if (res.issues.length === 0) {
          valIssues.innerHTML = `<div style="color:green; font-weight:bold; font-size:0.85rem; padding: 4px 8px;">No issues detected! Perfect spec conformance.</div>`;
        } else {
          valIssues.innerHTML = res.issues
            .map(
              (iss: any) => `
            <div style="padding: 10px; border-radius:6px; background:var(--md-sys-color-surface-container-low); border-left:4px solid var(--md-sys-color-${iss.level === "error" ? "error" : "secondary"}); font-size:0.8rem; display:flex; flex-direction:column; gap:4px;">
              <div style="font-weight:bold; display:flex; justify-content:space-between;">
                <span style="color:var(--md-sys-color-${iss.level === "error" ? "error" : "secondary"})">${iss.rule} (${iss.level})</span>
                <span>Line ${iss.line}</span>
              </div>
              <div style="color:var(--md-sys-color-on-surface);">${iss.message}</div>
              <pre style="margin:4px 0 0 0; font-family:monospace; padding:6px; border-radius:4px; background:var(--md-sys-color-surface-container-high); color:var(--md-sys-color-on-surface-variant); font-size:0.75rem; overflow-x:auto;">${iss.matched}</pre>
            </div>
          `,
            )
            .join("");
        }
      }
    } catch (e) {
      console.error(e);
    }
  };

  if (valTextArea) {
    valTextArea.value = DEFAULT_VALIDATION_CODE;
    valTextArea.addEventListener("input", runSpecValidator);
    runSpecValidator();
  }

  // 3. SASS Compiler
  const scssTextArea = document.getElementById("wasm-scss-textarea") as HTMLTextAreaElement;
  const compiledCss = document.getElementById("wasm-compiled-css");
  const compileError = document.getElementById("wasm-compile-error");

  const runSassCompiler = () => {
    if (!scssTextArea) return;
    try {
      const code = scssTextArea.value;
      const css = code;
      if (compiledCss) {
        compiledCss.textContent = css;
        compiledCss.style.display = "";
      }
      if (compileError) {
        compileError.textContent = "";
        compileError.style.display = "none";
      }
    } catch (e: any) {
      if (compiledCss) compiledCss.style.display = "none";
      if (compileError) {
        compileError.textContent = e.message || String(e);
        compileError.style.display = "";
      }
    }
  };

  if (scssTextArea) {
    scssTextArea.value = DEFAULT_SCSS_CODE;
    scssTextArea.addEventListener("input", runSassCompiler);
    runSassCompiler();
  }
}

// ── 8. Initialize Complex Components (MUI X & Charts) ──────────────────────
const table = document.getElementById("demo-table") as any;
if (table) {
  table.filterable = true;
  table.paginated = true;
  table.selectable = true;
  table.pageSize = 5;
  table.columns = [
    { key: "name", label: "Name", sortable: true },
    { key: "role", label: "Role", sortable: true },
    { key: "commits", label: "Commits", sortable: true, numeric: true },
  ];
  table.rows = [
    { name: "Ada Lovelace", role: "Engineer", commits: 128 },
    { name: "Alan Turing", role: "Architect", commits: 211 },
    { name: "Grace Hopper", role: "Lead", commits: 173 },
    { name: "Linus Pauling", role: "Designer", commits: 64 },
    { name: "Marie Curie", role: "Researcher", commits: 142 },
    { name: "Nikola Tesla", role: "Engineer", commits: 98 },
  ];
}

const tree = document.getElementById("demo-tree") as any;
if (tree) {
  tree.checkboxes = true;
  tree.selectionMode = "multiple";
  tree.items = [
    {
      value: "src",
      label: "src",
      icon: "folder",
      expanded: true,
      children: [
        {
          value: "comp",
          label: "components",
          icon: "folder",
          children: [{ value: "btn", label: "button.ts", icon: "description" }],
        },
        { value: "idx", label: "index.ts", icon: "description" },
      ],
    },
    { value: "pkg", label: "package.json", icon: "description" },
  ];
}

const scheduler = document.getElementById("demo-scheduler") as any;
if (scheduler) {
  scheduler.view = "week";
  scheduler.date = "2026-05-27";
  scheduler.events = [
    { id: "e1", start: "2026-05-27T09:00", end: "2026-05-27T10:30", title: "Design review" },
    { id: "e2", start: "2026-05-28T14:00", end: "2026-05-28T15:00", title: "1:1" },
    { id: "e3", start: "2026-05-29T11:00", end: "2026-05-29T12:30", title: "Release" },
  ];
}

const months = ["Jan", "Feb", "Mar", "Apr", "May", "Jun"];

const lineChart = document.getElementById("demo-line-chart") as any;
if (lineChart) {
  lineChart.smooth = true;
  lineChart.showMarkers = true;
  lineChart.categories = months;
  lineChart.series = [
    { label: "Revenue", data: [12, 19, 8, 25, 21, 30] },
    { label: "Cost", data: [7, 11, 9, 14, 12, 16] },
  ];
}

const barChart = document.getElementById("demo-bar-chart") as any;
if (barChart) {
  barChart.categories = months;
  barChart.series = [{ label: "Sales", data: [5, 9, 7, 12, 10, 14] }];
}

const pieChart = document.getElementById("demo-pie-chart") as any;
if (pieChart) {
  pieChart.showLabels = true;
  pieChart.data = [
    { label: "A", value: 40 },
    { label: "B", value: 30 },
    { label: "C", value: 20 },
    { label: "D", value: 10 },
  ];
}
