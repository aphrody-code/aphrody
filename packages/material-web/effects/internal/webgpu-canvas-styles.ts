/**
 * @license
 * Copyright 2026 Google LLC
 * SPDX-License-Identifier: Apache-2.0
 */

import { css } from "lit";

/**
 * Styles for `<md-webgpu-canvas>`. The `.gpu` canvas is painted by WebGPU; the
 * `.fallback` canvas is painted by animated CSS gradients that approximate each
 * effect when WebGPU is unavailable.
 */
export const styles = css`
  :host {
    display: block;
    position: relative;
    inline-size: 100%;
    block-size: 100%;
    overflow: hidden;
    --_blue: var(--md-sys-color-gemini-brand-blue, #4285f4);
    --_purple: var(--md-sys-color-gemini-brand-purple, #9168c0);
    --_pink: var(--md-sys-color-gemini-brand-pink, #ec4899);
  }

  canvas {
    display: block;
    inline-size: 100%;
    block-size: 100%;
  }

  /* ── CSS fallbacks (no WebGPU) ──────────────────────────────────────── */

  canvas.fallback[data-effect="spectrum"] {
    background: linear-gradient(
      120deg,
      var(--_blue),
      var(--_purple),
      var(--_pink),
      var(--_purple),
      var(--_blue)
    );
    background-size: 300% 300%;
    animation: spectrum-shift 12s ease-in-out infinite;
  }

  canvas.fallback[data-effect="sparkle"] {
    background:
      radial-gradient(
        circle at 50% 50%,
        color-mix(in srgb, var(--_purple) 60%, transparent),
        transparent 60%
      ),
      conic-gradient(from 0deg at 50% 50%, var(--_blue), var(--_purple), var(--_pink), var(--_blue));
    animation: sparkle-spin 18s linear infinite;
  }

  canvas.fallback[data-effect="glimmer"] {
    background: linear-gradient(
      100deg,
      transparent 30%,
      color-mix(in srgb, var(--_blue) 40%, transparent) 50%,
      transparent 70%
    );
    background-size: 200% 100%;
    animation: glimmer-sweep 6s ease-in-out infinite;
  }

  @keyframes spectrum-shift {
    0%,
    100% {
      background-position: 0% 50%;
    }
    50% {
      background-position: 100% 50%;
    }
  }

  @keyframes sparkle-spin {
    to {
      transform: rotate(360deg);
    }
  }

  @keyframes glimmer-sweep {
    0% {
      background-position: 200% 0;
    }
    100% {
      background-position: -200% 0;
    }
  }

  @media (prefers-reduced-motion: reduce) {
    canvas.fallback {
      animation: none;
    }
  }
`;
