/** @license SPDX-License-Identifier: Apache-2.0 */

// Minimal hello-world example. Run with: bun run examples/hello.tsx
// Emits a single aphrody-jsx-mount OSC frame to stdout containing the tree:
//   <Box padding=1 flexDirection=column>
//     <Text bold color=primary>Hello aphrody-jsx</Text>
//     <Text dimColor>Layer B — Bun-native JSX over OSC</Text>
//   </Box>

import { render, Box, Text } from "../src/index.ts";

function App() {
  return (
    <Box flexDirection="column" padding={1}>
      <Text bold color="primary">Hello aphrody-jsx</Text>
      <Text dimColor>Layer B — Bun-native JSX over OSC</Text>
    </Box>
  );
}

const instance = render(<App />, { target: "pty", exitOnCtrlC: false });

// Allow the reconciler to flush, then tear down cleanly so the process exits.
setTimeout(() => instance.unmount(), 50);
