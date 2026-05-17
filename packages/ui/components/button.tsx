/**
 * Button — POC wrapper around Material Web 3 button family.
 * ----------------------------------------------------------
 *
 * Bridges the shadcn variant vocabulary (default, outline, ghost, secondary,
 * destructive, link) to the five Material Web button components:
 *
 *   default     → <md-filled-button>
 *   outline     → <md-outlined-button>
 *   ghost       → <md-text-button>
 *   secondary   → <md-filled-tonal-button>
 *   destructive → <md-filled-button>     + .aph-btn-destructive  (error palette)
 *   link        → <md-text-button>       + href passthrough
 *   elevated    → <md-elevated-button>
 *
 * React 19 has good custom-element support but it tries to assign certain
 * "known" HTML props (`disabled`, `value`, `form`, `name`, `type`, `href`,
 * `target`) as DOM properties. Material Web exposes some of those as Lit
 * reactive properties (read-only via the public surface) and uses attribute
 * reflection to coordinate state. To avoid both directions of breakage we:
 *
 *   1. Render the custom element with NO React-handled attributes.
 *   2. Use a ref + layoutEffect to setAttribute() (or removeAttribute()) for
 *      every prop, exactly as the underlying HTMLElement would expect.
 *   3. Attach the `click` listener imperatively to bypass React's synthetic
 *      event normalisation (which would not bubble custom events properly).
 */

import * as React from "react";

// Side-effect imports — register the custom elements with `customElements`.
import "@material/web/button/filled-button.js";
import "@material/web/button/outlined-button.js";
import "@material/web/button/text-button.js";
import "@material/web/button/filled-tonal-button.js";
import "@material/web/button/elevated-button.js";

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

export type ButtonVariant =
	| "default"
	| "outline"
	| "ghost"
	| "secondary"
	| "destructive"
	| "link"
	| "elevated";

export type ButtonSize = "xs" | "sm" | "md" | "lg";
export type ButtonType = "button" | "submit" | "reset";

export interface ButtonProps {
	variant?: ButtonVariant;
	size?: ButtonSize;
	disabled?: boolean;
	type?: ButtonType;
	form?: string;
	name?: string;
	value?: string | number;
	href?: string;
	target?: string;
	trailingIcon?: boolean;
	onClick?: (ev: MouseEvent) => void;
	className?: string;
	id?: string;
	style?: React.CSSProperties;
	children?: React.ReactNode;
}

// ---------------------------------------------------------------------------
// Variant resolution
// ---------------------------------------------------------------------------

type MdTag =
	| "md-filled-button"
	| "md-outlined-button"
	| "md-text-button"
	| "md-filled-tonal-button"
	| "md-elevated-button";

const VARIANT_TAG: Record<ButtonVariant, MdTag> = {
	default: "md-filled-button",
	outline: "md-outlined-button",
	ghost: "md-text-button",
	secondary: "md-filled-tonal-button",
	destructive: "md-filled-button",
	link: "md-text-button",
	elevated: "md-elevated-button",
};

const SIZE_CLASS: Record<ButtonSize, string> = {
	xs: "aph-btn-xs",
	sm: "aph-btn-sm",
	md: "aph-btn-md",
	lg: "aph-btn-lg",
};

// ---------------------------------------------------------------------------
// JSX intrinsic element declarations (Material Web custom elements)
// ---------------------------------------------------------------------------

type MdButtonAttrs = React.DetailedHTMLProps<
	React.HTMLAttributes<HTMLElement> & {
		ref?: React.Ref<HTMLElement>;
	},
	HTMLElement
>;

declare module "react" {
	namespace JSX {
		interface IntrinsicElements {
			"md-filled-button": MdButtonAttrs;
			"md-outlined-button": MdButtonAttrs;
			"md-text-button": MdButtonAttrs;
			"md-filled-tonal-button": MdButtonAttrs;
			"md-elevated-button": MdButtonAttrs;
			"md-icon": React.DetailedHTMLProps<
				React.HTMLAttributes<HTMLElement> & { slot?: string },
				HTMLElement
			>;
		}
	}
}

// ---------------------------------------------------------------------------
// Attribute reconciliation
// ---------------------------------------------------------------------------

function reconcile(
	el: HTMLElement,
	name: string,
	value: string | number | boolean | undefined,
): void {
	if (value === undefined || value === null || value === false) {
		el.removeAttribute(name);
		return;
	}
	if (value === true) {
		el.setAttribute(name, "");
		return;
	}
	el.setAttribute(name, String(value));
}

// ---------------------------------------------------------------------------
// Component
// ---------------------------------------------------------------------------

export const Button = React.forwardRef<HTMLElement, ButtonProps>(function Button(
	props,
	forwardedRef,
) {
	const {
		variant = "default",
		size = "md",
		disabled = false,
		type = "button",
		form,
		name,
		value,
		href,
		target,
		trailingIcon,
		onClick,
		className,
		id,
		style,
		children,
	} = props;

	const innerRef = React.useRef<HTMLElement | null>(null);

	const setRef = React.useCallback(
		(node: HTMLElement | null) => {
			innerRef.current = node;
			if (typeof forwardedRef === "function") forwardedRef(node);
			else if (forwardedRef) {
				(forwardedRef as React.MutableRefObject<HTMLElement | null>).current = node;
			}
		},
		[forwardedRef],
	);

	// Sync attributes imperatively — React can't be trusted to leave the
	// custom-element property surface alone.
	React.useLayoutEffect(() => {
		const el = innerRef.current;
		if (!el) return;
		reconcile(el, "disabled", disabled);
		reconcile(el, "type", type);
		reconcile(el, "form", form);
		reconcile(el, "name", name);
		reconcile(el, "value", value);
		reconcile(el, "href", href);
		reconcile(el, "target", target);
		reconcile(el, "trailing-icon", trailingIcon);
		if (id !== undefined) reconcile(el, "id", id);
	}, [disabled, type, form, name, value, href, target, trailingIcon, id]);

	// Imperative click binding — survives Lit attribute mutations and
	// preserves access to the native MouseEvent.
	React.useEffect(() => {
		const el = innerRef.current;
		if (!el || !onClick) return;
		const handler = (ev: Event) => onClick(ev as MouseEvent);
		el.addEventListener("click", handler);
		return () => el.removeEventListener("click", handler);
	}, [onClick]);

	const Tag = VARIANT_TAG[variant];
	const variantClass = `aph-btn-${variant}`;
	const finalClass = ["aph-btn", SIZE_CLASS[size], variantClass, className]
		.filter((s): s is string => Boolean(s))
		.join(" ");

	return (
		<Tag ref={setRef} className={finalClass} style={style}>
			{children}
		</Tag>
	);
});

Button.displayName = "Button";

export default Button;
