// SPDX-License-Identifier: Apache-2.0
//! React wrappers for Material Web (`md-*`) custom elements.
//
// Each export wraps a Material Web `LitElement` as a React component via
// `@lit/react`'s `createComponent`, mapping React props to custom-element
// properties and DOM events to React handler props. Theme with the aphrody
// M3 tokens (`--md-sys-color-*`) — import `@aphrody/m3-react/theme.css` or the
// output of `aphrody design tokens --fusion`.
//
// Grouped by the M3 component taxonomy (action / selection / text input /
// communication / navigation / containment).

import * as React from 'react';
import {createComponent} from '@lit/react';

// --- Action: buttons -------------------------------------------------------
import {MdElevatedButton} from '@material/web/button/elevated-button.js';
import {MdFilledButton} from '@material/web/button/filled-button.js';
import {MdFilledTonalButton} from '@material/web/button/filled-tonal-button.js';
import {MdOutlinedButton} from '@material/web/button/outlined-button.js';
import {MdTextButton} from '@material/web/button/text-button.js';
import {MdIconButton} from '@material/web/iconbutton/icon-button.js';
import {MdFilledIconButton} from '@material/web/iconbutton/filled-icon-button.js';
import {MdFab} from '@material/web/fab/fab.js';
import {MdBrandedFab} from '@material/web/fab/branded-fab.js';

// --- Selection -------------------------------------------------------------
import {MdCheckbox} from '@material/web/checkbox/checkbox.js';
import {MdRadio} from '@material/web/radio/radio.js';
import {MdSwitch} from '@material/web/switch/switch.js';
import {MdSlider} from '@material/web/slider/slider.js';
import {MdAssistChip} from '@material/web/chips/assist-chip.js';
import {MdFilterChip} from '@material/web/chips/filter-chip.js';
import {MdInputChip} from '@material/web/chips/input-chip.js';
import {MdSuggestionChip} from '@material/web/chips/suggestion-chip.js';
import {MdChipSet} from '@material/web/chips/chip-set.js';

// --- Text input ------------------------------------------------------------
import {MdFilledTextField} from '@material/web/textfield/filled-text-field.js';
import {MdOutlinedTextField} from '@material/web/textfield/outlined-text-field.js';

// --- Communication ---------------------------------------------------------
import {MdDialog} from '@material/web/dialog/dialog.js';
import {MdCircularProgress} from '@material/web/progress/circular-progress.js';
import {MdLinearProgress} from '@material/web/progress/linear-progress.js';

// --- Navigation ------------------------------------------------------------
import {MdTabs} from '@material/web/tabs/tabs.js';
import {MdPrimaryTab} from '@material/web/tabs/primary-tab.js';
import {MdSecondaryTab} from '@material/web/tabs/secondary-tab.js';
import {MdMenu} from '@material/web/menu/menu.js';
import {MdMenuItem} from '@material/web/menu/menu-item.js';

// --- Containment -----------------------------------------------------------
import {MdList} from '@material/web/list/list.js';
import {MdListItem} from '@material/web/list/list-item.js';
import {MdDivider} from '@material/web/divider/divider.js';

