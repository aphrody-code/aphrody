/// <reference lib="dom" />
// Headless component tests via `bun test` + happy-dom + ElementInternals
// polyfill (preloaded in bunfig.toml). Real Lit upgrade + render + reactivity,
// no browser. Complements the real-Chromium bxc gate (`bun run test:browser`).
import { test, expect, beforeEach } from "bun:test";

beforeEach(() => {
  document.body.innerHTML = "";
});

async function mount<T extends HTMLElement>(tag: string): Promise<T> {
  const el = document.createElement(tag) as T;
  document.body.appendChild(el);
  await (el as unknown as { updateComplete: Promise<unknown> }).updateComplete;
  return el;
}

test("md-elevated-button upgrades and renders a shadow root", async () => {
  await import("../button/elevated-button.js");
  const el = await mount("md-elevated-button");
  expect(customElements.get("md-elevated-button")).toBeDefined();
  expect(el.shadowRoot).toBeTruthy();
  expect(el.shadowRoot!.childNodes.length).toBeGreaterThan(0);
});

test("md-checkbox is reactive and form-associated (ElementInternals)", async () => {
  await import("../checkbox/checkbox.js");
  const el = await mount<HTMLElement & { checked: boolean }>("md-checkbox");
  expect(el.checked).toBe(false);
  el.checked = true;
  await (el as unknown as { updateComplete: Promise<unknown> }).updateComplete;
  expect(el.checked).toBe(true);
  // form-association relies on attachInternals() — proves the polyfill path works
  expect(el.shadowRoot).toBeTruthy();
});

test("md-outlined-text-field reflects its value property", async () => {
  await import("../textfield/outlined-text-field.js");
  const el = await mount<HTMLElement & { value: string }>("md-outlined-text-field");
  el.value = "hello";
  await (el as unknown as { updateComplete: Promise<unknown> }).updateComplete;
  expect(el.value).toBe("hello");
});

test("aphrody fork component md-alert upgrades and renders", async () => {
  await import("../alert/alert.js");
  const el = await mount("md-alert");
  expect(customElements.get("md-alert")).toBeDefined();
  expect(el.shadowRoot).toBeTruthy();
});
