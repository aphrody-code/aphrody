// SPDX-License-Identifier: Apache-2.0
//! Frontend logic for the aphrody console.
//
// Fetches the version, runs commands against the local API (which drives the
// real aphrody CLI in-process), and renders the captured exit code / stdout /
// stderr. Every byte shown comes from the actual command.

// Canonical M3 fusion tokens (the unified aphrody UI foundation); Bun bundles
// this CSS into the page. A stylesheet can only be loaded as a side effect, so
// the unassigned-import lint does not apply here.
// oxlint-disable-next-line import/no-unassigned-import
import "@aphrody-code/theme/tokens.css";

interface CapturedResult {
  code: number;
  stdout: string;
  stderr: string;
}

const versionEl = document.querySelector<HTMLSpanElement>("#version");
const form = document.querySelector<HTMLFormElement>("#form");
const input = document.querySelector<HTMLInputElement>("#input");
const runButton = document.querySelector<HTMLButtonElement>("#run");
const output = document.querySelector<HTMLElement>("#output");
const presets = document.querySelector<HTMLDivElement>("#presets");

/** Split a command line into argument tokens (whitespace-separated). */
function splitArgs(line: string): string[] {
  return line
    .trim()
    .split(/\s+/u)
    .filter((part) => part.length > 0);
}

function render(args: string[], result: CapturedResult): void {
  if (!output) {
    return;
  }
  output.replaceChildren();

  const badge = document.createElement("div");
  badge.className = "status";
  badge.dataset.state = result.code === 0 ? "ok" : "err";
  badge.textContent = `aphrody ${args.join(" ")} -> exit ${result.code}`;
  output.append(badge);

  if (result.stdout.trimEnd().length > 0) {
    const pre = document.createElement("pre");
    pre.textContent = result.stdout.trimEnd();
    output.append(pre);
  }
  if (result.stderr.trim().length > 0) {
    const pre = document.createElement("pre");
    pre.className = "stderr";
    pre.textContent = result.stderr.trimEnd();
    output.append(pre);
  }
}

function renderError(message: string): void {
  if (!output) {
    return;
  }
  output.replaceChildren();
  const badge = document.createElement("div");
  badge.className = "status";
  badge.dataset.state = "err";
  badge.textContent = message;
  output.append(badge);
}

async function run(args: string[]): Promise<void> {
  if (args.length === 0 || !runButton) {
    return;
  }
  runButton.disabled = true;
  try {
    const res = await fetch("/api/run", {
      method: "POST",
      headers: { "content-type": "application/json" },
      body: JSON.stringify({ args }),
    });
    if (!res.ok) {
      const err = (await res.json()) as { error?: string };
      renderError(err.error ?? `request failed (${res.status})`);
      return;
    }
    render(args, (await res.json()) as CapturedResult);
  } catch (cause) {
    renderError(cause instanceof Error ? cause.message : "request failed");
  } finally {
    runButton.disabled = false;
  }
}

form?.addEventListener("submit", (event) => {
  event.preventDefault();
  if (input) {
    void run(splitArgs(input.value));
  }
});

presets?.addEventListener("click", (event) => {
  const target = event.target;
  if (target instanceof HTMLElement && target.dataset.cmd) {
    if (input) {
      input.value = target.dataset.cmd;
    }
    void run(splitArgs(target.dataset.cmd));
  }
});

async function loadVersion(): Promise<void> {
  if (!versionEl) {
    return;
  }
  try {
    const res = await fetch("/api/version");
    const json = (await res.json()) as { version: string };
    versionEl.textContent = `v${json.version}`;
  } catch {
    versionEl.textContent = "offline";
  }
}

void loadVersion();
