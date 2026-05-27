<!-- SPDX-License-Identifier: Apache-2.0 -->

# Material 3 layout

> Paraphrased reference notes synthesized from the Material 3 "Layout" foundations
> section — <https://m3.material.io/foundations/layout>. Every definition below is
> reworded in our own terms; nothing is copied verbatim. Fetched 2026-05-22.

Layout in M3 is the deliberate arrangement of on-screen elements: it organizes
content, conveys hierarchy, and steers attention toward the primary actions. The
same guidance ships for Android and carries over to the web. As of the May 2026
revision, Google renamed "window size classes" to **breakpoints** and renamed
"responsive layout" to **adaptive design**; this doc uses the current naming and
notes the old terms where useful.

Core ideas:

- A **layout scaffold** is the standard skeleton you assemble screen parts onto.
- Begin from a **canonical layout example** rather than from a blank canvas.
- Make every layout scale across the five breakpoints.
- Support both LTR and RTL (bidirectionality) — navigation flips to the right side for RTL.
- Keep arrangement, sizing, and spacing consistent so structure stays legible.

---

## Parts of layout

M3 names seven layout elements: column, fold, margin, bar, drag handle, pane, and rail.
The scaffold composes these into a screen.

**Window** — the resizable, movable surface an app occupies. On desktop, multiple
windows can share the display (e.g. side by side over a taskbar) and each must adapt
to whatever size it is given.

**Grid** — the structural backbone underneath every layout. It groups related
content into columns, enforces uniform spacing, sets up focal points for key actions,
and aligns the bars/rails/panes. Column count, width, and spacing all shift per breakpoint.

**Bars** — frame the main content and host things like the app bar (top-of-screen
navigation plus 1–2 essential actions such as search or back) or a bottom navigation
bar. A bar may span one or several panes.

**Rails** — the perimeter band around (or floating above) panes. Rails hold primary
controls: navigation rails, toolbars, chat inputs, FABs, and similar.

**Panes** — the containers that hold the actual content; *all* content lives in a
pane. A layout has 1–3 panes whose widths adapt to the breakpoint and language. A
pane can be fixed, flexible, floating, or semi-permanent, and represents a single
destination (e.g. the message list vs. an open conversation). Showing several panes
at once makes a product faster to use.

- *Implicit grouping*: panes share the background color to imply relationship.
- *Explicit grouping*: distinct color or outline visually separates pane content.
- In XR/spatial environments, panes take a container color to stand out from the passthrough.

**Drag handles** — the control that resizes panes: widen a flexible pane, or collapse/
expand a fixed pane to toggle between one- and two-pane views.

**Spacer** — the gap between two panes across a foldable's hinge.

**Other terms** — *column* (vertical block(s) of content inside a pane), *fold* (the
flexible/hinge region splitting two foldable displays), *gap* (space between elements
in a container), *margin* (space between screen edge and inner elements), *rulers*
(global alignment guides keeping margins and placement consistent), *safety region*
(zones reserved for system UI like the status or gesture bar), *multi-window mode*
(several apps sharing one screen).

---

## Breakpoints (window size classes)

A breakpoint is the window width at which the layout should change to fit the
available space, device conventions, and ergonomics. Design for breakpoints, not
specific devices — available space is dynamic (multi-window, foldables, rotation).
The five Material breakpoints and their **exact dp ranges** (verified against the page):

| Breakpoint  | Width (dp)     | Typical devices                                                   | Panes (recommended) |
|-------------|----------------|-------------------------------------------------------------------|---------------------|
| Compact     | under 600      | phone in portrait                                                 | 1                   |
| Medium      | 600–839        | tablet / unfolded foldable in portrait                            | 1 (or 2)            |
| Expanded    | 840–1199       | phone or tablet in landscape, unfolded foldable landscape, desktop| 1 or 2 (2 preferred)|
| Large       | 1200–1599      | desktop                                                           | 1 or 2 (2 preferred)|
| Extra-large | 1600 and up    | desktop, ultra-wide monitors                                      | 1 to 3 (3 possible) |

Notes:

- As the window grows, layouts typically move from one pane to two, then three.
- Android also exposes compact/medium/expanded **height** breakpoints, but since most
  content scrolls vertically, layouts rarely need to react to available height.
- Rotation often moves a device between breakpoints (e.g. portrait→landscape).
- Component recommendations shift per breakpoint — e.g. navigation bar (compact) →
  collapsed navigation rail (medium/expanded) → expanded navigation rail (large/extra-large);
  bottom sheet (compact) → menu (medium+); full-screen dialog (compact) → basic dialog (medium+).

---

## Adaptive design

Adaptive design is a toolkit for reshaping an interface to fit context. Where
*responsive* design merely scales one layout to any size, *adaptive* design
customizes structure, components, and whole layouts to optimize each device. It
adapts based on **people** (preferences/settings), **devices** (watch → phone →
foldable → tablet → desktop → XR), and **usage** (resizing, rotating, switching device).

Three primary experience types — start at mobile and scale up:

