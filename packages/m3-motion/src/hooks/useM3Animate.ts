import { useAnimate } from "motion/react";
import type {
  AnimationScope,
  AnimationPlaybackControlsWithThen,
  AnimationSequence,
  SequenceOptions,
  ElementOrSelector,
  DOMKeyframesDefinition,
  AnimationOptions,
  MotionValue,
  UnresolvedValueKeyframe,
  ValueAnimationTransition,
  ObjectTarget,
} from "motion/react";
import { m3Easings, m3Durations } from "../easings.js";

/**
 * Overloaded type for the scoped animate function returned by useM3Animate.
 * Mirrors the overload set of the underlying motion animate, adding M3 defaults.
 */
export interface M3AnimateFn {
  (sequence: AnimationSequence, options?: SequenceOptions): AnimationPlaybackControlsWithThen;
  (
    value: string | MotionValue<string>,
    keyframes: string | UnresolvedValueKeyframe<string>[],
    options?: ValueAnimationTransition<string>,
  ): AnimationPlaybackControlsWithThen;
  (
    value: number | MotionValue<number>,
    keyframes: number | UnresolvedValueKeyframe<number>[],
    options?: ValueAnimationTransition<number>,
  ): AnimationPlaybackControlsWithThen;
  <V extends string | number>(
    value: V | MotionValue<V>,
    keyframes: V | UnresolvedValueKeyframe<V>[],
    options?: ValueAnimationTransition<V>,
  ): AnimationPlaybackControlsWithThen;
  (
    element: ElementOrSelector,
    keyframes: DOMKeyframesDefinition,
    options?: AnimationOptions,
  ): AnimationPlaybackControlsWithThen;
  <O extends object>(
    object: O | O[],
    keyframes: ObjectTarget<O>,
    options?: AnimationOptions,
  ): AnimationPlaybackControlsWithThen;
}

/**
 * A custom hook wrapping motion's `useAnimate` that injects Material 3 motion defaults
 * (emphasized easing and medium2 duration) for all imperative DOM animations.
 *
 * SSR-safe: useAnimate creates refs; no DOM access occurs at module or render time.
 */
export function useM3Animate<T extends Element = HTMLElement>(): [AnimationScope<T>, M3AnimateFn] {
  const [scope, animate] = useAnimate<T>();

  const m3Animate: M3AnimateFn = (
    elementOrSelector: unknown,
    keyframes?: unknown,
    options?: unknown,
  ) => {
    if (Array.isArray(elementOrSelector)) {
      // Sequence overload
      return (animate as CallableFunction)(
        elementOrSelector,
        keyframes as SequenceOptions,
      ) as AnimationPlaybackControlsWithThen;
    }

    if (
      typeof elementOrSelector === "string" ||
      elementOrSelector instanceof Element ||
      (typeof elementOrSelector === "object" &&
        elementOrSelector !== null &&
        "current" in elementOrSelector) ||
      typeof elementOrSelector === "number" ||
      (typeof elementOrSelector === "object" &&
        elementOrSelector !== null &&
        "get" in elementOrSelector)
    ) {
      return (animate as CallableFunction)(elementOrSelector, keyframes, {
        ease: m3Easings.emphasized,
        duration: m3Durations.medium2,
        ...(typeof options === "object" && options !== null ? options : {}),
      }) as AnimationPlaybackControlsWithThen;
    }

    return (animate as CallableFunction)(elementOrSelector, keyframes, {
      ease: m3Easings.emphasized,
      duration: m3Durations.medium2,
      ...(typeof options === "object" && options !== null ? options : {}),
    }) as AnimationPlaybackControlsWithThen;
  };

  return [scope, m3Animate];
}
