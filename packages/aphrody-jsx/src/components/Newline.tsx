/** @license SPDX-License-Identifier: Apache-2.0 */

import { createElement, type ReactElement } from "react";

export interface NewlineProps {
  count?: number;
}

// Newline — emits one or more line breaks. The renderer treats `count` as the
// number of vertical advance units to insert.
export function Newline({ count = 1 }: NewlineProps = {}): ReactElement {
  return createElement("Newline", { count });
}
