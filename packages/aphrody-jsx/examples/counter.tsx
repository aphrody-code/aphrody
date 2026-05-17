/** @license SPDX-License-Identifier: Apache-2.0 */

// useInput-driven counter. Press + to increment, - to decrement, q to exit.
// Run with: bun run examples/counter.tsx

import { useState } from "react";
import { render, Box, Text, useApp, useInput } from "../src/index.ts";

function Counter() {
  const { exit } = useApp();
  const [count, setCount] = useState(0);

  useInput((input, key) => {
    if (input === "q" || key.escape) exit();
    if (input === "+" || input === "=") setCount((c) => c + 1);
    if (input === "-") setCount((c) => c - 1);
  });

  return (
    <Box flexDirection="column" padding={1}>
      <Text bold color="primary">aphrody-jsx counter</Text>
      <Text>Count: {String(count)}</Text>
      <Text dimColor>+ to increment, - to decrement, q to exit</Text>
    </Box>
  );
}

const instance = render(<Counter />);
await instance.waitUntilExit();
