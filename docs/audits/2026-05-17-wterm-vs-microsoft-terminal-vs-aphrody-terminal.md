<!-- SPDX-License-Identifier: Apache-2.0 -->
# Audit: wterm vs microsoft/terminal vs aphrody-terminal

**Date**: 2026-05-17
**Author**: Aphrody Core Team (AI Collaboration)
**Status**: Final

## 1. Executive Summary

This audit compares three terminal emulator architectures to justify the design and implementation of `aphrody-terminal` (Phase T of the 2026-05-17 Pivot).

1. **`vercel-labs/wterm`**: A lightweight, web-first terminal emulator built with TypeScript and Zig compiled to WASM. It relies on standard DOM elements for rendering.
2. **`microsoft/terminal`**: The official Windows Terminal. A massive, feature-rich C++ codebase utilizing DirectX for rendering and deep Windows OS integration (ConPTY).
3. **`aphrody-terminal`**: Our custom LLM-first terminal emulator. Built entirely in Rust (compiled to WASM for the frontend, native for the backend).

**Conclusion**: `aphrody-terminal` is not a generic terminal emulator clone. While it borrows architectural concepts from both predecessors, it is structurally optimized for AI agent interactions, native JSON parsing, and inline markdown rendering.

---

## 2. Architecture Comparison

### 2.1 `vercel-labs/wterm`
* **Language**: TypeScript (UI/Glue) + Zig (Core logic compiled to WASM).
* **Rendering**: DOM-based (Canvas/WebGL available via xterm.js but `wterm` focuses on minimal DOM).
* **Strengths**: Highly portable, runs entirely in the browser, easy to embed in web applications.
* **Weaknesses**: Generic. It does not understand structured output (JSON/Markdown) intrinsically. It treats all output as raw strings/ANSI escapes.

### 2.2 `microsoft/terminal`
* **Language**: C++ / C# / XAML.
* **Rendering**: DirectX (AtlasEngine) for extreme high-performance rendering of text and glyphs.
* **Strengths**: Unmatched performance on Windows, deep OS integration, tabs, panes, accessibility (UIA).
* **Weaknesses**: Windows-only (mostly). Extremely heavy build process. Difficult to embed in a cross-platform or web context. Unaware of modern LLM workflows (relies purely on ConPTY byte streams).

### 2.3 `aphrody-terminal`
* **Language**: Rust (Native + WASM).
* **Rendering**: Dual-target. `aphrody-terminal-wasm` for DOM/Browser rendering using Material Design 3 (M3) components, and `aphrody-tui` for native CLI environments.
* **Strengths**:
  - **LLM-First**: Natively intercepts and parses JSON payloads (`aphrody-terminal-json-out`) and inline Markdown (`aphrody-terminal-markdown`).
  - **OSC Extensions**: Implements custom Operating System Command (OSC) sequences to bridge the terminal with agent workflows (e.g., skill activation, MCP status bus).
  - **Cross-Platform**: The exact same Rust VT engine (`aphrody-terminal-vt`) runs in the native CLI backend and the WASM browser frontend.
  - **Component UI**: Allows rendering rich React/Ink-style UI components directly inside the terminal buffer via `packages/aphrody-jsx`.
* **Weaknesses**: Higher initial development cost to build a custom VT engine and renderer from scratch rather than wrapping `xterm.js`.

---

## 3. Deep Dive: Why Not Just Wrap `xterm.js` or `wterm`?

The primary requirement for `aphrody-terminal` was to support **Claude Code** and **Gemini CLI** workflows flawlessly. These CLIs output rich interactive components (spinners, multi-select menus, markdown blocks).

If we used `xterm.js` or `wterm`, the architecture would look like this:
`LLM CLI -> ANSI Bytes -> ConPTY -> WebSocket -> Browser -> xterm.js -> Canvas Render`

In this flow, the semantic meaning of a "Markdown Block" or a "Sub-agent Status" is permanently lost; it becomes just colored pixels.

With `aphrody-terminal`, the architecture is:
`LLM CLI -> JSON Envelopes / Custom OSC -> WebSocket -> aphrody-terminal-llm -> Structured State -> UI Render`

By controlling the VT parser (`aphrody-terminal-vt`), we can intercept custom sequences (`OSC aphrody-md`) and render true DOM components (or native Ratatui widgets) for that content, bypassing the grid-cell limitation of traditional terminals.

---

## 4. Feature Matrix

| Feature | `wterm` | `microsoft/terminal` | `aphrody-terminal` |
|---------|---------|----------------------|--------------------|
| **Core VT Parsing** | Yes | Yes (Exhaustive) | Yes (Targeted Ink/React TUI subset) |
| **Native JSON Parsing** | No | No | **Yes** |
| **Inline Markdown** | No | No | **Yes** |
| **WASM Target** | Yes | No | **Yes** |
| **Agent / MCP Bus** | No | No | **Yes** |
| **Rendering** | DOM | DirectX | DOM (M3) / Native TUI |
| **ConPTY Backend** | No | Yes | **Yes** (`aphrody-terminal-backend`) |

## 5. Next Steps

With Phase T implementation complete, `aphrody-terminal` now serves as the canonical host environment for all `aphrody` and AI-agent interactions. Future work will focus on optimizing the 60fps ratatui-style DSL (`aphrody-tui`) and expanding the `aphrody-jsx` component library.
