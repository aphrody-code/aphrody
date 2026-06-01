// SPDX-License-Identifier: Apache-2.0

import { dlopen, CString } from "bun:ffi";
import { join } from "path";

const libSuffix =
  process.platform === "win32" ? "dll" : process.platform === "darwin" ? "dylib" : "so";
const libPath = join(import.meta.dir, `../target/release/libbun_rs.${libSuffix}`);

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
    returns: "i64",
  },
  bun_rs_hct_to_argb: {
    args: ["f32", "f32", "f32"],
    returns: "u32",
  },
  bun_rs_argb_to_hct: {
    args: ["u32", "ptr", "ptr", "ptr"],
    returns: "void",
  },
  bun_rs_hct_tones: {
    args: ["f32", "f32", "f32", "ptr"],
    returns: "void",
  },
  bun_rs_derive_scheme: {
    args: ["f32", "f32", "bool", "ptr"],
    returns: "void",
  },
  bun_rs_compile_sass: {
    args: ["ptr", "usize", "ptr", "usize", "i32", "bool", "ptr"],
    returns: "ptr",
  },
  bun_rs_compile_sass_file: {
    args: ["ptr", "usize", "ptr", "usize", "i32", "bool", "ptr"],
    returns: "ptr",
  },
  bun_rs_free_string: {
    args: ["ptr"],
    returns: "void",
  },
  bun_rs_validate_spec: {
    args: ["ptr", "usize"],
    returns: "ptr",
  },
});

export const ROLES = [
  "background",
  "on-background",
  "surface",
  "surface-dim",
  "surface-bright",
  "surface-container-lowest",
  "surface-container-low",
  "surface-container",
  "surface-container-high",
  "surface-container-highest",
  "on-surface",
  "surface-variant",
  "on-surface-variant",
  "inverse-surface",
  "inverse-on-surface",
  "outline",
  "outline-variant",
  "shadow",
  "scrim",
  "surface-tint",
  "primary",
  "on-primary",
  "primary-container",
  "on-primary-container",
  "inverse-primary",
  "secondary",
  "on-secondary",
  "secondary-container",
  "on-secondary-container",
  "tertiary",
  "on-tertiary",
  "tertiary-container",
  "on-tertiary-container",
  "error",
  "on-error",
  "error-container",
  "on-error-container",
  "primary-fixed",
  "primary-fixed-dim",
  "on-primary-fixed",
  "on-primary-fixed-variant",
  "secondary-fixed",
  "secondary-fixed-dim",
  "on-secondary-fixed",
  "on-secondary-fixed-variant",
  "tertiary-fixed",
  "tertiary-fixed-dim",
  "on-tertiary-fixed",
  "on-tertiary-fixed-variant",
] as const;

export type ColorRoleMap = Record<string, string>;

/** Get the native library version. */
pub_fn_version();
function pub_fn_version(): string {
  return lib.bun_rs_version().toString();
}
export { pub_fn_version as version };

/** Run native addition to test round-trip FFI overhead. */
pub_fn_add(1, 2);
function pub_fn_add(a: number, b: number): number {
  return lib.bun_rs_add(a, b);
}
export { pub_fn_add as add };

/** SIMD count char using memchr. */
pub_fn_countChar("abc", "b");
function pub_fn_countChar(text: string, char: string): number {
  const buf = Buffer.from(text);
  return Number(lib.bun_rs_count_char(buf, buf.length, char.charCodeAt(0)));
}
export { pub_fn_countChar as countChar };

/** SIMD substring search using memchr. */
pub_fn_findBytes("abc", "b");
function pub_fn_findBytes(haystack: string, needle: string): number {
  const hBuf = Buffer.from(haystack);
  const nBuf = Buffer.from(needle);
  return Number(lib.bun_rs_find_bytes(hBuf, hBuf.length, nBuf, nBuf.length));
}
export { pub_fn_findBytes as findBytes };

/** Convert HCT to packed ARGB u32. */
pub_fn_hctToArgb(277.0, 40.0, 40.0);
function pub_fn_hctToArgb(h: number, c: number, t: number): number {
  return lib.bun_rs_hct_to_argb(h, c, t);
}
export { pub_fn_hctToArgb as hctToArgb };

/** Convert ARGB to HCT coordinates. */
pub_fn_argbToHct(0xff6750a4);
function pub_fn_argbToHct(argb: number): { hue: number; chroma: number; tone: number } {
  const h = new Float32Array(1);
  const c = new Float32Array(1);
  const t = new Float32Array(1);
  lib.bun_rs_argb_to_hct(argb, h, c, t);
  return {
    hue: h[0],
    chroma: c[0],
    tone: t[0],
  };
}
export { pub_fn_argbToHct as argbToHct };

/** Generate the 13-stop tonal palette from HCT. */
pub_fn_hctTones(277.0, 40.0, 40.0);
function pub_fn_hctTones(h: number, c: number, t: number): Uint32Array {
  const p = new Uint32Array(13);
  lib.bun_rs_hct_tones(h, c, t, p);
  return p;
}
export { pub_fn_hctTones as hctTones };

