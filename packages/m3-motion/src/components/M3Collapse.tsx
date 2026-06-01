import * as React from "react";
import { motion, AnimatePresence } from "motion/react";
import type { HTMLMotionProps } from "motion/react";
import { m3Easings, m3Durations } from "../easings.js";

export interface M3CollapseProps extends HTMLMotionProps<"div"> {
  /**
   * If true, the component expands. If false, it collapses.
   * If omitted, the component behaves as a standard motion element meant to be unmounted/mounted
   * by an external <AnimatePresence>.
   */
  in?: boolean;
  /**
   * If true, animate the transition on initial mount.
   * @default false
   */
  appear?: boolean;
}

/**
 * M3Collapse transition component.
 * Animates the height and opacity of its children, perfect for accordions, menus, or disclosures.
 */
export const M3Collapse = React.forwardRef<HTMLDivElement, M3CollapseProps>(
  ({ in: inProp, appear = false, children, ...props }, ref) => {
    const variants = {
      initial: { height: 0, opacity: 0 },
      animate: {
        height: "auto",
        opacity: 1,
        transition: {
          height: {
            ease: m3Easings.emphasized,
            duration: m3Durations.medium4,
          },
          opacity: {
            ease: m3Easings.standard,
            duration: m3Durations.medium2,
          },
        },
      },
      exit: {
        height: 0,
        opacity: 0,
        transition: {
          height: {
            ease: m3Easings.emphasized,
            duration: m3Durations.medium4,
          },
          opacity: {
            ease: m3Easings.standard,
            duration: m3Durations.medium2,
          },
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
        style={{ overflow: "hidden", willChange: "height, opacity" }}
        {...props}
      >
        {children}
      </motion.div>
    );

    if (inProp !== undefined) {
      return <AnimatePresence initial={appear}>{inProp ? content : null}</AnimatePresence>;
    }

    return content;
  },
);

M3Collapse.displayName = "M3Collapse";
