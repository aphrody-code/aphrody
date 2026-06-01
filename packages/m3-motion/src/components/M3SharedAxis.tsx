import * as React from "react";
import { motion, AnimatePresence } from "motion/react";
import type { HTMLMotionProps, Transition, Variants } from "motion/react";
import { m3Easings, m3Durations } from "../easings.js";

export interface M3SharedAxisProps extends HTMLMotionProps<"div"> {
  /**
   * If true, the component will transition in. If false, it will transition out.
   * If omitted, the component behaves as a standard motion element meant to be unmounted/mounted
   * by an external <AnimatePresence>.
   */
  in?: boolean;
  /**
   * Axis of translation.
   * @default "x"
   */
  axis?: "x" | "y" | "z";
  /**
   * Direction of navigation. If true, slides forward/up/zoom-in. If false, slides backward/down/zoom-out.
   * @default true
   */
  forward?: boolean;
  /**
   * If true, animate the transition on initial mount.
   * @default true
   */
  appear?: boolean;
  /**
   * Custom transition for the shared axis animation.
   */
  transition?: Transition;
}

/**
 * M3SharedAxis transition component.
 * Implements the Material 3 Shared Axis motion pattern (X, Y, Z axes).
 */
export const M3SharedAxis = React.forwardRef<HTMLDivElement, M3SharedAxisProps>(
  ({ in: inProp, axis = "x", forward = true, appear = true, transition: customTransition, variants: customVariants, children, ...props }, ref) => {
    // Offset values according to M3 specifications (usually 30px translation or ~8% scale change)
    const translateOffset = 30;
    const scaleOffset = 0.08;

    const defaultVariants: Variants = {
      initial: {
        opacity: 0,
        x: axis === "x" ? (forward ? translateOffset : -translateOffset) : 0,
        y: axis === "y" ? (forward ? translateOffset : -translateOffset) : 0,
        scale: axis === "z" ? (forward ? 1 - scaleOffset : 1 + scaleOffset) : 1,
      },
      animate: {
        opacity: 1,
        x: 0,
        y: 0,
        scale: 1,
        transition: customTransition || {
          x: { ease: m3Easings.emphasizedDecelerate, duration: m3Durations.medium2 },
          y: { ease: m3Easings.emphasizedDecelerate, duration: m3Durations.medium2 },
          scale: { ease: m3Easings.emphasizedDecelerate, duration: m3Durations.medium2 },
          opacity: { ease: m3Easings.standard, duration: m3Durations.medium2 },
        },
      },
      exit: {
        opacity: 0,
        x: axis === "x" ? (forward ? -translateOffset : translateOffset) : 0,
        y: axis === "y" ? (forward ? -translateOffset : translateOffset) : 0,
        scale: axis === "z" ? (forward ? 1 + scaleOffset : 1 - scaleOffset) : 1,
        transition: customTransition || {
          x: { ease: m3Easings.emphasizedAccelerate, duration: m3Durations.short4 },
          y: { ease: m3Easings.emphasizedAccelerate, duration: m3Durations.short4 },
          scale: { ease: m3Easings.emphasizedAccelerate, duration: m3Durations.short4 },
          opacity: { ease: m3Easings.standard, duration: m3Durations.short4 },
        },
      },
    };

    const resolvedVariants = customVariants || defaultVariants;

    const content = (
      <motion.div
        ref={ref}
        variants={resolvedVariants}
        initial={appear ? "initial" : false}
        animate="animate"
        exit="exit"
        style={{ willChange: "opacity, transform" }}
        {...props}
      >
        {children}
      </motion.div>
    );

    if (inProp !== undefined) {
      return <AnimatePresence mode="wait">{inProp ? content : null}</AnimatePresence>;
    }

    return content;
  },
);

M3SharedAxis.displayName = "M3SharedAxis";

