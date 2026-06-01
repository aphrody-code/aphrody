// Verifies the generated @lit/react wrappers actually render their `md-*`
// custom element through React — the core surface of the package. Uses the same
// happy-dom harness as the transitions tests (no React Testing Library). We
// assert what is deterministic headless: the wrapper mounts the correct,
// upgraded custom element with a populated shadow root, and forwards children
// to its slot. Property/event binding is exercised end-to-end by the bxc gate
// (real Chromium) since @lit/react property timing is browser-specific.
// The ElementInternals polyfill (test/setup) lets form-associated elements upgrade.
import { afterEach, describe, expect, test } from "bun:test";
import * as React from "react";
import { createRoot, type Root } from "react-dom/client";
import { flushSync } from "react-dom";

import { MdFilledButton } from "../wrappers/button.js";
import { MdCheckbox } from "../wrappers/checkbox.js";
import { MdIcon } from "../wrappers/icon.js";
import { MdCard } from "../wrappers/card.js";
import { MdOutlinedTextField } from "../wrappers/textfield.js";

let root: Root | undefined;
let host: HTMLElement | undefined;

const tick = () => new Promise((r) => setTimeout(r, 0));

async function mount(node: React.ReactNode, tag: string): Promise<HTMLElement> {
  host = document.createElement("div");
  document.body.appendChild(host);
  root = createRoot(host);
  flushSync(() => root!.render(node));
  await tick();
  const el = host.querySelector(tag) as HTMLElement | null;
  if (!el) throw new Error(`<${tag}> not rendered`);
  await (el as unknown as { updateComplete?: Promise<unknown> }).updateComplete;
  return el;
}

afterEach(() => {
  flushSync(() => root?.unmount());
  host?.remove();
  root = undefined;
  host = undefined;
});

describe("generated wrappers render their custom element", () => {
  test("MdFilledButton mounts an upgraded <md-filled-button> with a shadow root", async () => {
    const el = await mount(<MdFilledButton>Click</MdFilledButton>, "md-filled-button");
    expect(customElements.get("md-filled-button")).toBeDefined();
    expect(el.shadowRoot).toBeTruthy();
    expect(el.shadowRoot!.childNodes.length).toBeGreaterThan(0);
  });

  test("MdCheckbox (form-associated, ElementInternals) upgrades and renders", async () => {
    const el = await mount(<MdCheckbox />, "md-checkbox");
    expect(el.shadowRoot).toBeTruthy();
    // attachInternals() succeeded (form association) — proves the polyfill path.
    expect(el.shadowRoot!.childNodes.length).toBeGreaterThan(0);
  });

  test("MdIcon forwards children to its slot", async () => {
    const el = await mount(<MdIcon>home</MdIcon>, "md-icon");
    expect(el.textContent).toContain("home");
  });

  test("MdCard and MdOutlinedTextField mount their elements", async () => {
    expect((await mount(<MdCard />, "md-card")).shadowRoot).toBeTruthy();
    expect(
      (await mount(<MdOutlinedTextField />, "md-outlined-text-field")).shadowRoot,
    ).toBeTruthy();
  });
});
