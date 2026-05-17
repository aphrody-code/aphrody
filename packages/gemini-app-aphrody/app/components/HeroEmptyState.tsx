// SPDX-License-Identifier: Apache-2.0
"use client";

import { useEffect, useRef, type JSX } from "react";
import { Sparkle } from "./Sparkle";
import { SuggestionChip } from "./SuggestionChip";
import {
  mountSpectrumShift,
  type SpectrumShiftHandle,
} from "../../lib/webgpu-gradient";

const SUGGESTIONS: ReadonlyArray<{
  text: string;
  glyph: string;
  featured?: boolean;
}> = [
  {
    text: "Walk me through the M3 token cascade I just shipped",
    glyph: "\u{1F4A1}",
    featured: true,
  },
  {
    text: "Generate a Rust unit test for the spectrum-shift gradient parser",
    glyph: "<>",
  },
  {
    text: "Sketch a launch tweet for the bxc mass-scrape orchestrator",
    glyph: "\u{1F4DD}",
  },
  {
    text: "Compare shadcn Button vs MWC3 md-filled-button accessibility",
    glyph: "\u{267F}",
  },
];

interface HeroEmptyStateProps {
  onSuggest(text: string): void;
}

export function HeroEmptyState({ onSuggest }: HeroEmptyStateProps): JSX.Element {
  const canvasRef = useRef<HTMLCanvasElement | null>(null);

  useEffect(() => {
    const canvas = canvasRef.current;
    if (!canvas) return;
    let handle: SpectrumShiftHandle | null = null;
    let cancelled = false;
    mountSpectrumShift(canvas)
      .then((h) => {
        if (cancelled) {
          h?.stop();
          return;
        }
        handle = h;
      })
      .catch(() => {
        // Fallback: CSS gradient already painted by the parent; nothing to do.
      });
    return () => {
      cancelled = true;
      handle?.stop();
    };
  }, []);

  return (
    <div className="hero">
      <canvas
        ref={canvasRef}
        className="hero__webgpu-canvas"
        aria-hidden="true"
      />
      <Sparkle size={64} spin />
      <h1 className="hero__greeting">Hello, Aphrody</h1>
      <p className="hero__sub">How can I help you today?</p>
      <div className="suggestions">
        {SUGGESTIONS.map((s) => (
          <SuggestionChip
            key={s.text}
            text={s.text}
            glyph={s.glyph}
            featured={s.featured ?? false}
            onSelect={onSuggest}
          />
        ))}
      </div>
    </div>
  );
}
