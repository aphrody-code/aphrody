/**
 * @license
 * Copyright 2026 Google LLC
 * SPDX-License-Identifier: Apache-2.0
 */

import { css } from "lit";

/** Material 3 top-app-bar styles (small / center / medium / large). */
export const styles = css`
  :host {
    --_container-color: var(--md-top-app-bar-container-color, var(--md-sys-color-surface, #fef7ff));
    --_on-scroll-container-color: var(
      --md-top-app-bar-on-scroll-container-color,
      var(--md-sys-color-surface-container, #f3edf7)
    );
    --_headline-color: var(
      --md-top-app-bar-headline-color,
      var(--md-sys-color-on-surface, #1d1b20)
    );
    --_leading-icon-color: var(
      --md-top-app-bar-leading-icon-color,
      var(--md-sys-color-on-surface, #1d1b20)
    );
    --_trailing-icon-color: var(
      --md-top-app-bar-trailing-icon-color,
      var(--md-sys-color-on-surface-variant, #49454f)
    );

    display: block;
    position: sticky;
    inset-block-start: 0;
    z-index: 4;
  }

  .bar {
    box-sizing: border-box;
    display: flex;
    flex-direction: column;
    width: 100%;
    background: var(--_container-color);
    color: var(--_headline-color);
    transition:
      background-color 250ms cubic-bezier(0.2, 0, 0, 1),
      box-shadow 250ms cubic-bezier(0.2, 0, 0, 1);
  }

  .bar.scrolled {
    background: var(--_on-scroll-container-color);
    box-shadow: var(
      --md-sys-elevation-level2,
      0 1px 2px 0 rgba(0, 0, 0, 0.3),
      0 2px 6px 2px rgba(0, 0, 0, 0.15)
    );
  }

  .row {
    display: flex;
    align-items: center;
    gap: 4px;
    min-height: 64px;
    padding-inline: 4px 4px;
  }

  .leading,
  .trailing {
    display: inline-flex;
    align-items: center;
    gap: 4px;
    color: var(--_leading-icon-color);
  }

  .trailing {
    color: var(--_trailing-icon-color);
    margin-inline-start: auto;
  }

  .spacer {
    flex: 1;
  }

  .headline {
    margin: 0;
    color: var(--_headline-color);
    font-family: var(
      --md-sys-typescale-title-large-font,
      "Google Sans Flex",
      Roboto,
      system-ui,
      sans-serif
    );
    font-weight: 400;
    font-size: var(--md-sys-typescale-title-large-size, 22px);
    line-height: 28px;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .headline.inline {
    flex: 1;
    padding-inline: 4px;
  }

  /* center-aligned */
  :host([variant="center"]) .headline.inline {
    text-align: center;
  }
  :host([variant="center"]) .leading {
    min-width: 48px;
  }

  /* medium */
  :host([variant="medium"]) .bar {
    min-height: 112px;
  }
  :host([variant="medium"]) .headline.block {
    padding-inline: 16px;
    padding-block-end: 24px;
    font-size: var(--md-sys-typescale-headline-small-size, 24px);
    line-height: 32px;
  }

  /* large */
  :host([variant="large"]) .bar {
    min-height: 152px;
  }
  :host([variant="large"]) .headline.block {
    padding-inline: 16px;
    padding-block-end: 28px;
    font-size: var(--md-sys-typescale-headline-medium-size, 28px);
    line-height: 36px;
  }

  .headline.block {
    flex: 1;
    display: flex;
    align-items: flex-end;
    white-space: normal;
  }

  /*
   * Scroll-driven on-scroll fill (declarative, compositor-friendly). Used when
   * no custom scroll target is set; the host gets [js-scroll] otherwise, which
   * disables this path in favour of the JS \`.scrolled\` toggle. Feature-detected
   * so unsupported browsers (e.g. Firefox) fall back to the JS listener.
   */
  @supports ((animation-timeline: scroll()) and (animation-range: 0% 100%)) {
    :host(:not([js-scroll])) .bar {
      animation: md-app-bar-fill auto linear both;
      animation-timeline: scroll(block nearest);
      animation-range: 0 24px;
    }
  }

  @keyframes md-app-bar-fill {
    to {
      background: var(--_on-scroll-container-color);
      box-shadow: var(
        --md-sys-elevation-level2,
        0 1px 2px 0 rgba(0, 0, 0, 0.3),
        0 2px 6px 2px rgba(0, 0, 0, 0.15)
      );
    }
  }

  @media (prefers-reduced-motion: reduce) {
    :host(:not([js-scroll])) .bar {
      animation: none;
    }
  }
`;
