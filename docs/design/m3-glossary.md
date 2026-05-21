<!-- SPDX-License-Identifier: Apache-2.0 -->
# Material 3 glossary

A complete A–Z recap of the Material Design 3 terminology, mapped from
<https://m3.material.io/foundations/glossary>. All definitions below are
**paraphrased in our own words** (short summaries, not Google's verbatim text);
defer to the source page for canonical wording. Where a term corresponds to a
token already defined in [`aphrody-m3-tokens.md`](aphrody-m3-tokens.md), an
"→ aphrody" cross-reference is added.

## A

### Adaptive design
A design approach where the UI responds to known user, device, or environmental
conditions, adapting both layout and components.

### App bar
A component that surfaces information and actions at the top of a screen.

## B

### Banner
A component showing a prominent message together with optional related actions.

### Bottom sheet
A surface anchored to the bottom of the screen that holds supplementary content.

### Button
A component people use to start actions such as sending, sharing, or liking.

## C

### Card
A container that groups related content and actions about a single subject.

### Checkbox
A control for selecting one or more items from a set, or toggling an option on/off.

### Chip
A compact element for entering info, making selections, filtering, or triggering
actions, often shown in groups.

### Color: Baseline scheme
The default set of tones used as the out-of-the-box light and dark theme colors.
→ aphrody: the dark color-role table is our baseline scheme.

### Color: Dynamic color
A feature where a user-generated color scheme is mapped onto the app's color
roles, making the app's appearance change in response to user input.
→ aphrody: not used; aphrody ships a fixed scheme seeded from rust `#CE422B`.

### Color: Extended color
An extra color (beyond the key colors) added to fill custom roles, e.g. brand or
semantic meanings.
→ aphrody: `brand-rust` and `success` are extended colors.

### Color: Key color
A color derived (via hue/chroma transformation) from the source color; it is the
basis of a tonal palette and a conceptual aid rather than a coded value.

### Color: Scheme
Any mapping of color roles to specific tones from tonal palettes; a dark scheme
maps tones for a dark theme. A light scheme is not the same as a light theme.

### Color: Source color
The single seed color that all five key colors are derived from; it anchors a
dynamic scheme as one initiating hue/chroma/tone.
→ aphrody: the source color is rust `#CE422B`.

### Color: Tonal palette
A 13-tone range of one hue/chroma used to map tones to roles, giving automatic
contrast and differentiation within a color group.
→ aphrody: surface-container tones are drawn from such palettes.

### Color: Tone
Colors sharing the same hue and chroma; informally, the degree of lightness.
→ aphrody: surface levels (`surface-dim` → `surface-bright`) are tone steps.

### Color: User-generated schemes
The part of dynamic color derived from a user's wallpaper choice or Android
preset colors.

### Condition
A signal that determines when and how an adaptive layout or component should adapt.

### Contrast
The difference between colors; for accessibility it means difference in tone (a
40-tone gap gives WCAG ratio ≥ 3.0, a 50-tone gap ≥ 4.5).

### Customization
A change to a UI reflecting an app's, OEM's, or user's visual preference or brand,
applied to single elements or globally as a theme.

## D

### Dark theme
A low-light UI made up mostly of dark surfaces.
→ aphrody: aphrody is dark-first; this is the default theme.

### Data table
A component that lays out data in rows and columns.

### Date picker
A component for selecting a date or a range of dates.

### Design attribute
The style aspect (e.g. color or font) that a token or hard-coded value applies to.

### Design guidelines
Long-form descriptive, illustrated docs showing usage and behavior through
examples to aid problem-solving.

### Design specs
Annotated designs and docs that specify the exact values and parameters defining
a component or feature.

### Design System Package (DSP)
An open folder-structure format created by Adobe for sharing design-system
information across tools.

### Design tokens
Named, reusable design decisions that replace static values with
self-explanatory names within a design system.
→ aphrody: the entire `aphrody-m3-tokens.md` is built on M3 tokens.

### Design tokens: Context
The set of conditions under which tokens point to non-default values (e.g. dark
theme, dense layout).

### Design tokens: Role
A shortened/system version of a token's name (e.g. "Secondary container color").

### Design tokens: Types
Reference tokens (all tokenized values), component tokens (attributes of a
component's elements), and system tokens (the role choices that make up the
system).

### Design tokens: Value
The information defining a design attribute, whether stored in a token or
hard-coded.

### Dialog
A component presenting important prompts that require action, convey info, or
help complete a task.

### Divider
A thin line that groups content in lists and layouts.
→ aphrody: rendered with `outline-variant`.

## E

### Element
The part of a component that a token or value applies to, e.g. container or label.

### Extended FAB
A wider floating action button that fits a text label and a larger target area.

## F

### Floating action button (FAB)
A component representing the single most important action on a screen.

## H

### HCT
The hue–chroma–tone color space that powers dynamic color; built on CAM16 hue and
chroma plus the L* luminance construct (denoted T for tone).

## I

### Image list
A component showing a collection of images in an organized grid.

## L

### Libraries
Material 3 developer libraries: Android and Jetpack Compose, with Flutter and Web
support in progress.

### List
A component made of continuous vertical indexes of text or images.

## M

### Material Components
Open-source UI elements helping developers implement Material Design on Android,
Flutter, and the web.

### Material Design
An adaptable, open-source-backed system of guidelines, components, and tools for
UI design. M1 (2014), M2 (2018, introduced Material Theming), M3 (2021, added
Material You / dynamic color). **Material You** is M3's expressive, personal
visual style and feature set.

### Material Theming
Systematically customizing Material Design to reflect a product's brand. A theme
combines many styles/attributes (color, elevation, state) and adjusts global
styles for a given context such as low light or high contrast.

### Menu
A component showing a list of choices on a temporary surface.

### Mode
A binary device setting that helps people use the device better (e.g. focus,
airplane, battery-saver).

## N

### Navigation bar
A persistent component for switching between primary destinations in an app.

### Navigation drawer
A component giving ergonomic access to an app's destinations.

### Navigation rail
A component providing access to primary destinations on tablet and desktop screens.

## O

### Orbiter
Floating UI elements that control the content inside spatial (XR) panels.

## P

### Pane
A building block of a layout; content and actions are grouped into panes that
adapt to fit the screen.

### Progress indicator
A component showing an indefinite wait or the length of a process.

## R

### Radio button
A control for selecting exactly one option from a set.

### Role
A short nickname (also called a slot) describing a design token's purpose, e.g.
"On surface".
→ aphrody: e.g. `on-surface`, `primary-container` in the token table.

## S

### Side sheet
A component holding supplementary content anchored to the left or right edge.

### Slider
A component for selecting from a range of values.

### Snackbar
A component giving brief messages about app processes at the bottom of the screen.
→ aphrody: uses `inverse-surface`.

### Spatial
Relating to placing UI in space via extended reality (XR).

### Style
One or more (usually customizable) properties defining an aspect of UI appearance,
such as color, typography, or shape.

### Switch
A component that toggles a single item on or off.

## T

### Tab
A component for organizing content across screens, data sets, or interactions.

### Text field
A component for entering and editing text.

### Theme
A set of styles (color, elevation, type) applied globally to consistently adjust
an app's appearance.
→ aphrody: the dark theme defined in the tokens doc.

### Time picker
A component for selecting and setting a specific time.

### Toolbar
A component showing navigation and key actions at the bottom of mobile screens.

### Tooltip
A component showing informative text when users hover, focus, or tap an element.

## X

### XR
Extended reality: UI viewed in virtual reality or in passthrough blended with the
physical world.

## Source

Mapped from <https://m3.material.io/foundations/glossary> — fetched 2026-05-21.
Definitions paraphrased; the source page is authoritative.
