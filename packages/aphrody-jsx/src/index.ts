/** @license SPDX-License-Identifier: Apache-2.0 */

// Public surface of @aphrody/jsx. Re-exports the render entry point, the host
// components, and the hooks. Implementation lives in sibling modules.

export { render, type Instance } from "./render.ts";

export { Box } from "./components/Box.tsx";
export { Text } from "./components/Text.tsx";
export { Newline, type NewlineProps } from "./components/Newline.tsx";
export { Static } from "./components/Static.tsx";
export { Transform } from "./components/Transform.tsx";
export { Spacer } from "./components/Spacer.tsx";

export { useApp, type AppContextValue } from "./hooks/use-app.ts";
export { useStdout, type StdoutHandle } from "./hooks/use-stdout.ts";
export { useInput, emptyKey } from "./hooks/use-input.ts";
export { useFocus, type UseFocusOptions, type UseFocusReturn } from "./hooks/use-focus.ts";
export { useFocusManager, type FocusManager } from "./hooks/use-focus-manager.ts";
export { useWindowSize } from "./hooks/use-window-size.ts";

export type {
  BoxProps,
  TextProps,
  StaticProps,
  TransformProps,
  Color,
  M3TokenName,
  FlexDirection,
  AlignItems,
  JustifyContent,
  BorderStyle,
  InputHandler,
  InputKey,
  InputModifiers,
  RenderOptions,
  WindowSize,
  JsonNode,
  JsonElement,
  JsonTextNode,
  JsonPatchOp,
  OscOpcode,
} from "./types.ts";

export {
  encode as encodeOsc,
  parseFrame as parseOscFrame,
  OscFrameParser,
  type OscFrame,
  type OscSink,
} from "./osc.ts";

export { resolveColor, encodeColor, isM3Token, M3_FALLBACK_DARK } from "./m3-tokens.ts";
