/**
 * @license
 * Copyright 2026 Google LLC
 * SPDX-License-Identifier: Apache-2.0
 */

import { css } from "lit";

/**
 * Material 3 carousel-item styles. Consumes the `--md-sys-*` design tokens
 * directly. Component-level `--md-carousel-item-*` custom properties allow
 * per-instance overrides.
 */
export const styles = css`
  :host {
    --_container-shape: var(
      --md-carousel-item-container-shape,
      var(--md-sys-shape-corner-extra-large, 28px)
    );

    display: block;
    box-sizing: border-box;
    height: 100%;
    transition:
      transform 0.3s cubic-bezier(0.2, 0, 0, 1),
      opacity 0.3s cubic-bezier(0.2, 0, 0, 1);
  }

  @supports (container-type: scroll-state) {
    :host {
      container-type: scroll-state;
    }
  }

  :host([active]) {
    transform: scale(1.03);
    z-index: 2;
  }

  :host([active]) .item {
    outline: 2px solid var(--md-sys-color-primary, #6750a4);
    outline-offset: 2px;
    box-shadow: 0 8px 16px rgba(0, 0, 0, 0.25);
  }

  @supports (container-type: scroll-state) {
    @container scroll-state(snapped: x) {
      :host {
        transform: scale(1.03);
        z-index: 2;
      }
      .item {
        outline: 2px solid var(--md-sys-color-primary, #6750a4);
        outline-offset: 2px;
        box-shadow: 0 8px 16px rgba(0, 0, 0, 0.25);
      }
    }
  }

  @media (prefers-reduced-motion: reduce) {
    :host {
      transition: none !important;
      transform: none !important;
    }
    .item {
      outline: none !important;
      box-shadow: none !important;
      transition: none !important;
    }
  }

  /* Decreasing widths for the multi-browse layout. */
  :host([size="large"]) {
    width: 320px;
  }

  :host([size="medium"]) {
    width: 200px;
  }

  :host([size="small"]) {
    width: 120px;
  }

  .item {
    box-sizing: border-box;
    width: 100%;
    height: 100%;
    overflow: hidden;
    border-radius: var(--_container-shape);
    background: var(--md-sys-color-surface-container, #f3edf7);
  }

  ::slotted(img),
  ::slotted(picture),
  ::slotted(video) {
    display: block;
    width: 100%;
    height: 100%;
    object-fit: cover;
    transition: transform 0.3s cubic-bezier(0.2, 0, 0, 1);
  }

  @supports (animation-timeline: view()) {
    @media (prefers-reduced-motion: no-preference) {
      ::slotted(img),
      ::slotted(video) {
        transform-origin: center;
        animation: image-parallax auto linear both;
        animation-timeline: view(inline);
      }
    }
  }

  @keyframes image-parallax {
    0% {
      transform: scale(1.15) translateX(-8%);
    }
    100% {
      transform: scale(1.15) translateX(8%);
    }
  }

  @media (forced-colors: active) {
    .item {
      outline: 1px solid CanvasText;
    }
  }
`;
