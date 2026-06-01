import * as React from "react";
import { motion, AnimatePresence } from "motion/react";
import type { HTMLMotionProps } from "motion/react";
import { m3Easings, m3Durations } from "../easings.js";

export interface M3FadeProps extends HTMLMotionProps<"div"> {
  /**
   * If true, the component will transition in. If false, it will transition out.
   * If omitted, the component behaves as a standard motion element meant to be unmounted/mounted
   * by an external <AnimatePresence>.
   */
  in?: boolean;
  /**
   * If true, animate the transition on initial mount.
   * @default true
   */
  appear?: boolean;
}

/**
 * M3Fade transition component.
 * Fades elements in/out using Material 3 standard decelerate/accelerate curves.
 */
export const M3Fade = React.forwardRef<HTMLDivElement, M3FadeProps>(
  ({ in: inProp, appear = true, children, ...props }, ref) => {
    const variants = {
      initial: { opacity: 0 },
      animate: {
        opacity: 1,
        transition: {
          ease: m3Easings.emphasizedDecelerate,
          duration: m3Durations.medium2,
        },
      },
      exit: {
        opacity: 0,
        transition: {
          ease: m3Easings.emphasizedAccelerate,
          duration: m3Durations.short4,
        },
      },
    };

    const content = (
      <motion.div
        ref={ref}
        variants={variants}
        initial={appear ? "initial" : false}
        animate="animate"
        exit="exit"
        style={{ willChange: "opacity" }}
        {...props}
      >
        {children}
      </motion.div>
    );

    if (inProp !== undefined) {
      return <AnimatePresence>{inProp ? content : null}</AnimatePresence>;
    }

    return content;
  },
);

M3Fade.displayName = "M3Fade";
