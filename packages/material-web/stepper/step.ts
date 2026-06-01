/**
 * @license
 * Copyright 2026 Google LLC
 * SPDX-License-Identifier: Apache-2.0
 */

import { CSSResultOrNative } from "lit";
import { customElement } from "lit/decorators.js";

import { Step } from "./internal/step.js";
import { styles } from "./internal/step-styles.js";

declare global {
  interface HTMLElementTagNameMap {
    "md-step": MdStep;
  }
}

/**
 * @summary A single step within an `<md-stepper>`.
 *
 * @description
 * Holds the step's label and completion/optional/editable flags, plus its
 * content in the default slot. The content is shown only while this is the
 * active step; the parent `<md-stepper>` draws the indicator and connector.
 *
 * ```html
 * <md-step label="Address" optional>
 *   <md-outlined-text-field label="Street"></md-outlined-text-field>
 * </md-step>
 * ```
 *
 * @final
 * @suppress {visibility}
 */
@customElement("md-step")
export class MdStep extends Step {
  static override styles: CSSResultOrNative[] = [styles];
}
