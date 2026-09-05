# @aphrody/m3-motion

High-fidelity React animation components for Material Design 3 (M3) motion specifications, powered by [Motion](https://motion.dev/) (formerly Framer Motion).

This package implements the core Material Design 3 motion patterns—including **Container Transform**, **Shared Axis**, **Fade Through**, **Collapse**, and **Fade**—using Google's official spring, easing, and duration tokens.

---

## Installation

```bash
bun add @aphrody/m3-motion motion
```

Ensure you have `react` (>= 18.0.0) and `motion` (>= 12.0.0) installed in your project.

---

## 1. Container Transform (`M3ContainerTransform`)

The container transform pattern transitions one UI element to another (e.g., morphing a small Card component into a full-page Detail view). 

### Key Features
* **Aspect Ratio Preservation**: Automatically prevents layout distortion (skewing/stretching) of children during container morphs using layout-position correction.
* **Spec-Compliant Cross-Fade**: Fades out the starting content early (0% to 30% of transition) and delays the fade-in of target content (20% to 100% of transition) to avoid messy intermediate states.
* **Pointer Event Management**: Prevents inactive, hidden content from intercepting pointer interactions.
* **Flexibility**: Supports both the **Single-Container Toggle** approach (re-rendering within the same container) and the **Two-Container Shared Layout** approach (different source/target elements linked via `layoutId`).

### Sub-Components
* `M3ContainerTransform`: Parent morphing container.
* `M3ContainerTransformStart`: Wrapper for starting content (e.g., collapsed card details).
* `M3ContainerTransformEnd`: Wrapper for ending content (e.g., expanded page details).

---

### Code Examples

#### Single-Container Toggle Approach
Best when morphing a single component instance in place using state.

```tsx
import React, { useState } from "react";
import { 
  M3ContainerTransform, 
  M3ContainerTransformStart, 
  M3ContainerTransformEnd 
} from "@aphrody/m3-motion";

export const MorphingCard = () => {
  const [isExpanded, setIsExpanded] = useState(false);

  return (
    <M3ContainerTransform
      isExpanded={isExpanded}
      speed="default"
      style={{
        width: isExpanded ? 500 : 250,
        height: isExpanded ? 400 : 150,
        borderRadius: isExpanded ? 28 : 16,
        backgroundColor: "var(--md-sys-color-surface-container)",
        cursor: "pointer"
      }}
      onClick={() => setIsExpanded(!isExpanded)}
    >
      {/* Start state (collapsed content) */}
      <M3ContainerTransformStart>
        <div style={{ padding: 16 }}>
          <h3>Collapsed Card</h3>
          <p>Click me to expand...</p>
        </div>
      </M3ContainerTransformStart>

      {/* End state (expanded detail content) */}
      <M3ContainerTransformEnd>
        <div style={{ padding: 24 }}>
          <h2>Detailed View</h2>
          <p>This is the expanded content showing the full detail surface.</p>
          <button onClick={(e) => { e.stopPropagation(); setIsExpanded(false); }}>
            Close
          </button>
        </div>
      </M3ContainerTransformEnd>
    </M3ContainerTransform>
  );
};
```

#### Two-Container Shared Layout Approach (linked via `layoutId`)
Best for page/route transitions or separate DOM elements. Wrap the conditional render in `<AnimatePresence>` to enable the exit fade-out transition.

```tsx
import React, { useState } from "react";
import { AnimatePresence } from "motion/react";
import { 
  M3ContainerTransform, 
  M3ContainerTransformStart, 
  M3ContainerTransformEnd 
} from "@aphrody/m3-motion";

export const ListView = () => {
  const [selectedId, setSelectedId] = useState<string | null>(null);

  return (
    <div>
      {/* List of items */}
      {!selectedId && (
        <div className="grid">
          <M3ContainerTransform 
            layoutId="card-item-1" 
            onClick={() => setSelectedId("1")}
            style={{ borderRadius: 16, background: "#eee", width: 200, height: 120 }}
          >
            <M3ContainerTransformStart>
              <div style={{ padding: 12 }}>Card Content 1</div>
            </M3ContainerTransformStart>
          </M3ContainerTransform>
        </div>
      )}

      {/* Expanded detail page */}
      <AnimatePresence>
        {selectedId === "1" && (
          <div className="backdrop">
            <M3ContainerTransform 
              layoutId="card-item-1" 
              style={{ borderRadius: 24, background: "#fff", width: 600, height: 450 }}
            >
              <M3ContainerTransformEnd>
                <div style={{ padding: 24 }}>
                  <h2>Expanded Title 1</h2>
                  <p>Long detail text...</p>
                  <button onClick={() => setSelectedId(null)}>Back to List</button>
                </div>
              </M3ContainerTransformEnd>
            </M3ContainerTransform>
          </div>
        )}
      </AnimatePresence>
    </div>
  );
};
```

---

## 2. Shared Axis (`M3SharedAxis`)

Used for transitions between UI elements that have a spatial or navigational relationship (e.g., page stepper, tab switching, or dialog overlays). Supports `x`, `y`, and `z` axes.

* Default enter duration: **300ms** (`medium2`) with `emphasizedDecelerate` ease.
* Default exit duration: **200ms** (`short4`) with `emphasizedAccelerate` ease.

### Code Example

```tsx
import React, { useState } from "react";
import { M3SharedAxis } from "@aphrody/m3-motion";

export const StepWizard = () => {
  const [step, setStep] = useState(0);

  return (
    <div>
      <M3SharedAxis 
        stateKey={step} 
        axis="x" 
        forward={step > 0} 
        style={{ padding: 20 }}
      >
        {step === 0 && <div>Step 1: Enter details</div>}
        {step === 1 && <div>Step 2: Review options</div>}
        {step === 2 && <div>Step 3: Confirm purchase</div>}
      </M3SharedAxis>
      
      <button onClick={() => setStep(prev => Math.max(0, prev - 1))}>Back</button>
      <button onClick={() => setStep(prev => Math.min(2, prev + 1))}>Next</button>
    </div>
  );
};
```

---

## 3. Fade Through (`M3FadeThrough`)

Used for transitions between primary views that do not have a strong spatial relationship (e.g., bottom navigation switching, settings screens).

* Default exit duration (fade-out): **90ms** with `emphasizedAccelerate` ease.
* Default enter duration (fade-in & scale-up from 92%): **210ms** with `emphasizedDecelerate` ease.

### Code Example

```tsx
import React, { useState } from "react";
import { M3FadeThrough } from "@aphrody/m3-motion";

export const AppNavigation = () => {
  const [currentTab, setCurrentTab] = useState("home");

  return (
    <div>
      <M3FadeThrough stateKey={currentTab} style={{ height: 400 }}>
        {currentTab === "home" && <div>Home View Content</div>}
        {currentTab === "search" && <div>Search Page Content</div>}
        {currentTab === "profile" && <div>User Profile Settings</div>}
      </M3FadeThrough>

      <nav>
        <button onClick={() => setCurrentTab("home")}>Home</button>
        <button onClick={() => setCurrentTab("search")}>Search</button>
        <button onClick={() => setCurrentTab("profile")}>Profile</button>
      </nav>
    </div>
  );
};
```

---

## 4. Duration and Easing Presets

Developers have direct access to official Material Design 3 timings and curves:

```tsx
import { m3Easings, m3Durations, m3Springs } from "@aphrody/m3-motion";

// Custom animation example using M3 tokens
const myCustomTransition = {
  ease: m3Easings.emphasizedDecelerate,
  duration: m3Durations.medium4, // 400ms
};
```

### Motion Presets Summary
* **`m3Easings`**: Bezier curves for `emphasized` (approx), `emphasizedDecelerate` (enter), `emphasizedAccelerate` (exit), `standard`, `standardDecelerate`, and `standardAccelerate`.
* **`m3Durations`**: Timing scales from `short1` (50ms) to `extraLong4` (1000ms).
* **`m3Springs`**: Hand-tuned spring constants (`fast`, `default`, `slow`) split into `spatial` (bouncy) and `effects` (critically damped).

---

## License

Apache-2.0
