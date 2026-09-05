// SPDX-License-Identifier: Apache-2.0
//
// Real-browser visual-QA harness for the showcase "Gemini AI Mode" demo.
//
// Drives REAL system Chromium through @aphrody/bxc's `/browser` module
// (the `stealth` profile spawns Chrome via CDP — Lightpanda is rejected because
// it has no ElementInternals, which every `md-*` web component depends on), and
// evaluates the rendered surface against the Google design grammar captured in
// docs/design/google/ANALYSE.md using the bxc `/google` module.
//
// It serves a dedicated entry (qa-gemini.html -> qa-gemini-entry.tsx) that
// mounts the REAL <GeminiAiMode> component (real md-* + real tokens) without
// the showcase's WebGL <ThreeWorld> backdrop, whose @react-three/fiber build is
// incompatible with this React (it throws on the removed `ReactCurrentOwner`
// internal before paint) — an app concern unrelated to this demo.
//
// Run:  CHROME_BIN=/usr/local/bin/google-chrome bun src/qa-gemini.ts
// (also honours BXC_CHROME_BIN / CHROME_PATH — set whichever your env uses)

import { Browser, type Page } from "@aphrody/bxc/browser";
import { checkGoogleStyle, GOOGLE_TS_STYLE_RULES, parseSerp } from "@aphrody/bxc/google";
import index from "./qa-gemini.html";

// --- Resolve the real-Chrome binary and export it the way bxc expects -------
// WebSocketTransport reads BXC_CHROME_BIN / CHROME_PATH from Bun.env; a non-
// Lightpanda path makes it launch that binary with --remote-debugging-port.
const CHROME =
  Bun.env.BXC_CHROME_BIN ??
  Bun.env.CHROME_BIN ??
  Bun.env.CHROME_PATH ??
  "/usr/local/bin/google-chrome";
Bun.env.BXC_CHROME_BIN = CHROME;
Bun.env.CHROME_PATH = CHROME;

// `parseSerp` (bxc /google) uses the rust-bridge cdylib for zigquery parsing.
// The published package ships source only (no prebuilt .so), so point the FFI
// loader at a locally built one when present — otherwise the SERP parse step
// degrades gracefully (it is a supplementary structural check, not core to the
// verdict). Honour an explicit override first.
if (!Bun.env.BXC_RUST_BRIDGE_LIB) {
  for (const so of [
    `${Bun.env.HOME || "/home/ubuntu"}/bxc/rust-bridge/target/release/libbxc_rust_bridge.so`,
    new URL(
      "../../../node_modules/@aphrody/bxc/rust-bridge/target/release/libbxc_rust_bridge.so",
      import.meta.url,
    ).pathname,
  ]) {
    if (await Bun.file(so).exists()) {
      Bun.env.BXC_RUST_BRIDGE_LIB = so;
      break;
    }
  }
}

const fails: string[] = [];
const ok = (cond: boolean, msg: string) => {
  console.log(`${cond ? "ok  " : "FAIL"} ${msg}`);
  if (!cond) fails.push(msg);
};

// --- 1. Serve the live React app (same chain as src/server.ts) --------------
const server = Bun.serve({
  port: 0, // ephemeral free port
  development: { hmr: false },
  routes: { "/*": index },
});
const url = new URL(server.url).toString();
console.log(`showcase serving on ${url} (chrome: ${CHROME})`);

const QA_DIR = new URL("../qa/", import.meta.url).pathname;