/** Derive all 49 M3 color roles from HCT seed. */
pub_fn_deriveScheme(277.0, 40.0, false);
function pub_fn_deriveScheme(h: number, c: number, isDark: boolean): ColorRoleMap {
  const colors = new Uint32Array(49);
  lib.bun_rs_derive_scheme(h, c, isDark, colors);

  const out: ColorRoleMap = {};
  for (let i = 0; i < 49; i++) {
    const argb = colors[i];
    const r = (argb >> 16) & 0xff;
    const g = (argb >> 8) & 0xff;
    const b = argb & 0xff;
    const hex = `#${((1 << 24) + (r << 16) + (g << 8) + b).toString(16).slice(1)}`;
    out[`--md-sys-color-${ROLES[i]}`] = hex;
  }
  return out;
}
export { pub_fn_deriveScheme as deriveScheme };

/** Compile SCSS/Sass source to CSS using the grass Rust compiler. */
pub_fn_compileSass("a { b { color: red; } }");
function pub_fn_compileSass(
  source: string,
  loadPaths: string[] = [],
  style?: "expanded" | "compressed",
  quiet: boolean = false,
): string {
  const buf = Buffer.from(source);
  const loadPathsStr = loadPaths.join(";");
  const loadPathsBuf = Buffer.from(loadPathsStr);
  const styleVal = style === "compressed" ? 1 : 0;
  const err = new Uint8Array(1);
  const ptr = lib.bun_rs_compile_sass(
    buf,
    buf.length,
    loadPathsBuf,
    loadPathsBuf.length,
    styleVal,
    quiet,
    err,
  );
  if (!ptr) {
    throw new Error("Sass compilation returned a null pointer");
  }
  const css = new CString(ptr).toString();
  lib.bun_rs_free_string(ptr);
  if (err[0]) {
    throw new Error(css);
  }
  return css;
}
export { pub_fn_compileSass as compileSass };

/** Compile SCSS/Sass file to CSS using the grass Rust compiler. */
function pub_fn_compileSassFile(
  path: string,
  loadPaths: string[] = [],
  style?: "expanded" | "compressed",
  quiet: boolean = false,
): string {
  const buf = Buffer.from(path);
  const loadPathsStr = loadPaths.join(";");
  const loadPathsBuf = Buffer.from(loadPathsStr);
  const styleVal = style === "compressed" ? 1 : 0;
  const err = new Uint8Array(1);
  const ptr = lib.bun_rs_compile_sass_file(
    buf,
    buf.length,
    loadPathsBuf,
    loadPathsBuf.length,
    styleVal,
    quiet,
    err,
  );
  if (!ptr) {
    throw new Error("Sass compilation returned a null pointer");
  }
  const css = new CString(ptr).toString();
  lib.bun_rs_free_string(ptr);
  if (err[0]) {
    throw new Error(css);
  }
  return css;
}
export { pub_fn_compileSassFile as compileSassFile };

export interface ValidationIssue {
  level: "error" | "warning";
  rule: string;
  message: string;
  line: number;
  matched: string;
}

export interface ValidationResult {
  score: number;
  issues: ValidationIssue[];
}

/** Validate source code against M3 specification rules. */
function pub_fn_validateSpec(code: string): ValidationResult {
  const buf = Buffer.from(code);
  const ptr = lib.bun_rs_validate_spec(buf, buf.length);
  if (!ptr) {
    return { score: 100, issues: [] };
  }
  const jsonStr = new CString(ptr).toString();
  lib.bun_rs_free_string(ptr);
  return JSON.parse(jsonStr);
}
export { pub_fn_validateSpec as validateSpec };

function escapeForTemplate(css: string): string {
  return css.replace(/\\/g, "\\\\").replace(/`/g, "\\`").replace(/\$\{/g, "\\${");
}

export interface SassPluginOptions {
  loadPaths?: string[];
  style?: "expanded" | "compressed";
  quiet?: boolean;
}

/** Bun compiler plugin to load .scss files at native speed using Rust grass. */
export function sassRustPlugin(options: SassPluginOptions = {}) {
  return {
    name: "sass-rust-loader",
    setup(build: any) {
      build.onLoad({ filter: /\.scss$/ }, async (args: any) => {
        try {
          const quietVal = options.quiet ?? true;
          const css = pub_fn_compileSassFile(args.path, options.loadPaths, options.style, quietVal);
          const styles = escapeForTemplate(css);
          return {
            contents:
              `import { css } from "lit";\n` +
              `export const styles = css\`${styles}\`;\n` +
              `export default styles;\n`,
            loader: "js",
          };
        } catch (e: any) {
          return {
            contents:
              `import { css } from "lit";\n` +
              `export const styles = css\`/* Sass compilation error: ${escapeForTemplate(e.message)} */\`;\n` +
              `export default styles;\n`,
            loader: "js",
          };
        }
      });
    },
  };
}
