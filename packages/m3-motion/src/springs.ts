import type { Transition } from "motion/react";

/**
 * Material Design 3 Spring Transition Presets.
 * Driven by physical stiffness, damping, and mass.
 *
 * Category Guide:
 * - Spatial: Used for movement (x/y positions, width/height, scale, borders). Includes overshoot (bounce).
 * - Effects: Used for color changes, opacities, and gradients. Critically damped (no overshoot).
 */
export const m3Springs = {
  fast: {
    spatial: {
      type: "spring",
      stiffness: 400,
      damping: 24,
      mass: 0.8,
    } as Transition,
    effects: {
      type: "spring",
      stiffness: 350,
      damping: 37,
      mass: 0.8,
    } as Transition,
  },
  default: {
    spatial: {
      type: "spring",
      stiffness: 220,
      damping: 17,
      mass: 1.0,
    } as Transition,
    effects: {
      type: "spring",
      stiffness: 180,
      damping: 26,
      mass: 1.0,
    } as Transition,
  },
  slow: {
    spatial: {
      type: "spring",
      stiffness: 80,
      damping: 12,
      mass: 1.2,
    } as Transition,
    effects: {
      type: "spring",
      stiffness: 70,
      damping: 16,
      mass: 1.2,
    } as Transition,
  },
} as const;
