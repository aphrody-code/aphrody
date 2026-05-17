// SPDX-License-Identifier: Apache-2.0
import { $ } from "bun";
import { readdir } from "node:fs/promises";
import { join, extname } from "node:path";

// -------------------------------------------------------------
// Aphrody Asset Optimization Workflow (2026 Edition)
// -------------------------------------------------------------
// This script utilizes the fastest modern CLIs for asset compression:
// 1. oxipng (Rust) - For lossless PNG compression.
// 2. svgo (Node) - For intelligent SVG minification.
// 3. lightningcss (Rust) - For lightning-fast CSS minification.
// 4. sharp-cli (C++/libvips) - For modern image format transcoding (AVIF/JXL).

const IGNORED_DIRS = new Set([
  "node_modules", ".git", "dist", "build", "target", "out",
  "test_outputs", "opt", "docs", "var", ".vscode", ".idea", ".cursor"
]);

async function walkDir(dir: string): Promise<string[]> {
  try {
    const dirents = await readdir(dir, { withFileTypes: true });
    const files = await Promise.all(
      dirents.map((dirent) => {
        if (dirent.isDirectory()) {
          if (IGNORED_DIRS.has(dirent.name)) return [];
          return walkDir(join(dir, dirent.name));
        }
        return join(dir, dirent.name);
      })
    );
    return Array.prototype.concat(...files);
  } catch (error) {
    return []; // Ignore if dir doesn't exist
  }
}

async function optimizePNG(file: string) {
  console.log(`[oxipng] Optimizing ${file}...`);
  // -o 4 = max compression, --strip safe = remove metadata, -a = optimize alpha
  await $`oxipng -o 4 --strip safe -a "${file}"`;
}

async function optimizeSVG(file: string) {
  console.log(`[svgo] Optimizing ${file}...`);
  await $`svgo --multipass "${file}"`;
}

async function convertToAVIF(file: string) {
  const outputFile = file.replace(/\.(png|jpg|jpeg)$/i, '.avif');
  console.log(`[sharp] Transcoding ${file} to AVIF...`);
  await $`sharp -i "${file}" -o "${outputFile}"`;
}

async function optimizeCSS(file: string) {
  console.log(`[lightningcss] Minifying ${file}...`);
  await $`lightningcss --minify --bundle --targets ">= 0.25%" "${file}" -o "${file}"`;
}

async function runWorkflow() {
  console.log("🚀 Starting Aphrody Asset Optimization Workflow...");

  const filesToOptimize = await walkDir(process.cwd());

  for (const file of filesToOptimize) {
    const ext = extname(file).toLowerCase();

    try {
      if (ext === ".png") {
        await optimizePNG(file);
        // Optional: Generate modern AVIF variant for the web
        await convertToAVIF(file);
      } else if (ext === ".svg") {
        await optimizeSVG(file);
      } else if (ext === ".css") {
        await optimizeCSS(file);
      }
    } catch (err) {
      console.error(`❌ Failed to optimize ${file}:`, err);
    }
  }

  console.log("✅ Asset Optimization Complete!");
}

runWorkflow();
