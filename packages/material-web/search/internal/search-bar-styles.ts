/**
 * @license
 * Copyright 2026 Google LLC
 * SPDX-License-Identifier: Apache-2.0
 */

import { css } from "lit";

/** Material 3 search bar + view styles. */
export const styles = css`
  :host {
    --_container-color: var(
      --md-search-container-color,
      var(--md-sys-color-surface-container-high, #ece6f0)
    );
    --_text-color: var(--md-search-input-text-color, var(--md-sys-color-on-surface, #1d1b20));
    --_supporting-text-color: var(
      --md-search-supporting-text-color,
      var(--md-sys-color-on-surface-variant, #49454f)
    );
    --_leading-icon-color: var(
      --md-search-leading-icon-color,
      var(--md-sys-color-on-surface, #1d1b20)
    );

    display: block;
    position: relative;
    width: 100%;
    max-width: 720px;
  }

  .bar {
    box-sizing: border-box;
    display: flex;
    align-items: center;
    gap: 4px;
    height: 56px;
    padding-inline: 16px 8px;
    border-radius: var(--md-sys-shape-corner-full, 28px);
    background: var(--_container-color);
    cursor: text;
  }

  :host([open]) .bar {
    border-end-start-radius: 0;
    border-end-end-radius: 0;
  }

  .leading {
    display: inline-flex;
    align-items: center;
    color: var(--_leading-icon-color);
  }

  .search-icon,
  .close svg {
    width: 24px;
    height: 24px;
    fill: currentColor;
  }

  input {
    flex: 1;
    min-width: 0;
    appearance: none;
    border: none;
    outline: none;
    background: none;
    color: var(--_text-color);
    font-family: var(
      --md-sys-typescale-body-large-font,
      "Google Sans Flex",
      Roboto,
      system-ui,
      sans-serif
    );
    font-size: var(--md-sys-typescale-body-large-size, 16px);
    line-height: 24px;
    letter-spacing: 0.5px;
    padding-inline: 4px;
  }

  input::placeholder {
    color: var(--_supporting-text-color);
  }

  input::-webkit-search-cancel-button {
    display: none;
  }

  .trailing {
    display: inline-flex;
    align-items: center;
  }

  .close {
    appearance: none;
    border: none;
    background: none;
    cursor: pointer;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    width: 40px;
    height: 40px;
    border-radius: var(--md-sys-shape-corner-full, 9999px);
    color: var(--_supporting-text-color);
  }

  .close:hover {
    background: color-mix(in srgb, currentColor 8%, transparent);
  }

  .view {
    position: absolute;
    inset-inline: 0;
    inset-block-start: 56px;
    z-index: 6;
    background: var(--_container-color);
    border-end-start-radius: 28px;
    border-end-end-radius: 28px;
    box-shadow: var(
      --md-sys-elevation-level3,
      0 1px 3px 0 rgba(0, 0, 0, 0.3),
      0 4px 8px 3px rgba(0, 0, 0, 0.15)
    );
    max-height: 60vh;
    overflow: auto;
  }

  :host([view="fullscreen"][open]) {
    position: fixed;
    inset: 0;
    max-width: none;
    z-index: 24;
  }

  :host([view="fullscreen"]) .bar {
    height: 72px;
    border-radius: 0;
  }

  :host([view="fullscreen"]) .view {
    inset-block-start: 72px;
    border-radius: 0;
    max-height: none;
    box-shadow: none;
  }

  .scrim {
    position: fixed;
    inset: 0;
    z-index: -1;
    background: rgba(0, 0, 0, 0.32);
  }

  [hidden] {
    display: none;
  }
`;
