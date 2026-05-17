<!-- SPDX-License-Identifier: Apache-2.0 -->

# @aphrody/jsx

Bun-native React reconciler for `aphrody-terminal`. Instead of writing ANSI to
stdout the way Ink does, this renderer emits structured `aphrody-jsx-*` OSC
sequences. The Rust-side `aphrody-terminal-vt` / `aphrody-terminal-wasm` stack
consumes those frames and computes layout via `taffy`, themes via M3 dynamic
color tokens, and renders to a native pty or to a WebAssembly DOM viewport.

This is Layer B of the Ink-fusion strategy described in
`docs/design/aphrody-terminal-spec.md`.

## Quick start

```tsx
import { render, Box, Text, useInput, useApp } from "@aphrody/jsx";

function App() {
  const { exit } = useApp();
  useInput((input) => { if (input === "q") exit(); });
  return (
    <Box flexDirection="column" padding={1}>
      <Text bold color="primary">aphrody-jsx demo</Text>
      <Text dimColor>Press q to exit</Text>
    </Box>
  );
}

render(<App />, { target: "pty" });
```

Run it with `bun run examples/hello.tsx`.

## API

| Export                | Kind        | Notes                                        |
|-----------------------|-------------|----------------------------------------------|
| `render`              | function    | Mounts a React element, returns `Instance`.  |
| `Box`                 | component   | Flex container (taffy-equivalent props).     |
| `Text`                | component   | Styled inline text node.                     |
| `Newline`             | component   | Soft line break.                             |
| `Static`              | component   | Append-only region, never re-rerendered.    |
| `Transform`           | component   | `(children) => children` function child.     |
| `Spacer`              | component   | Equivalent to `flexGrow: 1`.                 |
| `useInput`            | hook        | Subscribes to keyboard input frames.         |
| `useApp`              | hook        | `{ exit, waitUntilExit }`.                   |
| `useStdout`           | hook        | Raw passthrough writer.                      |
| `useFocus`            | hook        | Focus state for the calling node.            |
| `useFocusManager`     | hook        | `focusNext` / `focusPrevious` etc.           |
| `useWindowSize`       | hook        | `{ columns, rows }` from the terminal.       |

## Comparison vs Ink

| Aspect              | Ink (`vadimdemedes/ink`)              | `@aphrody/jsx`                                |
|---------------------|---------------------------------------|-----------------------------------------------|
| Runtime             | Node only                             | Bun only                                      |
| JSX transform       | Babel / SWC pre-step                  | Native `bun --jsx react-jsx`                  |
| Layout engine       | Yoga (WASM, in-process JS)            | Rust `taffy`, terminal-side                  |
| Output channel      | ANSI directly to stdout               | Structured `aphrody-jsx-*` OSC frames        |
| Theming             | ANSI color codes                      | M3 dynamic-color tokens                      |
| WASM target         | Not supported                         | Same source renders in `aphrody-terminal-wasm` |
| Native bindings     | `react-reconciler`, Yoga WASM         | `react-reconciler` only                      |

## OSC envelope

Every mount / update / unmount emits one frame:

```
\x1b]aphrody-jsx-mount;<id>;<base64-json-tree>\x07
\x1b]aphrody-jsx-update;<id>;<base64-json-patch>\x07
\x1b]aphrody-jsx-unmount;<id>\x07
```

Terminal-side echoes back input frames:

```
\x1b]aphrody-jsx-input;<id>;<base64-json-event>\x07
\x1b]aphrody-jsx-window-size;<cols>;<rows>\x07
\x1b]aphrody-jsx-focus;<id>;<true|false>\x07
```

Layout, paint, hit-testing, and theming all happen on the Rust side.
