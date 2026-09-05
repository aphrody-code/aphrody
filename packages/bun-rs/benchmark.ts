// SPDX-License-Identifier: Apache-2.0

import { dlopen, ptr } from "bun:ffi";
import { join } from "node:path";

// Locate the compiled .so library
const libPath = join(import.meta.dir, "./target/release/libbun_rs.so");

console.log(`Loading native library from: ${libPath}\n`);

// Load the dynamic library using Bun's FFI
const { symbols: lib } = dlopen(libPath, {
  bun_rs_version: {
    args: [],
    returns: "cstring",
  },
  bun_rs_add: {
    args: ["i32", "i32"],
    returns: "i32",
  },
  bun_rs_count_char: {
    args: ["ptr", "usize", "u8"],
    returns: "usize",
  },
  bun_rs_find_bytes: {
    args: ["ptr", "usize", "ptr", "usize"],
    returns: "isize",
  },
});

// 1. Basic functionality check
console.log(`[FFI] Version: ${lib.bun_rs_version()}`);
console.log(`[FFI] Add (40 + 2): ${lib.bun_rs_add(40, 2)}`);

// 2. High-performance count_char FFI
const text = "A very long string containing some characters to count, with a lot of words.";
const textBuffer = Buffer.from(text);
const charToCount = "o".charCodeAt(0);

const count = lib.bun_rs_count_char(ptr(textBuffer), textBuffer.length, charToCount);
console.log(`[FFI] Count occurrences of 'o': ${count} (expected: 8)`);

// 3. Substring find_bytes FFI
const needle = "characters";
const needleBuffer = Buffer.from(needle);

const index = lib.bun_rs_find_bytes(
  ptr(textBuffer),
  textBuffer.length,
  ptr(needleBuffer),
  needleBuffer.length,
);
console.log(`[FFI] Index of "${needle}": ${index} (expected: 35)`);

// 4. Performance Benchmark: FFI vs Pure JS
const ITERATIONS = 10000000;
console.log(`\nRunning benchmark with ${ITERATIONS.toLocaleString()} iterations...`);

const startFFI = performance.now();
for (let i = 0; i < ITERATIONS; i++) {
  lib.bun_rs_add(100, i);
}
const durationFFI = performance.now() - startFFI;
console.log(
  `-> Rust FFI bun_rs_add: ${durationFFI.toFixed(2)}ms (${(ITERATIONS / durationFFI / 1000).toFixed(2)}M ops/sec)`,
);

const jsAdd = (a, b) => a + b;
const startJS = performance.now();
for (let i = 0; i < ITERATIONS; i++) {
  jsAdd(100, i);
}
const durationJS = performance.now() - startJS;
console.log(
  `-> Pure JS add: ${durationJS.toFixed(2)}ms (${(ITERATIONS / durationJS / 1000).toFixed(2)}M ops/sec)`,
);
