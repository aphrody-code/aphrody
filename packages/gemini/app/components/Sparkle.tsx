// SPDX-License-Identifier: Apache-2.0
import type { JSX } from "react";

/**
 * Pure-JSX SVG sparkle — no `dangerouslySetInnerHTML`. Mirrors the path
 * in crates/aphrody-wasm/examples/gemini-clone-pixel-perfect.html lines
 * 606-616 with the four-color sparkle gradient anchored against the four
 * Google-logo lineage dots.
 */
export function Sparkle({
  size = 64,
  className,
  spin = false,
}: {
  size?: number;
  className?: string;
  spin?: boolean;
}): JSX.Element {
  const classes = [
    "gemini-sparkle",
    spin ? "gemini-sparkle--animated" : null,
    className ?? null,
  ]
    .filter((s): s is string => Boolean(s))
    .join(" ");
  return (
    <span
      className={classes}
      style={{ width: size, height: size, display: "inline-block" }}
      aria-hidden="true"
    >
      <svg
        viewBox="0 0 24 24"
        width={size}
        height={size}
        fill="url(#aphrody-sparkle-grad)"
      >
        <defs>
          <linearGradient id="aphrody-sparkle-grad" x1="0" y1="0" x2="1" y2="1">
            <stop offset="0%" stopColor="#4285F4" />
            <stop offset="33%" stopColor="#EA4335" />
            <stop offset="66%" stopColor="#FBBC04" />
            <stop offset="100%" stopColor="#34A853" />
          </linearGradient>
        </defs>
        <path d="M12 2.5 C12 7.5 14.5 10 19.5 10 C14.5 10 12 12.5 12 17.5 C12 12.5 9.5 10 4.5 10 C9.5 10 12 7.5 12 2.5 Z" />
      </svg>
    </span>
  );
}
