/** @license SPDX-License-Identifier: Apache-2.0 */

import { Children, createElement, type ReactElement } from "react";
import type { TransformProps } from "../types.ts";

// Transform — applies `transform(children)` to flat string children before
// they reach the renderer. Mirrors Ink's `<Transform>` API. Non-string
// children (nested elements) are passed through untouched.
export function Transform(props: TransformProps): ReactElement {
  const { transform, children } = props;
  const parts: string[] = [];
  let allPrimitive = true;

  Children.forEach(children, (child) => {
    if (typeof child === "string" || typeof child === "number") {
      parts.push(String(child));
    } else if (child === null || child === undefined || child === false) {
      // skip
    } else {
      allPrimitive = false;
    }
  });

  if (allPrimitive) {
    const transformed = transform(parts.join(""));
    return createElement("Transform", {}, transformed);
  }
  return createElement("Transform", {}, children);
}
