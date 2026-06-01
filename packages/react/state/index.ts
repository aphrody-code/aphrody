// SPDX-License-Identifier: Apache-2.0

/**
 * Hoisted state primitives for Material Design 3 React components.
 *
 * Currently exposes {@link useTextFieldState} / {@link WebTextFieldState}: the
 * Web counterpart of Jetpack Compose `TextFieldState` / `rememberTextFieldState`,
 * an observable, transactional text + selection container for driving
 * `<md-outlined-text-field>` / `<md-filled-text-field>` without spurious
 * re-renders.
 *
 * @packageDocumentation
 */

export {
  WebTextFieldState,
  useTextFieldState,
  bindTextFieldState,
  syncFromElement,
} from "./use-text-field-state.js";
export type {
  TextFieldSelection,
  TextFieldEditBuffer,
  TextFieldStateListener,
  SelectionCapableElement,
} from "./use-text-field-state.js";
