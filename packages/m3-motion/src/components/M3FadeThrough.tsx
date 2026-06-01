import * as React from "react";
import { motion, AnimatePresence } from "motion/react";
import type { HTMLMotionProps, Transition, Variants } from "motion/react";
import { m3Easings } from "../easings.js";

export interface M3FadeThroughProps extends HTMLMotionProps<"div"> {
  /**
   * Unique key representing the active view/content. Changing this key triggers the transition.
   */
  stateKey: React.Key;
  /**
   * Custom transition for the fade through animation.
   */
  transition?: Transition;
}

/**
 * M3FadeThrough transition component.
 * Implements the Material 3 Fade Through navigation pattern.
 * The outgoing element fades out over 90ms, followed by the incoming element fading in and scaling up over 210ms.
 */
export const M3FadeThrough: React.FC<M3FadeThroughProps> = ({ stateKey, transition: customTransition, variants: customVariants, children, ...props }) => {
  const defaultVariants: Variants = {
    initial: {
      opacity: 0,
      scale: 0.92,
    },
    animate: {
      opacity: 1,
      scale: 1,
      transition: customTransition || {
        opacity: {
          ease: m3Easings.emphasizedDecelerate,
          duration: 0.21,
        },
        scale: {
          ease: m3Easings.emphasizedDecelerate,
          duration: 0.21,
        },
      },
    },
    exit: {
      opacity: 0,
      transition: customTransition || {
        opacity: {
          ease: m3Easings.emphasizedAccelerate,
          duration: 0.09,
        },
      },
    },
  };

  const resolvedVariants = customVariants || defaultVariants;

  return (
    <AnimatePresence mode="wait">
      <motion.div
        key={stateKey}
        variants={resolvedVariants}
        initial="initial"
        animate="animate"
        exit="exit"
        style={{ willChange: "opacity, transform" }}
        {...props}
      >
        {children}
      </motion.div>
    </AnimatePresence>
  );
};

M3FadeThrough.displayName = "M3FadeThrough";

