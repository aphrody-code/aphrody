/**
 * @license
 * Copyright 2026 Google LLC
 * SPDX-License-Identifier: Apache-2.0
 */

/**
 * @fileoverview Aphrody's Material Design 3 extension bundle — the components,
 * layouts, typography, and brand effects added on top of upstream
 * `@aphrody/material-web` to complete the M3 catalog (snackbar, app-bars, navigation
 * rail, search, toolbars, sheets, carousel, loading indicator, button group,
 * FAB menu, date/time pickers), the adaptive layout family (scaffold, panes,
 * list-detail, supporting-pane), the Google Sans Flex typography element, and
 * the WebGPU brand-effect surface.
 *
 * Each component is self-contained: it consumes the `--md-sys-*` design tokens
 * directly (with fallbacks) and does not depend on the SASS-compiled style
 * pipeline, so it builds with `tsc` + `lit` alone.
 *
 * Importing this module registers every custom element as a side effect.
 */

// ── Communication & feedback ──────────────────────────────────────────────
import "./snackbar/snackbar.js";
import "./loadingindicator/loading-indicator.js";

// ── Navigation ────────────────────────────────────────────────────────────
import "./navigationrail/navigation-rail.js";
import "./navigationrail/navigation-rail-item.js";
import "./appbar/top-app-bar.js";
import "./appbar/bottom-app-bar.js";
import "./search/search-bar.js";
import "./toolbar/md-toolbar.js";

// ── Containment ───────────────────────────────────────────────────────────
import "./sheet/bottom-sheet.js";
import "./sheet/side-sheet.js";
import "./carousel/carousel.js";
import "./carousel/carousel-item.js";

// ── Action & selection ────────────────────────────────────────────────────
import "./buttongroup/button-group.js";
import "./fabmenu/fab-menu.js";
import "./fabmenu/fab-menu-item.js";
import "./datepicker/date-picker.js";
import "./timepicker/time-picker.js";

// ── Adaptive layout ───────────────────────────────────────────────────────
import "./layout/md-scaffold.js";
import "./layout/md-pane.js";
import "./layout/md-list-detail.js";
import "./layout/md-supporting-pane.js";

// ── Angular-Material parity (data, disclosure, navigation helpers) ─────────
import "./tooltip/tooltip.js";
import "./expansion/expansion-panel.js";
import "./expansion/accordion.js";
import "./gridlist/grid-list.js";
import "./gridlist/grid-tile.js";
import "./table/table.js";
import "./paginator/paginator.js";
import "./virtualscroll/virtual-scroller.js";
import "./stepper/stepper.js";
import "./stepper/step.js";
import "./autocomplete/autocomplete.js";
import "./tree/tree.js";
import "./tree/tree-item.js";

// ── Typography & brand effects ────────────────────────────────────────────
import "./typography/md-type.js";
import "./effects/webgpu-canvas.js";
import "./card/card.js";
import "./badge/badge.js";
import "./canvas3d/canvas-3d.js";
import "./globe3d/globe-3d.js";
import "./card3d/card-3d.js";

export * from "./snackbar/snackbar.js";
export * from "./loadingindicator/loading-indicator.js";
export * from "./navigationrail/navigation-rail.js";
export * from "./navigationrail/navigation-rail-item.js";
export * from "./appbar/top-app-bar.js";
export * from "./appbar/bottom-app-bar.js";
export * from "./search/search-bar.js";
export * from "./toolbar/md-toolbar.js";
export * from "./sheet/bottom-sheet.js";
export * from "./sheet/side-sheet.js";
export * from "./carousel/carousel.js";
export * from "./carousel/carousel-item.js";
export * from "./buttongroup/button-group.js";
export * from "./fabmenu/fab-menu.js";
export * from "./fabmenu/fab-menu-item.js";
export * from "./datepicker/date-picker.js";
export * from "./timepicker/time-picker.js";
export * from "./layout/md-scaffold.js";
export * from "./layout/md-pane.js";
export * from "./layout/md-list-detail.js";
export * from "./layout/md-supporting-pane.js";
export * from "./tooltip/tooltip.js";
export * from "./expansion/expansion-panel.js";
export * from "./expansion/accordion.js";
export * from "./gridlist/grid-list.js";
export * from "./gridlist/grid-tile.js";
export * from "./table/table.js";
export * from "./paginator/paginator.js";
export * from "./virtualscroll/virtual-scroller.js";
export * from "./stepper/stepper.js";
export * from "./stepper/step.js";
export * from "./autocomplete/autocomplete.js";
export * from "./tree/tree.js";
export * from "./tree/tree-item.js";
export * from "./typography/md-type.js";
export * from "./effects/webgpu-canvas.js";
export * from "./card/card.js";
export * from "./badge/badge.js";
export * from "./canvas3d/canvas-3d.js";
export * from "./globe3d/globe-3d.js";
export * from "./card3d/card-3d.js";

// ── MUI parity (data display / feedback / surfaces / overlays) ─────────────
export * from "./avatar/avatar.js";
export * from "./avatar/avatar-group.js";
export * from "./alert/alert.js";
export * from "./skeleton/skeleton.js";
export * from "./breadcrumbs/breadcrumbs.js";
export * from "./link/link.js";
export * from "./surface/surface.js";
export * from "./backdrop/backdrop.js";
export * from "./popover/popover.js";
export * from "./rating/rating.js";

// ── MUI X parity (charts + date range) ────────────────────────────────────
export * from "./charts/line-chart.js";
export * from "./charts/bar-chart.js";
export * from "./charts/pie-chart.js";
export * from "./charts/area-chart.js";
export * from "./charts/sparkline.js";
export * from "./charts/gauge.js";
export * from "./datepicker/date-range-picker.js";

// ── Coverage closure (MUI MobileStepper, MUI X scatter/radar/datetime/scheduler) ──
export * from "./mobilestepper/mobile-stepper.js";
export * from "./charts/scatter-chart.js";
export * from "./charts/radar-chart.js";
export * from "./datepicker/date-time-picker.js";
export * from "./scheduler/scheduler.js";

// ── labs/gb unified components (barrel parity so bundle consumers reach them) ──
export * from "./labs/gb/components/button/md-button.js";
export * from "./labs/gb/components/menu/md-menu-group.js";
export * from "./labs/gb/components/splitbutton/md-split-button.js";
export * from "./labs/item/item.js";
