"use client";

/**
 * @license
 * Copyright 2026 Google LLC
 * SPDX-License-Identifier: Apache-2.0
 */

import * as React from "react";

import {
  BaseTransitionProps,
  isVisible,
  resolveTimeout,
  useTransitionState,
} from "./use-transition-state.js";

const EASING = "var(--md-sys-motion-easing-standard, cubic-bezier(0.2, 0, 0, 1))";
const ENTER_DURATION_FALLBACK = 300;
const EXIT_DURATION_FALLBACK = 250;

export interface CollapseProps extends BaseTransitionProps {
  /** The axis along which to collapse. Defaults to `vertical` (height). */
  orientation?: "vertical" | "horizontal";
  /**
   * The size to keep visible while collapsed (e.g. `'0px'` or `'40px'` for a
   * peeking header). Defaults to `'0px'`.
   */
  collapsedSize?: string;
}

/**
 * Collapses its child along one axis by animating its measured size between
 * `collapsedSize` and its natural extent. The MUI `Collapse` equivalent.
 *
 * The child is wrapped in a clipping container so that overflow is hidden
 * during the animation. The natural size is measured from a live ref each
 * commit, so dynamic content is handled.
 */
export function Collapse(props: CollapseProps): React.ReactElement | null {
  const {
    in: inProp,
    timeout,
    appear = false,
    mountOnEnter = false,
    unmountOnExit = false,
    orientation = "vertical",
    collapsedSize = "0px",
    children,
  } = props;

  const enterMs = resolveTimeout(timeout, "enter", ENTER_DURATION_FALLBACK);
  const exitMs = resolveTimeout(timeout, "exit", EXIT_DURATION_FALLBACK);

  const { mounted, phase } = useTransitionState(inProp, {
    appear,
    mountOnEnter,
    unmountOnExit,
    enterTimeout: enterMs,
    exitTimeout: exitMs,
  });

  const innerRef = React.useRef<HTMLDivElement>(null);
  const [naturalSize, setNaturalSize] = React.useState<number>(0);
  const isHorizontal = orientation === "horizontal";
  const visible = isVisible(phase);

  // Measure the child's natural extent before paint whenever it should be
  // shown, so the enter animation has a concrete target size.
  React.useLayoutEffect(() => {
    const node = innerRef.current;
    if (!node) {
      return;
    }
    const measured = isHorizontal ? node.scrollWidth : node.scrollHeight;
    setNaturalSize(measured);
  }, [isHorizontal, phase, children]);

  if (!mounted) {
    return null;
  }

  const durationMs = visible ? enterMs : exitMs;
  const sizeProp = isHorizontal ? "width" : "height";
  // `entered` lets the content size itself naturally (responsive); the other
  // phases use an explicit pixel size so the transition can interpolate.
  const targetSize = phase === "entered" ? "auto" : visible ? `${naturalSize}px` : collapsedSize;

  const containerStyle: React.CSSProperties = {
    [sizeProp]: targetSize,
    overflow: "hidden",
    transition: `${sizeProp} ${durationMs}ms ${EASING}`,
    willChange: sizeProp,
  };

  const innerStyle: React.CSSProperties = isHorizontal
    ? { display: "inline-block", height: "100%" }
    : {};

  return (
    <div style={containerStyle}>
      <div ref={innerRef} style={innerStyle}>
        {children}
      </div>
    </div>
  );
}
