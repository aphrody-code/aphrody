/**
 * Tests for the Button wrapper.
 *
 * Runs under `bun test` with happy-dom registered into the global scope
 * BEFORE React and Material Web are loaded. React 19's scheduler reads
 * `window.event` at flush time, so the DOM globals must be present
 * throughout every microtask the renderer queues.
 */

import { describe, expect, test } from "bun:test";
import { GlobalRegistrator } from "@happy-dom/global-registrator";

// Register happy-dom synchronously at module load. We do NOT unregister at
// the end: doing so removes `window`/`document` which React's scheduler may
// still reach for in a trailing microtask.
GlobalRegistrator.register();

const React = await import("react");
const ReactDOMClient = await import("react-dom/client");
const ReactDOM = await import("react-dom");
const ButtonMod = await import("./button.tsx");
const Button = ButtonMod.Button;

interface MountResult {
	container: HTMLElement;
	root: import("react-dom/client").Root;
	el: HTMLElement;
}

function mountInto(
	container: HTMLElement,
	node: import("react").ReactElement,
): MountResult {
	const root = ReactDOMClient.createRoot(container);
	ReactDOM.flushSync(() => root.render(node));
	const el = container.querySelector(
		"md-filled-button, md-outlined-button, md-text-button, md-filled-tonal-button, md-elevated-button",
	) as HTMLElement | null;
	if (!el) throw new Error("Button rendered no element");
	return { container, root, el };
}

function mountSync(node: import("react").ReactElement): MountResult {
	const container = document.createElement("div");
	document.body.appendChild(container);
	return mountInto(container, node);
}

function unmount(result: MountResult): void {
	ReactDOM.flushSync(() => result.root.unmount());
	result.container.remove();
}

describe("Button", () => {
	test("renders one Material Web tag per variant", () => {
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
			const r = mountSync(
				React.createElement(Button, { variant }, `Hello ${variant}`),
			);
			expect(r.el.tagName.toLowerCase()).toBe(expectedTag);
			expect(r.el.textContent).toBe(`Hello ${variant}`);
			expect(r.el.classList.contains("aph-btn")).toBe(true);
			expect(r.el.classList.contains(`aph-btn-${variant}`)).toBe(true);
			unmount(r);
		}
	});

	test("calls onClick and reflects disabled attribute", () => {
		let clicks = 0;
		const handler = () => {
			clicks += 1;
		};

		const enabled = mountSync(
			React.createElement(Button, { onClick: handler }, "Click me"),
		);
		enabled.el.dispatchEvent(new MouseEvent("click", { bubbles: true }));
		expect(clicks).toBe(1);
		expect(enabled.el.hasAttribute("disabled")).toBe(false);
		unmount(enabled);

		const disabled = mountSync(
			React.createElement(
				Button,
				{ onClick: handler, disabled: true },
				"Nope",
			),
		);
		expect(disabled.el.hasAttribute("disabled")).toBe(true);
		// Click handler is bound but the underlying md-* element honours
		// the disabled attribute and will not fire its own click side
		// effects; we still verify the attribute is reflected.
		unmount(disabled);
	});

	test("threads href + size class + form association attributes", () => {
		// Provide a real form so Material Web's form-associated controller
		// can resolve the `form` attribute without throwing.
		const host = document.createElement("div");
		host.innerHTML = '<form id="my-form"></form><div id="slot"></div>';
		document.body.appendChild(host);
		const slot = host.querySelector("#slot") as HTMLElement;

		const r = mountInto(
			slot,
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
		expect(r.el.tagName.toLowerCase()).toBe("md-text-button");
		expect(r.el.getAttribute("href")).toBe("https://example.com");
		expect(r.el.getAttribute("target")).toBe("_blank");
		expect(r.el.getAttribute("form")).toBe("my-form");
		expect(r.el.getAttribute("name")).toBe("submit-btn");
		expect(r.el.getAttribute("value")).toBe("go");
		expect(r.el.getAttribute("type")).toBe("submit");
		expect(r.el.classList.contains("aph-btn-lg")).toBe(true);
		expect(r.el.classList.contains("aph-btn-link")).toBe(true);
		unmount(r);
		host.remove();
	});
});
