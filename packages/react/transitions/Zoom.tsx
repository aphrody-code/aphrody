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

const ENTER_EASING =
  "var(--md-sys-motion-easing-emphasized-decelerate, cubic-bezier(0.05, 0.7, 0.1, 1))";
const EXIT_EASING =
  "var(--md-sys-motion-easing-emphasized-accelerate, cubic-bezier(0.3, 0, 0.8, 0.15))";
const ENTER_DURATION_FALLBACK = 250;
const EXIT_DURATION_FALLBACK = 200;

/**
 * Zooms its child in from scale 0 to its natural size and back out, animating
 * only `transform`. The MUI `Zoom` equivalent.
 */
export function Zoom(props: BaseTransitionProps): React.ReactElement | null {
  const {
    in: inProp,
    timeout,
    appear = false,
    mountOnEnter = false,
    unmountOnExit = false,
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

  if (!mounted) {
    return null;
  }

  const visible = isVisible(phase);
  const durationMs = visible ? enterMs : exitMs;
  const easing = visible ? ENTER_EASING : EXIT_EASING;

  const childStyle = (children.props as { style?: React.CSSProperties }).style;
  const style: React.CSSProperties = {
    ...childStyle,
    transform: visible ? "scale(1)" : "scale(0)",
    transformOrigin: childStyle?.transformOrigin ?? "center center",
    transition: `transform ${durationMs}ms ${easing}`,
    willChange: "transform",
  };

  return React.cloneElement(children, {
    style,
  } as React.HTMLAttributes<HTMLElement>);
}
