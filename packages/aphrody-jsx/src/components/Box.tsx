/** @license SPDX-License-Identifier: Apache-2.0 */

import { createElement, type ReactElement } from "react";
import type { BoxProps } from "../types.ts";

// Box — flex container. Rendered as a `Box` host element; the terminal-side
// renderer applies taffy layout using the prop set. Padding short-hands are
// expanded here so the renderer only sees per-edge values.
export function Box(props: BoxProps): ReactElement {
  const {
    padding,
    paddingX,
    paddingY,
    margin,
    marginX,
    marginY,
    children,
    ...rest
  } = props;

  const expanded: Record<string, unknown> = { ...rest };

  if (padding !== undefined) {
    expanded.paddingTop ??= padding;
    expanded.paddingRight ??= padding;
    expanded.paddingBottom ??= padding;
    expanded.paddingLeft ??= padding;
  }
  if (paddingX !== undefined) {
    expanded.paddingLeft ??= paddingX;
    expanded.paddingRight ??= paddingX;
  }
  if (paddingY !== undefined) {
    expanded.paddingTop ??= paddingY;
    expanded.paddingBottom ??= paddingY;
  }
  if (margin !== undefined) {
    expanded.marginTop ??= margin;
    expanded.marginRight ??= margin;
    expanded.marginBottom ??= margin;
    expanded.marginLeft ??= margin;
  }
  if (marginX !== undefined) {
    expanded.marginLeft ??= marginX;
    expanded.marginRight ??= marginX;
  }
  if (marginY !== undefined) {
    expanded.marginTop ??= marginY;
    expanded.marginBottom ??= marginY;
  }

  // Use createElement directly so this file doesn't depend on its own JSX
  // runtime (which would create a circular import at build time).
  return createElement("Box", expanded, children);
}
