"use client";

/**
 * @license
 * Copyright 2026 Google LLC
 * SPDX-License-Identifier: Apache-2.0
 */

import * as React from "react";

/**
 * Resolves an MUI-style `timeout` (a single duration or per-direction
 * durations) to the millisecond value for the given direction.
 */
export type TransitionTimeout = number | { appear?: number; enter?: number; exit?: number };

export function resolveTimeout(
  timeout: TransitionTimeout | undefined,
  direction: "appear" | "enter" | "exit",
  fallback: number,
): number {
  if (timeout == null) {
    return fallback;
  }
  if (typeof timeout === "number") {
    return timeout;
  }
  if (direction === "appear") {
    return timeout.appear ?? timeout.enter ?? fallback;
  }
  return timeout[direction] ?? fallback;
}

/** The discrete lifecycle phases of a transition. */
export type TransitionPhase = "entering" | "entered" | "exiting" | "exited";

/** Props shared by every transition component. */
export interface BaseTransitionProps {
  /** Whether the child should be shown. */
  in: boolean;
  /**
   * Duration in ms, or per-direction durations. When omitted, the component's
   * M3 motion-token default is used.
   */
  timeout?: TransitionTimeout;
  /** Animate on the initial mount when `in` is already true. */
  appear?: boolean;
  /** Keep the child mounted while hidden instead of unmounting it. */
  mountOnEnter?: boolean;
  /** Unmount the child once it has finished exiting. */
  unmountOnExit?: boolean;
  /** The single React element to animate. */
  children: React.ReactElement;
}

export interface UseTransitionStateResult {
  /** Whether the child should currently be rendered in the tree. */
  mounted: boolean;
  /** The current lifecycle phase. */
  phase: TransitionPhase;
  /** Whether this is the very first commit (used to gate `appear`). */
  isAppearing: boolean;
  /** Call from the element's `onTransitionEnd` to settle the phase. */
  handleTransitionEnd: () => void;
}

/**
 * Drives mount/unmount and entering/entered/exiting/exited phases from the
 * boolean `in` prop, using a small internal state machine. No external
 * dependency. SSR-safe (effects run only on the client).
 */
export function useTransitionState(
  inProp: boolean,
  {
    appear = false,
    mountOnEnter = false,
    unmountOnExit = false,
    enterTimeout,
    exitTimeout,
    onEntered,
    onExited,
  }: {
    appear?: boolean;
    mountOnEnter?: boolean;
    unmountOnExit?: boolean;
    enterTimeout: number;
    exitTimeout: number;
    onEntered?: () => void;
    onExited?: () => void;
  },
): UseTransitionStateResult {
  const firstRender = React.useRef(true);
  const isAppearing = firstRender.current && inProp && appear;

  const [phase, setPhase] = React.useState<TransitionPhase>(() => {
    if (inProp) {
      return appear ? "entering" : "entered";
    }
    return "exited";
  });

  // Whether the element is in the DOM at all.
  const [mounted, setMounted] = React.useState(() => inProp || (!mountOnEnter && !unmountOnExit));

  const timer = React.useRef<ReturnType<typeof setTimeout> | undefined>(undefined);
  const clear = () => {
    if (timer.current !== undefined) {
      clearTimeout(timer.current);
      timer.current = undefined;
    }
  };

  React.useEffect(() => {
    firstRender.current = false;
    return clear;
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  React.useEffect(() => {
    if (inProp) {
      clear();
      setMounted(true);
      // Defer to the next frame so the element mounts in its closed state
      // before transitioning to open (otherwise no transition fires).
      const raf = requestAnimationFrame(() => {
        setPhase("entering");
        timer.current = setTimeout(() => {
          setPhase("entered");
          onEntered?.();
        }, enterTimeout);
      });
      return () => {
        cancelAnimationFrame(raf);
      };
    }

    clear();
    setPhase("exiting");
    timer.current = setTimeout(() => {
      setPhase("exited");
      if (unmountOnExit) {
        setMounted(false);
      }
      onExited?.();
    }, exitTimeout);
    return undefined;
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [inProp, enterTimeout, exitTimeout, unmountOnExit]);

  const handleTransitionEnd = React.useCallback(() => {
    // The timeout is the source of truth; this is a best-effort fast-path.
  }, []);

  return { mounted, phase, isAppearing, handleTransitionEnd };
}

/** True while the element should be in its "shown" visual state. */
export function isVisible(phase: TransitionPhase): boolean {
  return phase === "entering" || phase === "entered";
}
