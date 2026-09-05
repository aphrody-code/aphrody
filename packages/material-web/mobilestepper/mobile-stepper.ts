/**
 * @license
 * Copyright 2026 Google LLC
 * SPDX-License-Identifier: Apache-2.0
 */

import { CSSResultOrNative } from "lit";
import { customElement } from "lit/decorators.js";

import { MobileStepper } from "./internal/mobile-stepper.js";
import { styles } from "./internal/mobile-stepper-styles.js";

declare global {
  interface HTMLElementTagNameMap {
    "md-mobile-stepper": MdMobileStepper;
  }
}

/**
 * @summary Mobile steppers show progress through a sequence of steps, typically
 * for paginated mobile content such as onboarding carousels.
 *
 * @description
 * Set `steps` to the total count and `active-step` to the current index. Choose
 * a progress treatment with `variant` (`dots`, `text`, or `progress`) and an
 * anchor with `position` (`bottom`, `top`, or `static`). The app supplies its
 * own navigation controls via the `back` and `next` slots.
 *
 * ```html
 * <md-mobile-stepper steps="6" active-step="2" variant="dots">
 *   <md-text-button slot="back">Back</md-text-button>
 *   <md-text-button slot="next">Next</md-text-button>
 * </md-mobile-stepper>
 * ```
 *
 * @final
 * @suppress {visibility}
 */
@customElement("md-mobile-stepper")
export class MdMobileStepper extends MobileStepper {
  static override styles: CSSResultOrNative[] = [styles];
}
