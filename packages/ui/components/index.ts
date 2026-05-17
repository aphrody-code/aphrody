/**
 * @aphrody-code/ui — Material Design 3 component surface.
 *
 * Migration tracker (shadcn → Material Web 3 wrapper):
 *   [x] button     — packages/ui/components/button.tsx + button.css
 *   [ ] input      — md-outlined-text-field / md-filled-text-field
 *   [ ] select     — md-select / md-menu
 *   [ ] checkbox   — md-checkbox
 *   [ ] radio      — md-radio
 *   [ ] switch     — md-switch
 *   [ ] card       — md-card
 *   [ ] dialog     — md-dialog
 *   [ ] tabs       — md-tabs / md-tab
 *   [ ] navigation — md-navigation-bar / md-navigation-rail / md-navigation-drawer
 *   [ ] badge      — md-badge
 *   [ ] avatar     — md-avatar (+ md-icon)
 *   [ ] progress   — md-linear-progress / md-circular-progress
 *   [ ] snackbar   — md-snackbar
 *
 * Each component lives in its own .tsx + .css pair and is re-exported here.
 * CSS bundles are imported as side-effects so consumers only need to import
 * the component to pull its theme.
 */

import "./button.css";

export { Button, default as ButtonDefault } from "./button.tsx";
export type {
	ButtonProps,
	ButtonSize,
	ButtonType,
	ButtonVariant,
} from "./button.tsx";
