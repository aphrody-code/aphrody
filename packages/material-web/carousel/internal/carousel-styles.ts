/**
 * @license
 * Copyright 2026 Google LLC
 * SPDX-License-Identifier: Apache-2.0
 */

import { css } from "lit";

/**
 * Material 3 carousel styles. Layout is driven by CSS scroll-snap. Consumes the
 * `--md-sys-*` design tokens directly. Component-level `--md-carousel-*` custom
 * properties allow per-instance overrides.
 */
export const styles = css`
  :host {
    --_gap: var(--md-carousel-gap, 8px);

    display: block;
    box-sizing: border-box;
    color: var(--md-sys-color-on-surface, #1d1b20);
  }

  .carousel {
    position: relative;
    display: flex;
    align-items: center;
  }

  .scroller {
    display: flex;
    flex-direction: row;
    flex: 1;
    gap: var(--_gap);
    overflow-x: auto;
    scroll-snap-type: var(--md-carousel-scroll-snap-type, x mandatory);
    scroll-behavior: smooth;
    scrollbar-width: none;
    -webkit-overflow-scrolling: touch;
    scroll-padding-inline: var(--md-carousel-scroll-padding-inline, 0px);
    scroll-padding-left: var(
      --md-carousel-scroll-padding-left,
      var(--md-carousel-scroll-padding-inline, 0px)
    );
    scroll-padding-right: var(
      --md-carousel-scroll-padding-right,
      var(--md-carousel-scroll-padding-inline, 0px)
    );
    overscroll-behavior-x: var(--md-carousel-overscroll-behavior, contain);
  }

  .scroller::-webkit-scrollbar {
    display: none;
  }

  /* Items snap to the configured edge. */
  ::slotted(*) {
    scroll-snap-align: var(--md-carousel-item-scroll-snap-align, start);
    scroll-snap-stop: var(--md-carousel-item-scroll-snap-stop, always);
    scroll-margin-inline: var(--md-carousel-item-scroll-margin-inline, 0px);
    flex: 0 0 auto;
  }

  /* Hero: the first/large item dominates the viewport. */
  :host([layout="hero"]) ::slotted([size="large"]),
  :host([layout="hero"]) ::slotted(:not([size])) {
    min-width: 60%;
  }

  .arrow {
    appearance: none;
    border: none;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    width: 40px;
    height: 40px;
    flex: 0 0 auto;
    padding: 8px;
    border-radius: var(--md-sys-shape-corner-full, 9999px);
    background: var(--md-sys-color-surface-container-high, #ece6f0);
    color: var(--md-sys-color-on-surface, #1d1b20);
    cursor: pointer;
    /* M3 elevation level 1 */
    box-shadow: var(
      --md-sys-elevation-level1,
      0 1px 2px 0 rgba(0, 0, 0, 0.3),
      0 1px 3px 1px rgba(0, 0, 0, 0.15)
    );
  }

  .arrow.prev {
    margin-inline-end: 8px;
  }

  .arrow.next {
    margin-inline-start: 8px;
  }

  .arrow:hover {
    background: color-mix(
      in srgb,
      currentColor 8%,
      var(--md-sys-color-surface-container-high, #ece6f0)
    );
  }

  .arrow:focus-visible {
    outline: 2px solid var(--md-sys-color-primary, #6750a4);
    outline-offset: 2px;
  }

  .arrow svg {
    width: 24px;
    height: 24px;
    fill: currentColor;
  }

  @media (forced-colors: active) {
    .arrow {
      outline: 1px solid CanvasText;
    }
  }
`;
