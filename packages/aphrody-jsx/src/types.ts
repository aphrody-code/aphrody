/** @license SPDX-License-Identifier: Apache-2.0 */

// Public type surface for @aphrody/jsx. Mirrors the Ink prop names where
// applicable so existing Ink components only need an import-path change.

import type { ReactNode } from "react";

export type FlexDirection = "row" | "column" | "row-reverse" | "column-reverse";

export type AlignItems =
  | "flex-start"
  | "flex-end"
  | "center"
  | "stretch"
  | "baseline";

export type JustifyContent =
  | "flex-start"
  | "flex-end"
  | "center"
  | "space-between"
  | "space-around"
  | "space-evenly";

export type BorderStyle =
  | "single"
  | "double"
  | "round"
  | "bold"
  | "singleDouble"
  | "doubleSingle"
  | "classic";

// Colors accept either a literal M3 token name or a CSS-style hex / rgb string.
// Token names are resolved terminal-side via the dynamic M3 palette;
// arbitrary strings are passed through verbatim.
export type M3TokenName =
  | "primary"
  | "onPrimary"
  | "primaryContainer"
  | "onPrimaryContainer"
  | "secondary"
  | "onSecondary"
  | "secondaryContainer"
  | "onSecondaryContainer"
  | "tertiary"
  | "onTertiary"
  | "tertiaryContainer"
  | "onTertiaryContainer"
  | "error"
  | "onError"
  | "errorContainer"
  | "onErrorContainer"
  | "background"
  | "onBackground"
  | "surface"
  | "onSurface"
  | "surfaceVariant"
  | "onSurfaceVariant"
  | "outline"
  | "outlineVariant"
  | "inverseSurface"
  | "inverseOnSurface"
  | "inversePrimary"
  | "shadow"
  | "scrim";

export type Color = M3TokenName | (string & {});

export interface BoxProps {
  flexDirection?: FlexDirection;
  alignItems?: AlignItems;
  justifyContent?: JustifyContent;
  flexGrow?: number;
  flexShrink?: number;
  flexBasis?: number | string;
  padding?: number;
  paddingX?: number;
  paddingY?: number;
  paddingTop?: number;
  paddingRight?: number;
  paddingBottom?: number;
  paddingLeft?: number;
  margin?: number;
  marginX?: number;
  marginY?: number;
  marginTop?: number;
  marginRight?: number;
  marginBottom?: number;
  marginLeft?: number;
  width?: number | string;
  height?: number | string;
  minWidth?: number | string;
  minHeight?: number | string;
  gap?: number;
  borderStyle?: BorderStyle;
  borderColor?: Color;
  children?: ReactNode;
}

export interface TextProps {
  color?: Color;
  backgroundColor?: Color;
  bold?: boolean;
  italic?: boolean;
  underline?: boolean;
  strikethrough?: boolean;
  inverse?: boolean;
  dimColor?: boolean;
  wrap?: "wrap" | "truncate" | "truncate-start" | "truncate-middle" | "truncate-end";
  children?: ReactNode;
}

export interface StaticProps<T> {
  items: ReadonlyArray<T>;
  children: (item: T, index: number) => ReactNode;
}

export interface TransformProps {
  transform: (children: string) => string;
  children?: ReactNode;
}

export interface RenderOptions {
  target?: "auto" | "pty" | "ws";
  wsUrl?: string;
  // Optional output stream override (for tests). Must accept Uint8Array or string.
  output?: { write(chunk: Uint8Array | string): unknown };
  // Optional input stream override (for tests).
  input?: AsyncIterable<Uint8Array | string>;
  // Whether to install a SIGINT handler that calls exit().
  exitOnCtrlC?: boolean;
}

export interface InputModifiers {
  ctrl: boolean;
  meta: boolean;
  shift: boolean;
}

export interface InputKey {
  upArrow: boolean;
  downArrow: boolean;
  leftArrow: boolean;
  rightArrow: boolean;
  return: boolean;
  escape: boolean;
  tab: boolean;
  backspace: boolean;
  delete: boolean;
  pageUp: boolean;
  pageDown: boolean;
  home: boolean;
  end: boolean;
  ctrl: boolean;
  shift: boolean;
  meta: boolean;
}

export type InputHandler = (input: string, key: InputKey) => void;

export interface WindowSize {
  columns: number;
  rows: number;
}

// JSON tree node format sent inside aphrody-jsx-mount frames.
export type JsonNode = JsonElement | JsonTextNode;

export interface JsonElement {
  type: "Box" | "Text" | "Newline" | "Static" | "Transform" | "Spacer" | "Root";
  id: string;
  props: Record<string, unknown>;
  children: JsonNode[];
}

export interface JsonTextNode {
  type: "TEXT_NODE";
  id: string;
  value: string;
}

// Patch operations sent inside aphrody-jsx-update frames.
export type JsonPatchOp =
  | { op: "replace-prop"; id: string; key: string; value: unknown }
  | { op: "remove-prop"; id: string; key: string }
  | { op: "replace-text"; id: string; value: string }
  | { op: "insert"; parentId: string; beforeId: string | null; node: JsonNode }
  | { op: "remove"; id: string };

export type OscOpcode =
  | "mount"
  | "update"
  | "unmount"
  | "input"
  | "window-size"
  | "focus";
