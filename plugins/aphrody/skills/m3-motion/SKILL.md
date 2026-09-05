---
name: m3-motion
description: >
  Apply Material Design 3 motion (transitions, easings, durations, springs) in React with
  @aphrody-code/m3-motion. Use when asked to "animate with M3 motion", "M3 transition
  pattern", "fade-through / shared-axis / container transform", "stagger a list", "M3
  easing/duration tokens", or "animate an md-* enter/exit".
---

# m3-motion

`@aphrody-code/m3-motion` provides the M3 motion system for React, built on Motion
(ex-Framer Motion). It encodes the M3 transition patterns, easing/duration tokens
and springs so animations match the spec instead of being hand-tuned.

## Tokens

```ts
import { m3Easings, m3Durations, m3Springs } from "@aphrody-code/m3-motion";
// m3Easings: standard / emphasized / emphasized-decelerate / emphasized-accelerate / …
// m3Durations: short1..4, medium1..4, long1..4, extra-long1..4 (ms)
// m3Springs: spatial / effects spring presets
```

## Transition components

| Component                             | M3 pattern               | Use for                                                         |
| ------------------------------------- | ------------------------ | --------------------------------------------------------------- |
| `M3Fade`                              | fade                     | simple enter/exit                                               |
| `M3FadeThrough`                       | fade-through             | switching unrelated content (keyed by `stateKey`)               |
| `M3SharedAxis`                        | shared-axis x / y / z    | sequential / hierarchical navigation                            |
| `M3ContainerTransform`                | container transform      | a card/element expanding into a detail surface                  |
| `M3ListStagger` + `M3ListStaggerItem` | list stagger             | a list/grid revealing its items                                 |
| `M3Collapse`                          | collapse                 | expand/collapse height (accordion, details)                     |
| `M3Transition`                        | any (via `pattern` prop) | one component, `pattern="fade-through" \| "shared-axis-x" \| …` |

```tsx
import { M3FadeThrough, M3Transition } from "@aphrody-code/m3-motion";

<M3FadeThrough stateKey={tab}>{panel}</M3FadeThrough>

<M3Transition pattern="shared-axis-x" show={open}>
  {content}
</M3Transition>
```

## Imperative animation

```ts
import { useM3Animate } from "@aphrody-code/m3-motion";
const [scope, animate] = useM3Animate();
// animate(scope.current, { opacity: 1 }, { duration: m3Durations.medium2 });
```

## Notes

- These wrap `motion/react`; props extend `HTMLMotionProps<"div">`, so `style`,
  `className`, layout props pass through.
- Pair with `@aphrody-code/m3-react` transition wrappers (`Fade`/`Grow`/`Zoom`/
  `Slide`/`Collapse`) which cover the MUI transition surface; `m3-motion` is the
  richer, pattern-level layer.
- Respect `prefers-reduced-motion`: gate emphasized transitions for accessibility.