- **Mobile** (phones, foldables, tablets): window modes are full-screen (default),
  split-screen (share with other apps), and bubbles (floating multitask windows).
- **Desktop**: free-form windows that adapt across breakpoints; supports split,
  floating, and free-form multi-tasking. A tablet with keyboard/mouse — or an Android
  phone on an external monitor — can become a desktop-like experience.
- **Spatial (XR)**: many free-form windows in near-limitless space; immersive "full
  space" modes let components float in 3D (a navigation rail can become a side orbiter).

Design for all input types (touch, pointer, keyboard) regardless of device, since the
same product may run in a desktop context.

**Adaptive strategies for panes** — *show and hide*, *levitate*, and *reflow*. Panes
can resize, enter/exit the screen, or rearrange as the window changes or the user
navigates. Pane presentations include *co-planar* (side by side), *floating* (above
other content, dialog-like), and *docked* (above content with one edge off-screen, bottom-sheet-like).

**Adapting components** — most components react via three strategies:

- *Resizing*: scale with the parent container, or hug content and stay edge-aligned
  (e.g. a button spanning full width vs. hugging its label).
- *Showing & hiding*: collapse/expand to reveal what fits (e.g. list items exposing
  extra text on a tablet).
- *Presentation changes*: shifts in orientation, color, type, shape, or configuration
  (e.g. FAB → extended FAB, or a navigation rail auto-expanding as the window grows).

**Five questions when moving up a breakpoint** — what to *reveal*, how to *divide*
(panes), what to *resize* (keep text ~40–60 chars/line), what to *reposition*
(reflow, second column, reachability), and what to *swap* (only functionally
equivalent components — e.g. navigation bar ↔ navigation rail; never button ↔ chip).

---

## Canonical layouts

Canonical examples are ready-made layouts for common screens across breakpoints,
shipped as code (Jetpack Compose, MDC-Android) to give products a strong starting
point. There are three, each configured for compact/medium/expanded:

- **Feed** — arranges cards (or similar) in a configurable grid for quickly scanning
  a large volume of content.
- **List-detail** — pairs an explorable list with the selected item's detail, splitting
  the window into two side-by-side panes (e.g. messaging: conversation list + open thread).
- **Supporting pane** — a primary area holding the main content (~two-thirds of the
  window) plus a secondary panel for supporting content in the remainder.

**Advanced/custom** — build on a canonical layout or layer scaffold elements. The
*levitate* strategy floats a pane above content for focused tasks (shopping basket,
replying to comments, creating a calendar event).

---

## Spacing, grids, and margins basics

- The **grid** organizes columns, spacing, and alignment; column count/width/spacing
  adjust per breakpoint.
- **Margins** are the space between the screen edge and inner elements; **gaps** are
  the space between elements within a container.
- **Rulers** provide global alignment lines so margins and placement stay consistent product-wide.
- Across breakpoints, tune margins and type styles to keep line length around **40–60 characters**.
- At medium breakpoints, two panes only suit low-density content with clear actions —
  avoid two panes for dense information.
- M3 ships a dedicated **spacing system & tokens** (see m3.material.io spacing pages)
  plus the M3 Design Kit (Figma) as the source for exact values.

---

## → aphrody mapping

Where this guidance plausibly lands in the aphrody workspace. The right column
describes *potential* consumption points, not existing APIs — `mui-rs` today ships
M3 components (Card, app bars, BottomSheet, Dialog) and an `m3-tokens`/`Theme` system,
but no breakpoint/pane/scaffold types yet, so several rows are forward-looking.

| M3 layout concept        | aphrody surface                          | How it could be consumed                                                                 |
|--------------------------|------------------------------------------|------------------------------------------------------------------------------------------|
| Breakpoints (5 classes)  | `mui-rs` (renderer: winit/wgpu)          | A breakpoint enum keyed off window dp could drive a responsive scaffold selecting pane count. |
| Panes (1–3, adaptive)    | `mui-rs-components` containers           | Pane/scaffold container types alongside the existing Card/app-bar components.            |
| Bars / rails             | `mui-rs-components` (`TopAppBar`, `BottomAppBar`) | App/bottom bars exist; rails (navigation rail/toolbar) would extend the same module.     |
| Spacing system & tokens  | `m3-tokens` + `mui-rs-core::Theme`       | Spacing/margin tokens fit the existing token + Theme infrastructure.                     |
| Canonical layouts        | `mui-rs` examples (e.g. `m3_showcase`)   | Feed / list-detail / supporting-pane could become example scaffolds or presets.          |
| Adaptive component strategies | `mui-rs-motion` + components        | Resize / show-hide / presentation transitions map onto the motion crate.                 |
| Breakpoints (text/columns) | `aphrody-terminal-*` (config, vt)       | Column count and reflow heuristics could reuse the compact/medium/expanded thresholds for terminal panes. |

Sources: the six M3 layout pages under `foundations/layout` (overview, parts-of-layout,
adaptive-design, understanding-layout, canonical-layouts, applying-layout/window-size-classes).