// --- Aphrody M3 extensions -------------------------------------------------
import {MdSnackbar} from '@material/web/snackbar/snackbar.js';
import {MdLoadingIndicator} from '@material/web/loadingindicator/loading-indicator.js';
import {MdNavigationRail} from '@material/web/navigationrail/navigation-rail.js';
import {MdNavigationRailItem} from '@material/web/navigationrail/navigation-rail-item.js';
import {MdTopAppBar} from '@material/web/appbar/top-app-bar.js';
import {MdBottomAppBar} from '@material/web/appbar/bottom-app-bar.js';
import {MdSearchBar} from '@material/web/search/search-bar.js';
import {MdToolbar} from '@material/web/toolbar/md-toolbar.js';
import {MdBottomSheet} from '@material/web/sheet/bottom-sheet.js';
import {MdSideSheet} from '@material/web/sheet/side-sheet.js';
import {MdCarousel} from '@material/web/carousel/carousel.js';
import {MdCarouselItem} from '@material/web/carousel/carousel-item.js';
import {MdButtonGroup} from '@material/web/buttongroup/button-group.js';
import {MdFabMenu} from '@material/web/fabmenu/fab-menu.js';
import {MdFabMenuItem} from '@material/web/fabmenu/fab-menu-item.js';
import {MdDatePicker} from '@material/web/datepicker/date-picker.js';
import {MdTimePicker} from '@material/web/timepicker/time-picker.js';
import {MdScaffold} from '@material/web/layout/md-scaffold.js';
import {MdPane} from '@material/web/layout/md-pane.js';
import {MdListDetail} from '@material/web/layout/md-list-detail.js';
import {MdSupportingPane} from '@material/web/layout/md-supporting-pane.js';
import {MdType} from '@material/web/typography/md-type.js';
import {MdWebgpuCanvas} from '@material/web/effects/webgpu-canvas.js';

// --- Angular-Material parity ----------------------------------------------
import {MdTooltip} from '@material/web/tooltip/tooltip.js';
import {MdExpansionPanel} from '@material/web/expansion/expansion-panel.js';
import {MdAccordion} from '@material/web/expansion/accordion.js';
import {MdGridList} from '@material/web/gridlist/grid-list.js';
import {MdGridTile} from '@material/web/gridlist/grid-tile.js';
import {MdTable} from '@material/web/table/table.js';
import {MdPaginator} from '@material/web/paginator/paginator.js';
import {MdVirtualScroller} from '@material/web/virtualscroll/virtual-scroller.js';
import {MdStepper} from '@material/web/stepper/stepper.js';
import {MdStep} from '@material/web/stepper/step.js';
import {MdAutocomplete} from '@material/web/autocomplete/autocomplete.js';
import {MdTree} from '@material/web/tree/tree.js';
import {MdTreeItem} from '@material/web/tree/tree-item.js';

const react = React;

// Action ---------------------------------------------------------------------
export const ElevatedButton = createComponent({tagName: 'md-elevated-button', elementClass: MdElevatedButton, react});
export const FilledButton = createComponent({tagName: 'md-filled-button', elementClass: MdFilledButton, react});
export const FilledTonalButton = createComponent({tagName: 'md-filled-tonal-button', elementClass: MdFilledTonalButton, react});
export const OutlinedButton = createComponent({tagName: 'md-outlined-button', elementClass: MdOutlinedButton, react});
export const TextButton = createComponent({tagName: 'md-text-button', elementClass: MdTextButton, react});
export const IconButton = createComponent({tagName: 'md-icon-button', elementClass: MdIconButton, react});
export const FilledIconButton = createComponent({tagName: 'md-filled-icon-button', elementClass: MdFilledIconButton, react});
export const Fab = createComponent({tagName: 'md-fab', elementClass: MdFab, react});
export const BrandedFab = createComponent({tagName: 'md-branded-fab', elementClass: MdBrandedFab, react});

// Selection (form controls expose change/input) ------------------------------
export const Checkbox = createComponent({tagName: 'md-checkbox', elementClass: MdCheckbox, react, events: {onChange: 'change', onInput: 'input'}});
export const Radio = createComponent({tagName: 'md-radio', elementClass: MdRadio, react, events: {onChange: 'change'}});
export const Switch = createComponent({tagName: 'md-switch', elementClass: MdSwitch, react, events: {onChange: 'change'}});
export const Slider = createComponent({tagName: 'md-slider', elementClass: MdSlider, react, events: {onChange: 'change', onInput: 'input'}});
export const AssistChip = createComponent({tagName: 'md-assist-chip', elementClass: MdAssistChip, react});
export const FilterChip = createComponent({tagName: 'md-filter-chip', elementClass: MdFilterChip, react});
export const InputChip = createComponent({tagName: 'md-input-chip', elementClass: MdInputChip, react});
export const SuggestionChip = createComponent({tagName: 'md-suggestion-chip', elementClass: MdSuggestionChip, react});
export const ChipSet = createComponent({tagName: 'md-chip-set', elementClass: MdChipSet, react});

