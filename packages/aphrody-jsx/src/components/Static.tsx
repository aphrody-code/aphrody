/** @license SPDX-License-Identifier: Apache-2.0 */

import {
  Children,
  createElement,
  useEffect,
  useRef,
  type ReactElement,
  type ReactNode,
} from "react";
import type { StaticProps } from "../types.ts";

// Static — accumulator region. New items are appended on each render; items
// already committed are never re-emitted, which matches Ink's behavior and
// keeps the OSC traffic bounded for log-style UIs.
export function Static<T>(props: StaticProps<T>): ReactElement {
  const { items, children } = props;
  const committed = useRef<number>(0);

  useEffect(() => {
    committed.current = items.length;
  });

  const slice = items.slice(committed.current);
  const rendered: ReactNode[] = slice.map((item, offset) => {
    const idx = committed.current + offset;
    const node = children(item, idx);
    return Children.only(
      createElement("Box", { key: `aphrody-static-${idx}` }, node),
    );
  });

  return createElement("Static", { committedCount: committed.current }, rendered);
}
