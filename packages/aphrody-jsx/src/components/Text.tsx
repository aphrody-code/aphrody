/** @license SPDX-License-Identifier: Apache-2.0 */

import { Children, createElement, type ReactElement, type ReactNode } from "react";
import type { TextProps } from "../types.ts";

// Recursively flatten children into a single string when they are primitives.
// Non-primitive React nodes (other components, arrays mixed with elements)
// are passed through as host children so the renderer can keep their styling
// (e.g. `<Text bold><Text italic>foo</Text></Text>`).
function flattenPrimitiveChildren(children: ReactNode): string | null {
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
  return allPrimitive ? parts.join("") : null;
}

// Text — styled inline text. Boolean style flags are normalized to true/false
// so the diff layer doesn't churn on `undefined` vs absent props.
export function Text(props: TextProps): ReactElement {
  const {
    children,
    bold = false,
    italic = false,
    underline = false,
    strikethrough = false,
    inverse = false,
    dimColor = false,
    ...rest
  } = props;

  const hostProps: Record<string, unknown> = { ...rest };
  if (bold) hostProps.bold = true;
  if (italic) hostProps.italic = true;
  if (underline) hostProps.underline = true;
  if (strikethrough) hostProps.strikethrough = true;
  if (inverse) hostProps.inverse = true;
  if (dimColor) hostProps.dimColor = true;

  const flat = flattenPrimitiveChildren(children);
  if (flat !== null) {
    return createElement("Text", hostProps, flat);
  }
  return createElement("Text", hostProps, children);
}