// Text input -----------------------------------------------------------------
export const FilledTextField = createComponent({tagName: 'md-filled-text-field', elementClass: MdFilledTextField, react, events: {onChange: 'change', onInput: 'input'}});
export const OutlinedTextField = createComponent({tagName: 'md-outlined-text-field', elementClass: MdOutlinedTextField, react, events: {onChange: 'change', onInput: 'input'}});

// Communication --------------------------------------------------------------
export const Dialog = createComponent({tagName: 'md-dialog', elementClass: MdDialog, react, events: {onOpen: 'open', onClose: 'close', onCancel: 'cancel'}});
export const CircularProgress = createComponent({tagName: 'md-circular-progress', elementClass: MdCircularProgress, react});
export const LinearProgress = createComponent({tagName: 'md-linear-progress', elementClass: MdLinearProgress, react});

// Navigation -----------------------------------------------------------------
export const Tabs = createComponent({tagName: 'md-tabs', elementClass: MdTabs, react, events: {onChange: 'change'}});
export const PrimaryTab = createComponent({tagName: 'md-primary-tab', elementClass: MdPrimaryTab, react});
export const SecondaryTab = createComponent({tagName: 'md-secondary-tab', elementClass: MdSecondaryTab, react});
export const Menu = createComponent({tagName: 'md-menu', elementClass: MdMenu, react, events: {onOpening: 'opening', onClosing: 'closing'}});
export const MenuItem = createComponent({tagName: 'md-menu-item', elementClass: MdMenuItem, react});

// Containment ----------------------------------------------------------------
export const List = createComponent({tagName: 'md-list', elementClass: MdList, react});
export const ListItem = createComponent({tagName: 'md-list-item', elementClass: MdListItem, react});
export const Divider = createComponent({tagName: 'md-divider', elementClass: MdDivider, react});

// === Aphrody M3 extensions =================================================
// New components completing the M3 catalog + adaptive layout + Google Sans Flex
// typography + WebGPU brand effects (imports declared at the top of the file).