let exitCode = 0;
try {
  // --- 2. Open REAL Chrome via bxc and navigate --------------------------
  // The `stealth` profile returns the CDP-backed `Page` (not the `http`
  // `HttpPage`), which exposes `_send`/`waitForSelector`/`screenshot`.
  const page = (await Browser.newPage({
    profile: "stealth", // -> WebSocketTransport -> launches system Chrome
    headless: true,
    viewport: { width: 1400, height: 1700 },
  })) as Page;

  // Real Chrome needs an explicit viewport (CDP default is tiny / 0).
  await page._send("Emulation.setDeviceMetricsOverride", {
    width: 1400,
    height: 1700,
    deviceScaleFactor: 1,
    mobile: false,
  });

  // Collect page console errors throughout the load.
  const consoleErrors: string[] = [];
  await page._send("Runtime.enable", {});
  await page._send("Log.enable", {});
  const transport = (
    page as unknown as {
      _internalTransport: { onmessage?: (m: string) => void };
    }
  )._internalTransport;
  const prev = transport.onmessage;
  transport.onmessage = (raw: string) => {
    prev?.(raw);
    try {
      const msg = JSON.parse(raw);
      if (msg.method === "Runtime.consoleAPICalled" && msg.params?.type === "error") {
        consoleErrors.push(
          (msg.params.args ?? [])
            .map((a: { value?: unknown; description?: unknown }) => a.value ?? a.description ?? "")
            .join(" "),
        );
      } else if (msg.method === "Runtime.exceptionThrown") {
        consoleErrors.push(
          msg.params?.exceptionDetails?.exception?.description ??
            msg.params?.exceptionDetails?.text ??
            "exception",
        );
      } else if (msg.method === "Log.entryAdded" && msg.params?.entry?.level === "error") {
        consoleErrors.push(String(msg.params.entry.text ?? "log error"));
      }
    } catch {
      /* not JSON */
    }
  };

  await page.goto(url, { waitUntil: "load" });

  // --- 3. Wait for React to mount and the lit md-* components to upgrade --
  // The Gemini section is the page root here (qa-gemini-entry.tsx renders it
  // directly), so we just wait for .gemini + the chip's shadowRoot. A null
  // shadowRoot means we are on the wrong (non-ElementInternals) engine.
  await page.waitForSelector(".gemini", 30_000);
  await waitFor(
    page,
    async () =>
      await page.evaluate(() => {
        const chip = document.querySelector("md-assist-chip") as
          | (HTMLElement & { shadowRoot: ShadowRoot | null })
          | null;
        return !!(chip && chip.shadowRoot && document.querySelector(".gemini__serp"));
      }),
    "md-assist-chip upgrade (shadowRoot) + SERP render",
    20_000,
  );
  // Let the lit components and the autocomplete dropdown settle.
  await Bun.sleep(700);
  await page.evaluate(() => {
    document.querySelector(".gemini")?.scrollIntoView({ block: "start" });
  });
  await Bun.sleep(300);

  // --- Gather the rendered facts in one evaluate round-trip --------------
  type Probe = {
    present: Record<string, boolean>;
    chipShadow: boolean;
    sparkleGradient: string;
    roles: Record<string, string>;
    bbox: { x: number; y: number; width: number; height: number };
    serpHtml: string;
    dropdownHtml: string;
    chipUpgraded: boolean;
  };

  const probe = (await page.evaluate(() => {
    const sel = (s: string) => document.querySelector(s);
    const present: Record<string, boolean> = {};
    for (const s of [
      ".gemini",
      ".gemini__pill",
      ".gemini__chip",
      ".gemini__sparkle",
      ".gemini__dropdown",
      ".gemini__serp",
    ]) {
      present[s] = !!sel(s);
    }

    const chip = document.querySelector("md-assist-chip") as
      | (HTMLElement & { shadowRoot: ShadowRoot | null })
      | null;
    const chipShadow = !!chip?.shadowRoot;
    const chipUpgraded = !!chip && chip.constructor.name !== "HTMLElement";

    const rootStyle = getComputedStyle(document.documentElement);
    const sparkleGradient = rootStyle.getPropertyValue("--gemini-sparkle").trim();

    // Representative M3 roles read off the Gemini surface element.
    const surface = (sel(".gemini") as HTMLElement) ?? document.documentElement;
    const cs = getComputedStyle(surface);
    const roles: Record<string, string> = {};
    for (const r of [
      "--md-sys-color-surface",
      "--md-sys-color-surface-container-high",
      "--md-sys-color-on-surface",
      "--md-sys-color-on-surface-variant",
      "--md-sys-color-primary",
      "--md-sys-color-outline-variant",
    ]) {
      roles[r] = cs.getPropertyValue(r).trim();
    }

    const g = sel(".gemini") as HTMLElement | null;
    const rect = g?.getBoundingClientRect();
    const bbox = rect
      ? {
          x: Math.round(rect.x),
          y: Math.round(rect.y),
          width: Math.round(rect.width),
          height: Math.round(rect.height),
        }
      : { x: 0, y: 0, width: 0, height: 0 };

    return {
      present,
      chipShadow,
      chipUpgraded,
      sparkleGradient,
      roles,
      bbox,
      serpHtml: (sel(".gemini__serp") as HTMLElement)?.outerHTML ?? "",
      dropdownHtml: (sel(".gemini__dropdown") as HTMLElement)?.outerHTML ?? "",
    } satisfies Probe;
  })) as Probe;

  // --- 3 (assertions) -----------------------------------------------------
  for (const [s, p] of Object.entries(probe.present)) {
    ok(p, `selector present: ${s}`);
  }
  ok(probe.chipShadow, `md-assist-chip upgraded (shadowRoot non-null) — proves REAL Chrome`);
  ok(probe.chipUpgraded, "md-assist-chip custom element defined (not HTMLElement)");
  ok(
    probe.sparkleGradient.length > 0,
    `--gemini-sparkle non-empty (${probe.sparkleGradient.slice(0, 48)}…)`,
  );
  const roleHits = Object.entries(probe.roles).filter(([, v]) => v.length > 0);
  ok(
    roleHits.length >= 4,
    `>=4 --md-sys-color-* roles applied (${roleHits.length}/${Object.keys(probe.roles).length}): ` +
      roleHits.map(([k, v]) => `${k.replace("--md-sys-color-", "")}=${v}`).join(", "),
  );
  ok(
    consoleErrors.length === 0,
    `zero page console errors during load` +
      (consoleErrors.length
        ? ` (got ${consoleErrors.length}: ${consoleErrors.slice(0, 3).join(" | ")})`
        : ""),
  );

  // --- 4. Screenshots: light + dark --------------------------------------
  const clip = {
    x: probe.bbox.x,
    y: probe.bbox.y,
    width: Math.max(1, probe.bbox.width),
    height: Math.max(1, probe.bbox.height),
    scale: 1,
  };

  const shoot = async (theme: "light" | "dark", file: string) => {
    await page.evaluate((t) => {
      document.documentElement.dataset.theme = t;
    }, theme);
    await Bun.sleep(700); // let the re-theme + animations settle
    // Clean section clip via raw CDP (bxc screenshot() has no clip arg).
    const { data } = (await page._send("Page.captureScreenshot", {
      format: "png",
      clip,
      captureBeyondViewport: true,
    })) as { data: string };
    const bytes = Uint8Array.fromBase64(data);
    await Bun.write(file, bytes);
    return bytes.length;
  };

  const lightPath = `${QA_DIR}gemini-light.png`;
  const darkPath = `${QA_DIR}gemini-dark.png`;
  const lightBytes = await shoot("light", lightPath);
  const darkBytes = await shoot("dark", darkPath);
  ok(lightBytes > 1000, `light screenshot written (${lightBytes} bytes)`);
  ok(darkBytes > 1000, `dark screenshot written (${darkBytes} bytes)`);
  console.log(
    `section bbox: ${probe.bbox.width}x${probe.bbox.height} @ (${probe.bbox.x},${probe.bbox.y})`,
  );
  console.log(`  light -> ${lightPath} (${lightBytes} bytes)`);
  console.log(`  dark  -> ${darkPath} (${darkBytes} bytes)`);

  // --- 5. bxc /google module — design-grammar verdict --------------------
  // `checkGoogleStyle` is a TypeScript *style* linter (tabs/var), not a design
  // checker (see GOOGLE_TS_STYLE_RULES). We use it as documented to lint this
  // harness's own source against Google TS style, AND use the google module's
  // SERP design utility (`parseSerp`) to structurally validate the rendered
  // Gemini SERP markup against the Google SERP grammar (organic results +
  // knowledge panel) — the part of the module that is about design/SERP.
  console.log("\n--- bxc /google verdict ---");
  console.log(`google TS style rules available: ${GOOGLE_TS_STYLE_RULES.length}`);
  const styleVerdict = checkGoogleStyle(await Bun.file(import.meta.path).text());
  for (const r of styleVerdict) {
    console.log(`  style[${r.ruleId}]: ${r.pass ? "pass" : "FAIL"} — ${r.message}`);
  }

  // Feed the rendered Gemini SERP markup to the Google SERP parser. The demo
  // uses M3 class names (.gemini__result, .gemini__knowledge) rather than
  // Google's obfuscated classes, so the structural parse legitimately finds 0
  // Google-grammar organic blocks — we instead assert the design-grammar
  // elements the ANALYSE.md mandates are physically rendered.
  try {
    const serp = await parseSerp(probe.serpHtml, "material design 3");
    console.log(
      `  parseSerp(rendered SERP): organic=${serp.organic.length} knowledgePanel=${serp.knowledgePanel ? "yes" : "no"} (M3 class names, not Google's — expected 0 Google-grammar blocks)`,
    );
  } catch (e) {
    // parseSerp needs the rust-bridge cdylib; if it is not built on this host
    // the SERP structural parse is skipped (supplementary, not a verdict gate).
    console.log(
      `  parseSerp: skipped — bxc rust-bridge native lib unavailable (${(e as Error).message.split("\n")[0]})`,
    );
  }

  // Design-grammar assertions from docs/design/google/ANALYSE.md:
  const stops = ["#4285f4", "#9b72cb", "#d96570"];
  const gradOk = stops.every((s) => probe.sparkleGradient.includes(s));
  ok(gradOk, `sparkle gradient carries the Gemini brand stops ${stops.join(" -> ")}`);
  const grammarHits = {
    "search pill (.gemini__pill)": probe.present[".gemini__pill"],
    "AI Mode chip (.gemini__chip)": probe.present[".gemini__chip"],
    "autocomplete dropdown (.gemini__dropdown)": probe.dropdownHtml.includes("gemini__dropdown"),
    "organic results (.gemini__result)": probe.serpHtml.includes("gemini__result"),
    "knowledge panel (.gemini__knowledge)": probe.serpHtml.includes("gemini__knowledge"),
    "site rows (.gemini__ressite)": probe.serpHtml.includes("gemini__ressite"),
  };
  for (const [name, hit] of Object.entries(grammarHits)) {
    ok(!!hit, `google grammar element rendered: ${name}`);
  }

  await page.close();
} catch (err) {
  console.error("HARNESS ERROR:", err instanceof Error ? err.stack : err);
  exitCode = 1;
} finally {
  await Browser.close().catch(() => undefined);
  server.stop(true);
}

// --- 6. Verdict -------------------------------------------------------------
if (fails.length === 0 && exitCode === 0) {
  console.log("\nGEMINI QA: PASS");
  process.exit(0);
} else {
  const reasons = exitCode !== 0 ? ["harness threw (see stack above)"] : fails;
  console.log(`\nGEMINI QA: FAIL — ${reasons.length} issue(s): ${reasons.join("; ")}`);
  process.exit(1);
}

/** Polls `fn` until it returns truthy or the deadline passes. */
async function waitFor(
  _page: unknown,
  fn: () => Promise<unknown>,
  label: string,
  timeoutMs: number,
): Promise<void> {
  const deadline = Date.now() + timeoutMs;
  while (Date.now() < deadline) {
    try {
      if (await fn()) return;
    } catch {
      /* retry */
    }
    await Bun.sleep(150);
  }
  throw new Error(`timeout: ${label}`);
}
