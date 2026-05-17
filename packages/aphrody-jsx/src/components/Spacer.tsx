/** @license SPDX-License-Identifier: Apache-2.0 */

import { createElement, type ReactElement } from "react";

// Spacer — flex-grow filler. The renderer treats this as `flexGrow: 1` with
// no other styling.
export function Spacer(): ReactElement {
  return createElement("Spacer", { flexGrow: 1 });
}
