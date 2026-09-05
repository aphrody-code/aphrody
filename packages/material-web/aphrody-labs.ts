/**
 * @license
 * Copyright 2026 Google LLC
 * SPDX-License-Identifier: Apache-2.0
 */

/**
 * @fileoverview Promotes the upstream `labs/` Material 3 components that are
 * stable enough for first-party aphrody use into a single entry point: badge,
 * the three card variants, navigation bar/drawer/tab, and the (deprecated but
 * still useful) outlined segmented button set.
 *
 * These reuse upstream's SASS-compiled styles, so they build through the
 * package's `npm run build` pipeline (build:sass → build:css-to-ts → build:ts),
 * unlike the self-contained `aphrody-components.ts` bundle.
 *
 * Importing this module registers each custom element as a side effect.
 */

import "./labs/card/elevated-card.js";
import "./labs/card/filled-card.js";
import "./labs/card/outlined-card.js";
import "./labs/navigationbar/navigation-bar.js";
import "./labs/navigationdrawer/navigation-drawer.js";
import "./labs/navigationdrawer/navigation-drawer-modal.js";
import "./labs/navigationtab/navigation-tab.js";
import "./labs/segmentedbutton/outlined-segmented-button.js";
import "./labs/segmentedbuttonset/outlined-segmented-button-set.js";

export * from "./labs/card/elevated-card.js";
export * from "./labs/card/filled-card.js";
export * from "./labs/card/outlined-card.js";
export * from "./labs/navigationbar/navigation-bar.js";
export * from "./labs/navigationdrawer/navigation-drawer.js";
export * from "./labs/navigationdrawer/navigation-drawer-modal.js";
export * from "./labs/navigationtab/navigation-tab.js";
export * from "./labs/segmentedbutton/outlined-segmented-button.js";
export * from "./labs/segmentedbuttonset/outlined-segmented-button-set.js";