export const Snackbar = createComponent({tagName: 'md-snackbar', elementClass: MdSnackbar, react, events: {onOpening: 'opening', onOpened: 'opened', onClosing: 'closing', onClosed: 'closed'}});
export const LoadingIndicator = createComponent({tagName: 'md-loading-indicator', elementClass: MdLoadingIndicator, react});
export const NavigationRail = createComponent({tagName: 'md-navigation-rail', elementClass: MdNavigationRail, react, events: {onChange: 'navigation-rail:change'}});
export const NavigationRailItem = createComponent({tagName: 'md-navigation-rail-item', elementClass: MdNavigationRailItem, react});
export const TopAppBar = createComponent({tagName: 'md-top-app-bar', elementClass: MdTopAppBar, react});
export const BottomAppBar = createComponent({tagName: 'md-bottom-app-bar', elementClass: MdBottomAppBar, react});
export const SearchBar = createComponent({tagName: 'md-search-bar', elementClass: MdSearchBar, react, events: {onInput: 'input', onSearch: 'search', onSearchOpen: 'search:open', onSearchClose: 'search:close'}});
export const Toolbar = createComponent({tagName: 'md-toolbar', elementClass: MdToolbar, react});
export const BottomSheet = createComponent({tagName: 'md-bottom-sheet', elementClass: MdBottomSheet, react, events: {onOpening: 'bottom-sheet:opening', onOpened: 'bottom-sheet:opened', onClosing: 'bottom-sheet:closing', onClosed: 'bottom-sheet:closed'}});
export const SideSheet = createComponent({tagName: 'md-side-sheet', elementClass: MdSideSheet, react, events: {onOpening: 'side-sheet:opening', onOpened: 'side-sheet:opened', onClosing: 'side-sheet:closing', onClosed: 'side-sheet:closed'}});
export const Carousel = createComponent({tagName: 'md-carousel', elementClass: MdCarousel, react});
export const CarouselItem = createComponent({tagName: 'md-carousel-item', elementClass: MdCarouselItem, react});
export const ButtonGroup = createComponent({tagName: 'md-button-group', elementClass: MdButtonGroup, react, events: {onChange: 'button-group:change'}});
export const FabMenu = createComponent({tagName: 'md-fab-menu', elementClass: MdFabMenu, react, events: {onOpen: 'fab-menu:open', onClose: 'fab-menu:close'}});
export const FabMenuItem = createComponent({tagName: 'md-fab-menu-item', elementClass: MdFabMenuItem, react});
export const DatePicker = createComponent({tagName: 'md-date-picker', elementClass: MdDatePicker, react, events: {onChange: 'date-picker:change'}});
export const TimePicker = createComponent({tagName: 'md-time-picker', elementClass: MdTimePicker, react, events: {onChange: 'time-picker:change'}});
export const Scaffold = createComponent({tagName: 'md-scaffold', elementClass: MdScaffold, react, events: {onSizeClassChange: 'scaffold:size-class-change'}});
export const Pane = createComponent({tagName: 'md-pane', elementClass: MdPane, react});
export const ListDetail = createComponent({tagName: 'md-list-detail', elementClass: MdListDetail, react, events: {onShowingChange: 'list-detail:showing-change'}});
export const SupportingPane = createComponent({tagName: 'md-supporting-pane', elementClass: MdSupportingPane, react});
export const TypeText = createComponent({tagName: 'md-type', elementClass: MdType, react});
export const WebgpuCanvas = createComponent({tagName: 'md-webgpu-canvas', elementClass: MdWebgpuCanvas, react});

// Angular-Material parity --------------------------------------------------
export const Tooltip = createComponent({tagName: 'md-tooltip', elementClass: MdTooltip, react});
export const ExpansionPanel = createComponent({tagName: 'md-expansion-panel', elementClass: MdExpansionPanel, react, events: {onToggle: 'expansion:toggle'}});
export const Accordion = createComponent({tagName: 'md-accordion', elementClass: MdAccordion, react});
export const GridList = createComponent({tagName: 'md-grid-list', elementClass: MdGridList, react});
export const GridTile = createComponent({tagName: 'md-grid-tile', elementClass: MdGridTile, react});
export const Table = createComponent({tagName: 'md-table', elementClass: MdTable, react, events: {onSort: 'table:sort', onSelectionChange: 'table:selection-change'}});
export const Paginator = createComponent({tagName: 'md-paginator', elementClass: MdPaginator, react, events: {onPage: 'paginator:page'}});
export const VirtualScroller = createComponent({tagName: 'md-virtual-scroller', elementClass: MdVirtualScroller, react, events: {onRange: 'virtual-scroll:range'}});
export const Stepper = createComponent({tagName: 'md-stepper', elementClass: MdStepper, react, events: {onChange: 'stepper:change'}});
export const Step = createComponent({tagName: 'md-step', elementClass: MdStep, react});
export const Autocomplete = createComponent({tagName: 'md-autocomplete', elementClass: MdAutocomplete, react, events: {onSelect: 'autocomplete:select', onInput: 'input'}});
export const Tree = createComponent({tagName: 'md-tree', elementClass: MdTree, react, events: {onSelect: 'tree:select'}});
export const TreeItem = createComponent({tagName: 'md-tree-item', elementClass: MdTreeItem, react, events: {onToggle: 'tree-item:toggle'}});

// Interaction primitives (View Transitions, scroll reveal, lazy image, Gemini
// thinking/streaming) distilled from design.google + gemini.google.com.
export * from './interactions.tsx';
