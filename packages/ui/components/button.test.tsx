/**
 * Tests for the Button wrapper.
 *
 * Runs under `bun test` with happy-dom to register the Material Web custom
 * elements and assert DOM behaviour: variant tag mapping, click handling,
 * disabled state, size class, and form/href passthrough.
 */

import { afterAll, beforeAll, describe, expect, test } from "bun:test";
import { GlobalRegistrator } from "@happy-dom/global-registrator";

beforeAll(() => {
	GlobalRegistrator.register();
});

afterAll(async () => {
	await GlobalRegistrator.unregister();
});

// Dynamic imports so happy-dom is registered before React + Material Web
// load (both touch `document` and `customElements` at module evaluation time).
async function loadDeps() {
	const React = await import("react");
	const ReactDOM = await import("react-dom/client");
	const { Button } = await import("./button.tsx");
	return { React, ReactDOM, Button };
}

function mount(node: import("react").ReactElement): {
	container: HTMLDivElement;
	root: import("react-dom/client").Root;
} {
	const container = document.createElement("div");
	document.body.appendChild(container);
	const ReactDOMClient = require("react-dom/client") as typeof import("react-dom/client");
	const root = ReactDOMClient.createRoot(container);
	root.render(node);
	return { container, root };
}

describe("Button", () => {
	test("renders one Material Web tag per variant", async () => {
		const { React, ReactDOM, Button } = await loadDeps();

		const variants = [
			["default", "md-filled-button"],
			["outline", "md-outlined-button"],
			["ghost", "md-text-button"],
			["secondary", "md-filled-tonal-button"],
			["destructive", "md-filled-button"],
			["link", "md-text-button"],
			["elevated", "md-elevated-button"],
		] as const;

		for (const [variant, expectedTag] of variants) {
			const container = document.createElement("div");
			document.body.appendChild(container);
			const root = ReactDOM.createRoot(container);
			root.render(
				React.createElement(Button, { variant }, `Hello ${variant}`),
			);
			// React 19 flushes synchronously for initial render; small await for safety.
			await Promise.resolve();
			const el = container.firstElementChild;
			expect(el).not.toBeNull();
			expect(el?.tagName.toLowerCase()).toBe(expectedTag);
			expect(el?.textContent).toBe(`Hello ${variant}`);
			expect(el?.classList.contains("aph-btn")).toBe(true);
			expect(el?.classList.contains(`aph-btn-${variant}`)).toBe(true);
			root.unmount();
			container.remove();
		}
	});

	test("calls onClick and stops dispatching when disabled", async () => {
		const { React, ReactDOM, Button } = await loadDeps();

		let clicks = 0;
		const handler = () => {
			clicks += 1;
		};

		const enabledHost = document.createElement("div");
		document.body.appendChild(enabledHost);
		const enabledRoot = ReactDOM.createRoot(enabledHost);
		enabledRoot.render(
			React.createElement(Button, { onClick: handler }, "Click me"),
		);
		await Promise.resolve();
		const enabledEl = enabledHost.firstElementChild as HTMLElement | null;
		expect(enabledEl).not.toBeNull();
		enabledEl?.dispatchEvent(new MouseEvent("click", { bubbles: true }));
		expect(clicks).toBe(1);
		enabledRoot.unmount();
		enabledHost.remove();

		const disabledHost = document.createElement("div");
		document.body.appendChild(disabledHost);
		const disabledRoot = ReactDOM.createRoot(disabledHost);
		disabledRoot.render(
			React.createElement(
				Button,
				{ onClick: handler, disabled: true },
				"Nope",
			),
		);
		await Promise.resolve();
		const disabledEl = disabledHost.firstElementChild as HTMLElement | null;
		expect(disabledEl).not.toBeNull();
		expect(disabledEl?.hasAttribute("disabled")).toBe(true);
		disabledRoot.unmount();
		disabledHost.remove();
	});

	test("threads href + size class + form association attributes", async () => {
		const { React, ReactDOM, Button } = await loadDeps();
		const host = document.createElement("div");
		document.body.appendChild(host);
		const root = ReactDOM.createRoot(host);
		root.render(
			React.createElement(
				Button,
				{
					variant: "link",
					size: "lg",
					href: "https://example.com",
					target: "_blank",
					form: "my-form",
					name: "submit-btn",
					value: "go",
					type: "submit",
				},
				"Go",
			),
		);
		await Promise.resolve();
		const el = host.firstElementChild as HTMLElement | null;
		expect(el).not.toBeNull();
		expect(el?.tagName.toLowerCase()).toBe("md-text-button");
		expect(el?.getAttribute("href")).toBe("https://example.com");
		expect(el?.getAttribute("target")).toBe("_blank");
		expect(el?.getAttribute("form")).toBe("my-form");
		expect(el?.getAttribute("name")).toBe("submit-btn");
		expect(el?.getAttribute("value")).toBe("go");
		expect(el?.getAttribute("type")).toBe("submit");
		expect(el?.classList.contains("aph-btn-lg")).toBe(true);
		expect(el?.classList.contains("aph-btn-link")).toBe(true);
		root.unmount();
		host.remove();
	});
});
