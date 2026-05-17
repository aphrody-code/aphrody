// SPDX-License-Identifier: Apache-2.0
"use client";

import type { JSX, ReactNode } from "react";

interface SuggestionChipProps {
  text: string;
  glyph: ReactNode;
  featured?: boolean;
  onSelect(text: string): void;
}

export function SuggestionChip({
  text,
  glyph,
  featured = false,
  onSelect,
}: SuggestionChipProps): JSX.Element {
  const className = featured ? "suggestion suggestion--featured" : "suggestion";
  return (
    <button
      type="button"
      className={className}
      onClick={() => onSelect(text)}
    >
      <span>{text}</span>
      <span className="suggestion__icon" aria-hidden="true">
        {glyph}
      </span>
    </button>
  );
}
