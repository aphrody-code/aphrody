<!-- SPDX-License-Identifier: Apache-2.0 -->
# Remotion & Material Design 3 Integration

This document describes how Remotion is used inside Aphrody to build a fully automated and programmatic video studio.

## 1. Overview
Aphrody incorporates a dynamic, programmatic video rendering studio in the Web frontend (`apps/web`). It marries the power of **Remotion** (React-based programmatic video editing) with **Material Design 3 (M3)** (Google's design system) to preview and export beautiful animations, technical code demos, and voice presentations.

## 2. Architecture & Components

### A. Live Player Preview
The live preview area in the Video Studio is powered by the `@remotion/player` component. It renders compositions in real time inside the browser.
*   **Location**: [`apps/web/src/aphrody/features/studio/Studio.tsx`](file:///home/ubuntu/aphrody/apps/web/src/aphrody/features/studio/Studio.tsx)
*   **State Syncing**: The player is controlled imperatively using a `useRef<PlayerRef>` handle.
    *   Play/Pause, replay, and seek actions are hooked to custom M3 transport controls (using `MdIconButton` and `MdSlider`).
    *   Frame updates are listened to via the `frameupdate` event to keep the custom seeker and timer in sync.

### B. Dynamic M3 Theming
Compositions dynamically consume M3 design tokens.
*   Background gradients update in real time when the user changes the active system theme (e.g. Sparkle, Cyber Sunset, Mint Forest, Midnight Deep).
*   Variables such as `var(--md-sys-color-primary)` and `var(--md-sys-color-primary-container)` are referenced directly within the canvas style declarations, ensuring visual cohesion.

### C. Compositions
All compositions are housed in [`apps/web/src/aphrody/features/studio/video/MascotVideo.tsx`](file:///home/ubuntu/aphrody/apps/web/src/aphrody/features/studio/video/MascotVideo.tsx).
Three templates are supported:
1.  **Mascotte Turntable 360° (`mascot`)**:
    *   Loops through 8 pre-rendered mascot turntable frames (`aphrody_body_r0` to `r7`).
    *   Features a smooth `spring` scaling entry effect.
    *   Draws procedural audio waveform bars matching the frame rhythm.
    *   Highlights voiceover text word-by-word at the bottom.
2.  **Démo Technique avec code (`showcase`)**:
    *   A high-fidelity simulated terminal output.
    *   Iteratively types out code scans (`mrx scan`) matching the frame index.
3.  **Présentation IA & Voix (`presentation`)**:
    *   Focuses on voiceover presentation.
    *   Draws a large pulsating central microphone icon and wider waveform spectrum.

## 3. Server-Side Export / Rendering
While the browser offers instant, interactive React-based previews, the actual video/animation export can be generated server-side.
*   **Command**: `aphrody image anim turntable`
*   **Flow**:
    1.  The user clicks **Générer & Automobiliser la Vidéo** in the UI.
    2.  The frontend issues a run command to the Bun server bridge.
    3.  The server executes python-based image toolchains using `uv` to compile the mascot turntable frames into an animated WebP/MP4 loop.
    4.  Completed assets are outputted to `/home/ubuntu/aphrody/assets/` and made available for immediate download in the **Rendus Récents** panel.

## 4. Setup & Dependencies
The following packages are installed:
*   `remotion` — Core framework helpers (`spring`, `interpolate`, `AbsoluteFill`, etc.).
*   `@remotion/player` — Interactive preview canvas.
*   `@remotion/media` — Visual asset manipulation.
