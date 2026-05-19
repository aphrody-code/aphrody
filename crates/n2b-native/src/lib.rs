// Copyright 2026 Yohan Pierre
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

// node2bun-native — hot-path helpers exposés à Bun via bun:ffi.
//
// Les fichiers source sont passés encodés en UTF-16LE (même représentation
// interne que les JS strings en général). Les offsets retournés sont en
// UTF-16 code units, ce qui correspond à `String.prototype.charCodeAt` et
// `indexOf` côté JS — donc compatibles avec l'index passé par makeFinding.

use memchr::memchr_iter;

/// Écrit dans `out_ptr` les offsets (en u16 units) de chaque '\n' trouvé
/// dans le buffer UTF-16LE pointé par `buf` (`byte_len` octets, donc
/// `byte_len/2` u16s).
///
/// Retourne le nombre total de newlines. Si ce nombre excède `out_cap`,
/// `out` n'est que partiellement rempli et l'appelant doit rappeler avec
/// un buffer plus grand.
///
/// # Safety
/// - `buf` doit pointer sur `byte_len` octets valides et alignés sur 2.
/// - `out_ptr` doit pointer sur `out_cap` `u32` valides et alignés.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn find_newlines_u16(
    buf: *const u8,
    byte_len: usize,
    out_ptr: *mut u32,
    out_cap: usize,
) -> usize {
    if buf.is_null() || byte_len < 2 {
        return 0;
    }
    let bytes = unsafe { std::slice::from_raw_parts(buf, byte_len & !1) };
    let out = if out_ptr.is_null() || out_cap == 0 {
        &mut [][..]
    } else {
        unsafe { std::slice::from_raw_parts_mut(out_ptr, out_cap) }
    };

    // UTF-16LE : le code unit 0x000A (\n) est représenté par les octets
    // 0x0A 0x00. On cherche 0x0A et on filtre : position paire + octet
    // haut à zéro. Ça exclut le cas où 0x0A apparaît comme octet haut
    // d'un autre caractère (U+0Axx).
    let mut count: usize = 0;
    for byte_pos in memchr_iter(0x0A, bytes) {
        if byte_pos & 1 != 0 {
            continue;
        }
        // SAFETY: byte_pos < bytes.len() et bytes.len() pair.
        let hi = unsafe { *bytes.get_unchecked(byte_pos + 1) };
        if hi != 0 {
            continue;
        }
        let u16_index = (byte_pos >> 1) as u32;
        if count < out.len() {
            out[count] = u16_index;
        }
        count += 1;
    }
    count
}

/// Sanity-check appelé par le wrapper TS au load pour vérifier que la
/// lib est chargeable et que l'ABI est la bonne.
#[unsafe(no_mangle)]
pub extern "C" fn node2bun_abi_version() -> u32 {
    1
}
