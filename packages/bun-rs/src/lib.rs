// SPDX-License-Identifier: Apache-2.0
#![deny(clippy::all, clippy::undocumented_unsafe_blocks)]

//! Bun-RS FFI module. Exposes optimized C-ABI functions to Bun applications
//! using `bun:ffi` for maximum performance and zero overhead.

use std::ffi::c_char;

/// Get the version of the wrapper crate as a C-string.
///
/// # Safety
/// The returned pointer points to static read-only memory and must not be freed.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn bun_rs_version() -> *const c_char {
    c"1.0.0-canary".as_ptr()
}

/// Simple addition function to benchmark FFI round-trip overhead.
#[unsafe(no_mangle)]
pub extern "C" fn bun_rs_add(a: i32, b: i32) -> i32 {
    a + b
}

/// Count occurrences of a specific character in a byte buffer.
/// Uses SIMD acceleration via the `memchr` crate.
///
/// # Safety
/// * `data` must be a valid pointer to a memory region of at least `len` bytes.
/// * The memory must be readable.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn bun_rs_count_char(data: *const u8, len: usize, needle: u8) -> usize {
    if data.is_null() || len == 0 {
        return 0;
    }
    // SAFETY: caller guarantees `data` points to at least `len` valid readable bytes.
    let slice = unsafe { std::slice::from_raw_parts(data, len) };
    memchr::memchr_iter(needle, slice).count()
}

/// Find the byte offset of a substring (needle) inside a larger buffer (haystack).
/// Returns `-1` if the needle is not found.
///
/// # Safety
/// * `haystack` must be a valid pointer to at least `haystack_len` readable bytes.
/// * `needle` must be a valid pointer to at least `needle_len` readable bytes.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn bun_rs_find_bytes(
    haystack: *const u8,
    haystack_len: usize,
    needle: *const u8,
    needle_len: usize,
) -> isize {
    if haystack.is_null() || needle.is_null() || haystack_len < needle_len {
        return -1;
    }
    if needle_len == 0 {
        return 0;
    }
    // SAFETY: caller guarantees pointers are valid for their respective lengths.
    let h = unsafe { std::slice::from_raw_parts(haystack, haystack_len) };
    // SAFETY: caller guarantees pointers are valid for their respective lengths.
    let n = unsafe { std::slice::from_raw_parts(needle, needle_len) };

    // Simple substring search using memchr to find first byte matches
    let first = n[0];
    let mut offset = 0;
    while offset + n.len() <= h.len() {
        if let Some(pos) = memchr::memchr(first, &h[offset..h.len() - n.len() + 1]) {
            let actual_pos = offset + pos;
            if &h[actual_pos..actual_pos + n.len()] == n {
                return actual_pos as isize;
            }
            offset = actual_pos + 1;
        } else {
            break;
        }
    }
    -1
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ffi_version() {
        // SAFETY: testing FFI version function with valid static pointer.
        unsafe {
            let ptr = bun_rs_version();
            let cstr = std::ffi::CStr::from_ptr(ptr);
            assert_eq!(cstr.to_str().unwrap(), "1.0.0-canary");
        }
    }

    #[test]
    fn test_ffi_add() {
        assert_eq!(bun_rs_add(10, 20), 30);
    }

    #[test]
    fn test_ffi_count_char() {
        let data = b"hello world, this is a test buffer!";
        // SAFETY: testing FFI count_char function with valid buffer pointer and length.
        unsafe {
            assert_eq!(bun_rs_count_char(data.as_ptr(), data.len(), b'o'), 2);
            assert_eq!(bun_rs_count_char(data.as_ptr(), data.len(), b'z'), 0);
            assert_eq!(bun_rs_count_char(std::ptr::null(), 0, b'a'), 0);
        }
    }

    #[test]
    fn test_ffi_find_bytes() {
        let haystack = b"supercalifragilisticexpialidocious";
        let needle = b"fragil";
        // SAFETY: testing FFI find_bytes function with valid haystack and needle pointers.
        unsafe {
            assert_eq!(
                bun_rs_find_bytes(haystack.as_ptr(), haystack.len(), needle.as_ptr(), needle.len()),
                9
            );
            let not_found = b"missing";
            assert_eq!(
                bun_rs_find_bytes(haystack.as_ptr(), haystack.len(), not_found.as_ptr(), not_found.len()),
                -1
            );
        }
    }
}
