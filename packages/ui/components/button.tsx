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
 *   destructive → <md-filled-button>     + .btn-destructive  (error palette)
 *   link        → <md-text-button>       + href passthrough
 *
 * Sizes use CSS custom properties keyed off `--md-sys-typescale-label-*` so
 * any host theme that overrides those vars automatically resizes the button.
 *
 * This file MUST be loaded in a DOM-capable environment (React 19 + a browser
 * or jsdom/happy-dom). The custom elements are registered as side-effects of
 * the `import '@material/web/...'` statements below.
 */

import * as React from "react";

// Side-effect imports — register the custom elements with `customElements`.
// Path comes from @material/web v2.x (validated via context7 against
// /material-components/material-web).
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

export interface ButtonProps
	extends Omit<React.HTMLAttributes<HTMLElement>, "type"> {
	/** Visual variant; defaults to `"default"` (filled). */
	variant?: ButtonVariant;
	/** Height-affecting size class; defaults to `"md"`. */
	size?: ButtonSize;
	/** Native `disabled` attribute on the underlying button. */
	disabled?: boolean;
	/** Native `type` attribute (defaults to `"button"` for safety). */
	type?: ButtonType;
	/** Associate the button with a form element by id. */
	form?: string;
	/** Submitted name for `type="submit"`. */
	name?: string;
	/** Submitted value for `type="submit"`. */
	value?: string | number;
	/** If supplied, renders an `<a>`-like button (uses md-* `href` attribute). */
	href?: string;
	/** Anchor target when `href` is set. */
	target?: string;
	/** Show a trailing icon slot (CSS-only — caller passes <md-icon> children). */
	trailingIcon?: boolean;
	/** Click handler. */
	onClick?: React.MouseEventHandler<HTMLElement>;
	/** Children — typically the button label and an optional `<md-icon slot="icon">`. */
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
		disabled?: boolean;
		"soft-disabled"?: boolean;
		href?: string;
		target?: string;
		type?: ButtonType;
		form?: string;
		name?: string;
		value?: string | number;
		"trailing-icon"?: boolean;
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
// Component
// ---------------------------------------------------------------------------

export const Button = React.forwardRef<HTMLElement, ButtonProps>(function Button(
	props,
	ref,
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
		children,
		...rest
	} = props;

	const Tag = VARIANT_TAG[variant];
	const sizeClass = SIZE_CLASS[size];
	const variantClass = `aph-btn-${variant}`;
	const finalClass = ["aph-btn", sizeClass, variantClass, className]
		.filter((s): s is string => Boolean(s))
		.join(" ");

	const linkProps =
		variant === "link" || href ? { href, target } : {};

	return (
		<Tag
			ref={ref}
			class={finalClass}
			disabled={disabled || undefined}
			type={type}
			form={form}
			name={name}
			value={value}
			trailing-icon={trailingIcon || undefined}
			onClick={onClick}
			{...linkProps}
			{...rest}
		>
			{children}
		</Tag>
	);
});

Button.displayName = "Button";

export default Button;
