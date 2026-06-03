// SPDX-License-Identifier: Apache-2.0

import { initAsyncCompiler } from "sass-embedded";
import { compileSassFile } from "./src/index.ts";
import { resolve } from "node:path";

console.log("=== Sass Compiler Benchmark (Dart Sass vs Rust FFI Grass) ===\n");

const PKG = resolve(import.meta.dir, "../material-web");
const targetFile = resolve(PKG, "button/internal/elevated-styles.scss");
const loadPaths = [resolve(PKG, "node_modules"), resolve(PKG, "../../node_modules")];

// Warmup and compilation verification
console.log(`Target file: ${targetFile}\n`);

// 1. Compile with Rust FFI Grass
const t0_grass = performance.now();
const cssGrass = compileSassFile(targetFile, loadPaths, "compressed", true);
const d_grass = performance.now() - t0_grass;
console.log(`[Grass FFI] Compiled in ${d_grass.toFixed(2)}ms`);

// 2. Compile with Dart Sass (sass-embedded)
const compiler = await initAsyncCompiler();
const t0_dart = performance.now();
const resultDart = await compiler.compileAsync(targetFile, {
  style: "compressed",
  loadPaths,
});
const d_dart = performance.now() - t0_dart;
console.log(`[Dart Sass FFI] Compiled in ${d_dart.toFixed(2)}ms`);
await compiler.dispose();

console.log(`\nRatio (Dart Sass / Grass): ${(d_dart / d_grass).toFixed(1)}x faster\n`);

// Running iteration benchmark
const ITERATIONS = 100;
console.log(`Running iteration benchmark (${ITERATIONS} iterations)...`);

// Grass Benchmark
const tStartGrass = performance.now();
for (let i = 0; i < ITERATIONS; i++) {
  compileSassFile(targetFile, loadPaths, "compressed", true);
}
const durationGrass = performance.now() - tStartGrass;
console.log(
  `-> Rust FFI Grass: ${durationGrass.toFixed(2)}ms (${(durationGrass / ITERATIONS).toFixed(2)}ms/compile)`,
);

// Dart Sass Benchmark
const compilerBench = await initAsyncCompiler();
const tStartDart = performance.now();
for (let i = 0; i < ITERATIONS; i++) {
  await compilerBench.compileAsync(targetFile, {
    style: "compressed",
    loadPaths,
  });
}
const durationDart = performance.now() - tStartDart;
await compilerBench.dispose();
console.log(
  `-> Dart Sass: ${durationDart.toFixed(2)}ms (${(durationDart / ITERATIONS).toFixed(2)}ms/compile)`,
);

console.log(
  `\n=== Benchmark Completed! Speedup: ${(durationDart / durationGrass).toFixed(1)}x ===`,
);
