---
name: clone-gemini-app
description: Skill to recursively scrape, reverse-engineer, and perfectly clone the Gemini web app into a native Rust desktop app using our MD3 UI library.
---

Loaded when the user asks you to clone, reverse-engineer, or rebuild the Gemini Web Interface (`https://gemini.google.com/app`) into our native Rust/MD3 environment.

## Goal
Build the ultimate native Rust desktop application replicating the Gemini Web Interface with pixel-perfect accuracy, using our internal `packages/ui` (Material Web Components) and a native Rust backend.

## Iteration Loop (The Pixel-Perfect Protocol)

This skill requires you to operate in an autonomous loop:

1. **Scrape, Fetch, & Crawl**
   - Target: `https://gemini.google.com/app`.
   - Use web fetching tools or a `browser_subagent` to capture the live DOM, CSS variables, and layout structures.
   - Extract raw tokens, color roles, spacing scales, and typography hierarchies.

2. **Snapshot & Screenshot**
   - Take visual screenshots of the target interface.
   - Take snapshots of the generated local UI.

3. **Reverse & Analyze**
   - Compare the live Gemini DOM with our `DESIGN.md` guidelines.
   - Identify specific MD3 components used (e.g., `md-filled-text-field`, `md-elevation`, `md-icon-button`).
   - Map Google Sans Flex configurations (wght, opsz) and CSS transitions.

4. **Clone & Build UI (MD3)**
   - Reconstruct the interface using strictly `@material/web` components from `packages/ui`.
   - Apply our Desktop Best Practices (Three-pane layout, high density, `Google Sans Mono` for code blocks, `Google Sans Flex` for UI).
   - Never use generic frameworks (like Tailwind) or inline styles for colors; always use `--md-sys-color-*` variables.

5. **Build Native Backend (Rust)**
   - Architect a high-performance native backend in Rust (integrating with `crosvm` or WinClean's native bindings) to serve the UI and handle local OS interactions.
   - Ensure the backend handles state management, IPC (Inter-Process Communication), and background agent lifecycle gracefully.

6. **Evaluate & Refine (Iterate)**
   - Visually and structurally compare the local build against the scraped Gemini app.
   - Identify discrepancies in margin, padding, typography, color tone, or animation easing.
   - Automatically correct the local codebase and repeat the loop until 100% pixel-perfect accuracy is achieved.

## Hard Rules

- **No Hallucination**: Do not invent UI components. If it exists in the real Gemini app, map it to the closest `@material/web` equivalent or build a faithful Custom Element.
- **Strict Typography**: Enforce `Google Sans Flex` (UI), `Google Sans Text` (Markdown/Code reading), and `Google Sans Mono` (Terminal/Raw text).
- **Backend Integrity**: The Rust backend must remain decoupled from the UI logic, focusing purely on high-performance I/O, IPC, and native OS capabilities.
- **Autonomous Execution**: If a deviation is found during the visual check, do not ask for user permission to fix it. Fix it, rebuild, and re-check.

## Ending the Task
The task is complete only when the visual diff between the native Rust application and `https://gemini.google.com/app` is indistinguishable, and the Rust backend is successfully compiling and running. At that point, present the final artifact paths and a summary of the architecture.
