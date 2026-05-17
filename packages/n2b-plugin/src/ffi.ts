// Copyright 2026 aphrody-code
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     https://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

// Binding bun:ffi vers libnode2bun_native.
//
// Expose `getLineOffsets(source)` : un Uint32Array des positions (u16
// code units) de chaque '\n' dans la source, calculé en Rust (memchr
// SIMD) et mis en cache par source via WeakMap. posFromIndex peut
// alors faire une binary search O(log n) au lieu d'un scan O(n).

import { FFIType, dlopen, ptr, suffix } from "bun:ffi";
import { join } from "node:path";
import { fileURLToPath } from "node:url";

type Native = {
  find_newlines_u16: (buf: Uint8Array, byteLen: number, out: Uint32Array, outCap: number) => number;
  node2bun_abi_version: () => number;
};

function tryLoad(): Native | null {
  const here = fileURLToPath(new URL(".", import.meta.url));
  // Chemins candidats :
  //   - dev workspace : ../../../../target/release/ (packages/n2b/src → root/target/)
  //   - dev legacy    : ../../../native/target/release/
  //   - installé      : ../lib/
  const candidates = [
    // aphrody native integration (bun_ffi)
    join(here, "..", "..", "..", "..", "..", "target", "release", process.platform === "win32" ? "bun_ffi.dll" : `libbun_ffi.${suffix}`),
    // n2b standalone native legacy
    join(here, "..", "..", "..", "..", "target", "release", `libnode2bun_native.${suffix}`),
    join(here, "..", "..", "..", "native", "target", "release", `libnode2bun_native.${suffix}`),
    join(here, "..", "lib", `libnode2bun_native.${suffix}`),
  ];
  for (const path of candidates) {
    try {
      const lib = dlopen(path, {
        find_newlines_u16: {
          args: [FFIType.ptr, FFIType.u64_fast, FFIType.ptr, FFIType.u64_fast],
          returns: FFIType.u64_fast,
        },
        node2bun_abi_version: { args: [], returns: FFIType.u32 },
      });
      if (lib.symbols.node2bun_abi_version() !== 1) continue;
      return {
        node2bun_abi_version: () => Number(lib.symbols.node2bun_abi_version()),
        find_newlines_u16: (buf, byteLen, out, outCap) =>
          Number(lib.symbols.find_newlines_u16(ptr(buf), byteLen, ptr(out), outCap)),
      };
    } catch {
      // Essaie le candidat suivant.
    }
  }
  return null;
}

const NATIVE = tryLoad();
export const nativeAvailable = NATIVE !== null;

const cache = new WeakMap<object, Uint32Array>();
const keys = new WeakMap<string & object, object>();

function keyFor(_source: string): object {
  // WeakMap exige des clés objet. On box le string via un wrapper stable
  // — mais les strings primitives ne peuvent pas être clés, donc on
  // utilise un Map faible indirect : en pratique, les sources restent
  // en mémoire le temps du scan, on peut se contenter d'un Map normal
  // scopé à l'appel. On utilise ici un WeakRef-style via global Map
  // éphémère clearé par le caller.
  // Simplification : on expose `computeLineOffsets(source)` sans cache
  // interne — le scanner l'appelle une fois et passe le tableau à
  // posFromIndex.
  throw new Error("unreachable");
}
void keyFor;
void cache;
void keys;

/**
 * Calcule les positions de newlines dans `source` (en u16 code units).
 * Utilise la lib Rust si disponible, sinon fallback JS.
 */
export function computeLineOffsets(source: string): Uint32Array {
  if (source.length === 0) return new Uint32Array(0);
  if (NATIVE) {
    // Bun supporte Buffer.from(str, "utf16le") en fast path.
    const bytes = Buffer.from(source, "utf16le");
    if (bytes.byteLength === 0) return new Uint32Array(0);
    // Heuristique : ~1 newline tous les 40 chars + marge.
    const cap = Math.max(16, (source.length >>> 5) + 8);
    let out = new Uint32Array(cap);
    let n = NATIVE.find_newlines_u16(bytes, bytes.byteLength, out, cap);
    if (n > cap) {
      out = new Uint32Array(n);
      n = NATIVE.find_newlines_u16(bytes, bytes.byteLength, out, n);
    }
    return out.subarray(0, n) as Uint32Array;
  }
  // Fallback JS pur.
  const offsets: number[] = [];
  for (let i = 0; i < source.length; i++) {
    if (source.charCodeAt(i) === 10) offsets.push(i);
  }
  return new Uint32Array(offsets);
}

/**
 * Binary search : plus grand offset de newline < index. Renvoie -1
 * si l'index précède toute newline.
 */
function lastNewlineBefore(offsets: Uint32Array, index: number): number {
  let lo = 0;
  let hi = offsets.length;
  while (lo < hi) {
    const mid = (lo + hi) >>> 1;
    if (offsets[mid]! < index) lo = mid + 1;
    else hi = mid;
  }
  return lo - 1;
}

/**
 * Convertit un index (u16 offset dans source) en (line, col), 1-based.
 * Reproduit exactement la sémantique de l'ancien posFromIndex JS.
 */
export function posFromOffsets(offsets: Uint32Array, index: number): { line: number; col: number } {
  const i = lastNewlineBefore(offsets, index);
  if (i < 0) return { line: 1, col: index + 1 };
  const line = i + 2;
  const col = index - offsets[i]!;
  return { line, col };
}
