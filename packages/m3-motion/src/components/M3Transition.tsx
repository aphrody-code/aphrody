"use client";
import * as React from "react";
import { AnimatePresence, motion } from "motion/react";
import type { HTMLMotionProps, MotionStyle, Transition, Variants } from "motion/react";
import { m3Easings, m3Durations } from "../easings.js";

export type M3TransitionPattern =
  | "fade"
  | "fade-through"
  | "shared-axis-x"
  | "shared-axis-y"
  | "shared-axis-z"
  | "collapse";

export interface M3TransitionProps extends Omit<HTMLMotionProps<"div">, "variants" | "style"> {
  style?: MotionStyle;
  /**
   * Unique key representing the active view/content. Changing this key triggers the transition.
   */
  stateKey: React.Key;
  /**
   * Which M3 transition pattern to apply.
   * @default "fade"
   */
  pattern?: M3TransitionPattern;
  /**
   * Direction of navigation for shared-axis-* patterns.
   * @default true
   */
  forward?: boolean;
  /**
   * Custom variants — overrides the pattern defaults entirely.
   */
  variants?: Variants;
  /**
   * AnimatePresence mode.
   * @default "wait"
   */
  mode?: "wait" | "sync" | "popLayout";
}

const OFFSET = 30;
const SCALE_OFFSET = 0.08;

function buildVariants(pattern: M3TransitionPattern, forward: boolean): Variants {
  const enterTransition: Transition = {
    ease: m3Easings.emphasizedDecelerate,
    duration: m3Durations.medium2,
  };
  const exitTransition: Transition = {
    ease: m3Easings.emphasizedAccelerate,
    duration: m3Durations.short4,
  };

  switch (pattern) {
    case "fade-through":
      return {
        initial: { opacity: 0, scale: 0.92 },
        animate: {
          opacity: 1,
          scale: 1,
          transition: {
            opacity: { ease: m3Easings.emphasizedDecelerate, duration: 0.21 },
            scale: { ease: m3Easings.emphasizedDecelerate, duration: 0.21 },
          },
        },
        exit: {
          opacity: 0,
          transition: { opacity: { ease: m3Easings.emphasizedAccelerate, duration: 0.09 } },
        },
      };
    case "shared-axis-x":
      return {
        initial: { opacity: 0, x: forward ? OFFSET : -OFFSET },
        animate: {
          opacity: 1,
          x: 0,
          transition: {
            x: enterTransition,
            opacity: { ease: m3Easings.standard, duration: m3Durations.medium2 },
          },
        },
        exit: {
          opacity: 0,
          x: forward ? -OFFSET : OFFSET,
          transition: {
            x: exitTransition,
            opacity: { ease: m3Easings.standard, duration: m3Durations.short4 },
          },
        },
      };
    case "shared-axis-y":
      return {
        initial: { opacity: 0, y: forward ? OFFSET : -OFFSET },
        animate: {
          opacity: 1,
          y: 0,
          transition: {
            y: enterTransition,
            opacity: { ease: m3Easings.standard, duration: m3Durations.medium2 },
          },
        },
        exit: {
          opacity: 0,
          y: forward ? -OFFSET : OFFSET,
          transition: {
            y: exitTransition,
            opacity: { ease: m3Easings.standard, duration: m3Durations.short4 },
          },
        },
      };
    case "shared-axis-z":
      return {
        initial: { opacity: 0, scale: forward ? 1 - SCALE_OFFSET : 1 + SCALE_OFFSET },
        animate: {
          opacity: 1,
          scale: 1,
          transition: {
            scale: enterTransition,
            opacity: { ease: m3Easings.standard, duration: m3Durations.medium2 },
          },
        },
        exit: {
          opacity: 0,
          scale: forward ? 1 + SCALE_OFFSET : 1 - SCALE_OFFSET,
          transition: {
            scale: exitTransition,
            opacity: { ease: m3Easings.standard, duration: m3Durations.short4 },
          },
        },
      };
    case "collapse":
      return {
        initial: { height: 0, opacity: 0 },
        animate: {
          height: "auto",
          opacity: 1,
          transition: {
            height: { ease: m3Easings.emphasized, duration: m3Durations.medium4 },
            opacity: { ease: m3Easings.standard, duration: m3Durations.medium2 },
          },
        },
        exit: {
          height: 0,
          opacity: 0,
          transition: {
            height: { ease: m3Easings.emphasized, duration: m3Durations.medium4 },
            opacity: { ease: m3Easings.standard, duration: m3Durations.medium2 },
          },
        },
      };
    case "fade":
    default:
      return {
        initial: { opacity: 0 },
        animate: { opacity: 1, transition: enterTransition },
        exit: { opacity: 0, transition: exitTransition },
      };
  }
}

/**
 * M3Transition — generic Material 3 transition wrapper.
 * Supports all standard M3 patterns via the `pattern` prop.
 * The `stateKey` prop drives AnimatePresence re-mounts.
 */
export const M3Transition = React.forwardRef<HTMLDivElement, M3TransitionProps>(
  (
    {
      stateKey,
      pattern = "fade",
      forward = true,
      mode = "wait",
      variants,
      children,
      style,
      ...props
    },
    ref,
  ) => {
    const resolvedVariants = variants ?? buildVariants(pattern, forward);
    const baseStyle: MotionStyle =
      pattern === "collapse" ? { overflow: "hidden" } : { willChange: "opacity, transform" };
    const resolvedStyle: MotionStyle = { ...baseStyle, ...style };

    return (
      <AnimatePresence mode={mode}>
        <motion.div
          key={stateKey}
          ref={ref}
          variants={resolvedVariants}
          initial="initial"
          animate="animate"
          exit="exit"
          style={resolvedStyle}
          {...props}
        >
          {children}
        </motion.div>
      </AnimatePresence>
    );
  },
);

M3Transition.displayName = "M3Transition";
