// SPDX-License-Identifier: Apache-2.0
#![deny(clippy::all, clippy::undocumented_unsafe_blocks)]

//! Bun-RS FFI module. Exposes optimized C-ABI functions to Bun applications
//! using `bun:ffi` for maximum performance and zero overhead.

use std::ffi::c_char;
use wasm_bindgen::prelude::*;

mod symbols;
mod validator;

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

/// Convert HCT coordinates to packed ARGB u32.
#[unsafe(no_mangle)]
pub extern "C" fn bun_rs_hct_to_argb(hue: f32, chroma: f32, tone: f32) -> u32 {
    m3_tokens::hct::Hct::new(hue as f64, chroma as f64, tone as f64).to_argb()
}

/// Convert packed ARGB u32 to HCT coordinates.
///
/// # Safety
/// * `hue`, `chroma`, and `tone` must be valid, writable pointers to f32.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn bun_rs_argb_to_hct(
    argb: u32,
    hue: *mut f32,
    chroma: *mut f32,
    tone: *mut f32,
) {
    if hue.is_null() || chroma.is_null() || tone.is_null() {
        return;
    }
    let hct = m3_tokens::hct::Hct::from_argb(argb);
    // SAFETY: caller guarantees pointers are valid and writable.
    unsafe {
        *hue = hct.hue as f32;
        *chroma = hct.chroma as f32;
        *tone = hct.tone as f32;
    }
}

/// Generate the 13-stop tonal palette from HCT coordinates, writing the resulting
/// 13 ARGB values (u32 each) into the buffer `out_palette`.
///
/// # Safety
/// * `out_palette` must be a valid, writable pointer to at least 13 u32s.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn bun_rs_hct_tones(
    hue: f32,
    chroma: f32,
    _tone: f32,
    out_palette: *mut u32,
) {
    if out_palette.is_null() {
        return;
    }
    let tones_list = [0.0, 10.0, 20.0, 30.0, 40.0, 50.0, 60.0, 70.0, 80.0, 90.0, 95.0, 99.0, 100.0];
    // SAFETY: caller guarantees out_palette has capacity for 13 elements.
    unsafe {
        for i in 0..13 {
            *out_palette.add(i) = m3_tokens::hct::Hct::new(hue as f64, chroma as f64, tones_list[i]).to_argb();
        }
    }
}

/// Helper function to calculate a tone value for a given hue and chroma.
fn p_hct(h: f32, c: f32, t: f32) -> u32 {
    m3_tokens::hct::Hct::new(h as f64, c as f64, t as f64).to_argb()
}

/// Derive all 49 M3 color roles for a single mode (light or dark) from HCT seed.
///
/// # Safety
/// * `out_colors` must be a valid, writable pointer to at least 49 u32s.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn bun_rs_derive_scheme(
    hue: f32,
    chroma: f32,
    is_dark: bool,
    out_colors: *mut u32,
) {
    if out_colors.is_null() {
        return;
    }

    let primary_h = hue;
    let primary_c = chroma.max(48.0);
    let secondary_h = hue;
    let secondary_c = 16.0;
    let tertiary_h = hue + 60.0;
    let tertiary_c = 24.0;
    let error_h = 25.0;
    let error_c = 84.0;
    let neutral_h = hue;
    let neutral_c = 4.0;
    let neutral_variant_h = hue;
    let neutral_variant_c = 8.0;

    // SAFETY: caller guarantees out_colors has capacity for 49 elements.
    unsafe {
        // 0: background
        *out_colors.add(0) = p_hct(neutral_h, neutral_c, if is_dark { 10.0 } else { 99.0 });
        // 1: onBackground
        *out_colors.add(1) = p_hct(neutral_h, neutral_c, if is_dark { 90.0 } else { 10.0 });
        // 2: surface
        *out_colors.add(2) = p_hct(neutral_h, neutral_c, if is_dark { 10.0 } else { 99.0 });
        // 3: surfaceDim
        *out_colors.add(3) = p_hct(neutral_h, neutral_c, if is_dark { 6.0 } else { 90.0 });
        // 4: surfaceBright
        *out_colors.add(4) = p_hct(neutral_h, neutral_c, if is_dark { 24.0 } else { 98.0 });
        // 5: surfaceContainerLowest
        *out_colors.add(5) = p_hct(neutral_h, neutral_c, if is_dark { 4.0 } else { 100.0 });
        // 6: surfaceContainerLow
        *out_colors.add(6) = p_hct(neutral_h, neutral_c, if is_dark { 12.0 } else { 96.0 });
        // 7: surfaceContainer
        *out_colors.add(7) = p_hct(neutral_h, neutral_c, if is_dark { 17.0 } else { 94.0 });
        // 8: surfaceContainerHigh
        *out_colors.add(8) = p_hct(neutral_h, neutral_c, if is_dark { 22.0 } else { 92.0 });
        // 9: surfaceContainerHighest
        *out_colors.add(9) = p_hct(neutral_h, neutral_c, if is_dark { 28.0 } else { 90.0 });
        // 10: onSurface
        *out_colors.add(10) = p_hct(neutral_h, neutral_c, if is_dark { 90.0 } else { 10.0 });
        // 11: surfaceVariant
        *out_colors.add(11) = p_hct(neutral_variant_h, neutral_variant_c, if is_dark { 30.0 } else { 90.0 });
        // 12: onSurfaceVariant
        *out_colors.add(12) = p_hct(neutral_variant_h, neutral_variant_c, if is_dark { 80.0 } else { 30.0 });
        // 13: inverseSurface
        *out_colors.add(13) = p_hct(neutral_h, neutral_c, if is_dark { 90.0 } else { 20.0 });
        // 14: inverseOnSurface
        *out_colors.add(14) = p_hct(neutral_h, neutral_c, if is_dark { 20.0 } else { 95.0 });
        // 15: outline
        *out_colors.add(15) = p_hct(neutral_variant_h, neutral_variant_c, if is_dark { 60.0 } else { 50.0 });
        // 16: outlineVariant
        *out_colors.add(16) = p_hct(neutral_variant_h, neutral_variant_c, if is_dark { 30.0 } else { 80.0 });
        // 17: shadow
        *out_colors.add(17) = p_hct(neutral_h, neutral_c, 0.0);
        // 18: scrim
        *out_colors.add(18) = p_hct(neutral_h, neutral_c, 0.0);
        // 19: surfaceTint
        *out_colors.add(19) = p_hct(primary_h, primary_c, if is_dark { 80.0 } else { 40.0 });
        // 20: primary
        *out_colors.add(20) = p_hct(primary_h, primary_c, if is_dark { 80.0 } else { 40.0 });
        // 21: onPrimary
        *out_colors.add(21) = p_hct(primary_h, primary_c, if is_dark { 20.0 } else { 100.0 });
        // 22: primaryContainer
        *out_colors.add(22) = p_hct(primary_h, primary_c, if is_dark { 30.0 } else { 90.0 });
        // 23: onPrimaryContainer
        *out_colors.add(23) = p_hct(primary_h, primary_c, if is_dark { 90.0 } else { 10.0 });
        // 24: inversePrimary
        *out_colors.add(24) = p_hct(primary_h, primary_c, if is_dark { 40.0 } else { 80.0 });
        // 25: secondary
        *out_colors.add(25) = p_hct(secondary_h, secondary_c, if is_dark { 80.0 } else { 40.0 });
        // 26: onSecondary
        *out_colors.add(26) = p_hct(secondary_h, secondary_c, if is_dark { 20.0 } else { 100.0 });
        // 27: secondaryContainer
        *out_colors.add(27) = p_hct(secondary_h, secondary_c, if is_dark { 30.0 } else { 90.0 });
        // 28: onSecondaryContainer
        *out_colors.add(28) = p_hct(secondary_h, secondary_c, if is_dark { 90.0 } else { 10.0 });
        // 29: tertiary
        *out_colors.add(29) = p_hct(tertiary_h, tertiary_c, if is_dark { 80.0 } else { 40.0 });
        // 30: onTertiary
        *out_colors.add(30) = p_hct(tertiary_h, tertiary_c, if is_dark { 20.0 } else { 100.0 });
        // 31: tertiaryContainer
        *out_colors.add(31) = p_hct(tertiary_h, tertiary_c, if is_dark { 30.0 } else { 90.0 });
        // 32: onTertiaryContainer
        *out_colors.add(32) = p_hct(tertiary_h, tertiary_c, if is_dark { 90.0 } else { 10.0 });
        // 33: error
        *out_colors.add(33) = p_hct(error_h, error_c, if is_dark { 80.0 } else { 40.0 });
        // 34: onError
        *out_colors.add(34) = p_hct(error_h, error_c, if is_dark { 20.0 } else { 100.0 });
        // 35: errorContainer
        *out_colors.add(35) = p_hct(error_h, error_c, if is_dark { 30.0 } else { 90.0 });
        // 36: onErrorContainer
        *out_colors.add(36) = p_hct(error_h, error_c, if is_dark { 90.0 } else { 10.0 });
        // 37: primaryFixed
        *out_colors.add(37) = p_hct(primary_h, primary_c, 90.0);
        // 38: primaryFixedDim
        *out_colors.add(38) = p_hct(primary_h, primary_c, 80.0);
        // 39: onPrimaryFixed
        *out_colors.add(39) = p_hct(primary_h, primary_c, 10.0);
        // 40: onPrimaryFixedVariant
        *out_colors.add(40) = p_hct(primary_h, primary_c, 30.0);
        // 41: secondaryFixed
        *out_colors.add(41) = p_hct(secondary_h, secondary_c, 90.0);
        // 42: secondaryFixedDim
        *out_colors.add(42) = p_hct(secondary_h, secondary_c, 80.0);
        // 43: onSecondaryFixed
        *out_colors.add(43) = p_hct(secondary_h, secondary_c, 10.0);
        // 44: onSecondaryFixedVariant
        *out_colors.add(44) = p_hct(secondary_h, secondary_c, 30.0);
        // 45: tertiaryFixed
        *out_colors.add(45) = p_hct(tertiary_h, tertiary_c, 90.0);
        // 46: tertiaryFixedDim
        *out_colors.add(46) = p_hct(tertiary_h, tertiary_c, 80.0);
        // 47: onTertiaryFixed
        *out_colors.add(47) = p_hct(tertiary_h, tertiary_c, 10.0);
        // 48: onTertiaryFixedVariant
        *out_colors.add(48) = p_hct(tertiary_h, tertiary_c, 30.0);
    }
}

/// Compile a Sass/SCSS string into CSS.
///
/// # Safety
/// * `source` must be a valid pointer to a UTF-8 encoded string of length `len`.
/// * Returns a pointer to a null-terminated C-style string containing either the compiled CSS
///   or an error message.
/// * The returned pointer is allocated by Rust (using `CString::into_raw`) and MUST be freed by JS
///   calling `bun_rs_free_string`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn bun_rs_compile_sass(
    source: *const u8,
    len: usize,
    load_paths: *const u8,
    load_paths_len: usize,
    style_val: i32,
    quiet: bool,
    error_occurred: *mut bool,
) -> *mut c_char {
    if source.is_null() || len == 0 {
        if !error_occurred.is_null() {
            // SAFETY: pointer check is non-null.
            unsafe { *error_occurred = true; }
        }
        return std::ffi::CString::new("Error: empty or null source").unwrap().into_raw();
    }

    // SAFETY: caller guarantees the pointer is valid and contains at least `len` bytes.
    let slice = unsafe { std::slice::from_raw_parts(source, len) };
    let sass_str = match std::str::from_utf8(slice) {
        Ok(s) => s,
        Err(e) => {
            if !error_occurred.is_null() {
                // SAFETY: pointer check is non-null.
                unsafe { *error_occurred = true; }
            }
            return std::ffi::CString::new(format!("UTF-8 error: {}", e)).unwrap().into_raw();
        }
    };

    let mut options = grass::Options::default();
    if style_val == 1 {
        options = options.style(grass::OutputStyle::Compressed);
    } else {
        options = options.style(grass::OutputStyle::Expanded);
    }
    options = options.quiet(quiet);

    if !load_paths.is_null() && load_paths_len > 0 {
        let l_slice = unsafe { std::slice::from_raw_parts(load_paths, load_paths_len) };
        if let Ok(s) = std::str::from_utf8(l_slice) {
            let paths: Vec<std::path::PathBuf> = s
                .split(';')
                .filter(|p| !p.is_empty())
                .map(std::path::PathBuf::from)
                .collect();
            options = options.load_paths(&paths);
        }
    }

    let options = std::panic::AssertUnwindSafe(options);

    // Use catch_unwind to prevent Rust panics from crashing Bun.
    let result = std::panic::catch_unwind(move || {
        grass::from_string(sass_str.to_owned(), &*options)
    });

    match result {
        Ok(Ok(css)) => {
            if !error_occurred.is_null() {
                // SAFETY: pointer check is non-null.
                unsafe { *error_occurred = false; }
            }
            std::ffi::CString::new(css).unwrap().into_raw()
        }
        Ok(Err(e)) => {
            if !error_occurred.is_null() {
                // SAFETY: pointer check is non-null.
                unsafe { *error_occurred = true; }
            }
            std::ffi::CString::new(format!("Sass error: {}", e)).unwrap().into_raw()
        }
        Err(_) => {
            if !error_occurred.is_null() {
                // SAFETY: pointer check is non-null.
                unsafe { *error_occurred = true; }
            }
            std::ffi::CString::new("Rust panic during Sass compilation").unwrap().into_raw()
        }
    }
}

/// Compile a Sass/SCSS file into CSS.
///
/// # Safety
/// * `path` must be a valid pointer to a UTF-8 encoded string of length `len`.
/// * Returns a pointer to a null-terminated C-style string containing either the compiled CSS
///   or an error message.
/// * The returned pointer is allocated by Rust (using `CString::into_raw`) and MUST be freed by JS
///   calling `bun_rs_free_string`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn bun_rs_compile_sass_file(
    path: *const u8,
    len: usize,
    load_paths: *const u8,
    load_paths_len: usize,
    style_val: i32,
    quiet: bool,
    error_occurred: *mut bool,
) -> *mut c_char {
    if path.is_null() || len == 0 {
        if !error_occurred.is_null() {
            // SAFETY: pointer check is non-null.
            unsafe { *error_occurred = true; }
        }
        return std::ffi::CString::new("Error: empty or null path").unwrap().into_raw();
    }

    // SAFETY: caller guarantees the pointer is valid and contains at least `len` bytes.
    let slice = unsafe { std::slice::from_raw_parts(path, len) };
    let path_str = match std::str::from_utf8(slice) {
        Ok(s) => s,
        Err(e) => {
            if !error_occurred.is_null() {
                // SAFETY: pointer check is non-null.
                unsafe { *error_occurred = true; }
            }
            return std::ffi::CString::new(format!("UTF-8 error: {}", e)).unwrap().into_raw();
        }
    };

    let mut options = grass::Options::default();
    if style_val == 1 {
        options = options.style(grass::OutputStyle::Compressed);
    } else {
        options = options.style(grass::OutputStyle::Expanded);
    }
    options = options.quiet(quiet);

    if !load_paths.is_null() && load_paths_len > 0 {
        let l_slice = unsafe { std::slice::from_raw_parts(load_paths, load_paths_len) };
        if let Ok(s) = std::str::from_utf8(l_slice) {
            let paths: Vec<std::path::PathBuf> = s
                .split(';')
                .filter(|p| !p.is_empty())
                .map(std::path::PathBuf::from)
                .collect();
            options = options.load_paths(&paths);
        }
    }

    let options = std::panic::AssertUnwindSafe(options);

    // Use catch_unwind to prevent Rust panics from crashing Bun.
    let result = std::panic::catch_unwind(move || {
        grass::from_path(path_str, &*options)
    });

    match result {
        Ok(Ok(css)) => {
            if !error_occurred.is_null() {
                // SAFETY: pointer check is non-null.
                unsafe { *error_occurred = false; }
            }
            std::ffi::CString::new(css).unwrap().into_raw()
        }
        Ok(Err(e)) => {
            if !error_occurred.is_null() {
                // SAFETY: pointer check is non-null.
                unsafe { *error_occurred = true; }
            }
            std::ffi::CString::new(format!("Sass error: {}", e)).unwrap().into_raw()
        }
        Err(_) => {
            if !error_occurred.is_null() {
                // SAFETY: pointer check is non-null.
                unsafe { *error_occurred = true; }
            }
            std::ffi::CString::new("Rust panic during Sass compilation").unwrap().into_raw()
        }
    }
}


/// Validate a source code string against M3 design system specifications.
/// Returns a pointer to a null-terminated C-style JSON string containing the validation score and issues.
///
/// # Safety
/// * `source` must be a valid pointer to a UTF-8 encoded string of length `len`.
/// * The returned pointer is allocated by Rust and must be freed by JS using `bun_rs_free_string`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn bun_rs_validate_spec(
	source: *const u8,
	len: usize,
) -> *mut c_char {
	if source.is_null() || len == 0 {
		return std::ffi::CString::new("{\"score\":100,\"issues\":[]}").unwrap().into_raw();
	}

	// SAFETY: caller guarantees the pointer is valid and contains at least `len` bytes.
	let slice = unsafe { std::slice::from_raw_parts(source, len) };
	let code_str = match std::str::from_utf8(slice) {
		Ok(s) => s,
		Err(_) => {
			return std::ffi::CString::new("{\"score\":0,\"issues\":[{\"level\":\"error\",\"rule\":\"invalid-utf8\",\"message\":\"Invalid UTF-8 source code\",\"line\":1,\"matched\":\"\"}]}").unwrap().into_raw();
		}
	};

	let (score, issues) = validator::validate_m3_spec(code_str);

	// Build the JSON manually to avoid heavy serde dependency
	let mut json = String::new();
	json.push_str("{\"score\":");
	json.push_str(&score.to_string());
	json.push_str(",\"issues\":[");

	let mut first = true;
	for issue in &issues {
		if !first {
			json.push(',');
		}
		first = false;
		json.push_str("{\"level\":\"");
		json.push_str(issue.level);
		json.push_str("\",\"rule\":\"");
		json.push_str(issue.rule);
		json.push_str("\",\"message\":\"");
		json.push_str(&escape_json(&issue.message));
		json.push_str("\",\"line\":");
		json.push_str(&issue.line.to_string());
		json.push_str(",\"matched\":\"");
		json.push_str(&escape_json(&issue.matched));
		json.push_str("\"}");
	}

	json.push_str("]}");

	std::ffi::CString::new(json).unwrap().into_raw()
}

fn escape_json(s: &str) -> String {
	let mut escaped = String::new();
	for c in s.chars() {
		match c {
			'"' => escaped.push_str("\\\""),
			'\\' => escaped.push_str("\\\\"),
			'\n' => escaped.push_str("\\n"),
			'\r' => escaped.push_str("\\r"),
			'\t' => escaped.push_str("\\t"),
			_ => escaped.push(c),
		}
	}
	escaped
}

/// Free a string allocated by Rust (returned by `bun_rs_compile_sass`).
///
/// # Safety
/// * `ptr` must be a pointer returned by `bun_rs_compile_sass`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn bun_rs_free_string(ptr: *mut c_char) {
    if ptr.is_null() {
        return;
    }
    // SAFETY: retake ownership of the pointer to let CString drop and deallocate.
    unsafe {
        let _ = std::ffi::CString::from_raw(ptr);
    }
}

// ============================================================================
//  WebAssembly (WASM) exports for browser-side execution
// ============================================================================

#[wasm_bindgen]
pub fn wasm_argb_to_hct(argb: u32) -> Vec<f32> {
    let hct = m3_tokens::hct::Hct::from_argb(argb);
    vec![hct.hue as f32, hct.chroma as f32, hct.tone as f32]
}

#[wasm_bindgen]
pub fn wasm_hct_to_argb(hue: f32, chroma: f32, tone: f32) -> u32 {
    m3_tokens::hct::Hct::new(hue as f64, chroma as f64, tone as f64).to_argb()
}

#[wasm_bindgen]
pub fn wasm_hct_tones(hue: f32, chroma: f32) -> Vec<u32> {
    let tones_list = [0.0, 10.0, 20.0, 30.0, 40.0, 50.0, 60.0, 70.0, 80.0, 90.0, 95.0, 99.0, 100.0];
    tones_list
        .iter()
        .map(|&tone| m3_tokens::hct::Hct::new(hue as f64, chroma as f64, tone).to_argb())
        .collect()
}

#[wasm_bindgen]
pub fn wasm_derive_scheme(hue: f32, chroma: f32, is_dark: bool) -> Vec<u32> {
    let mut out = vec![0u32; 49];
    // SAFETY: out has exactly 49 capacity and is a valid mut pointer
    unsafe {
        bun_rs_derive_scheme(hue, chroma, is_dark, out.as_mut_ptr());
    }
    out
}

#[wasm_bindgen]
pub fn wasm_validate_spec(source: &str) -> String {
    let (score, issues) = validator::validate_m3_spec(source);
    // Build JSON
    let mut json = String::new();
    json.push_str("{\"score\":");
    json.push_str(&score.to_string());
    json.push_str(",\"issues\":[");

    let mut first = true;
    for issue in &issues {
        if !first {
            json.push(',');
        }
        first = false;
        json.push_str("{\"level\":\"");
        json.push_str(issue.level);
        json.push_str("\",\"rule\":\"");
        json.push_str(issue.rule);
        json.push_str("\",\"message\":\"");
        json.push_str(&escape_json(&issue.message));
        json.push_str("\",\"line\":");
        json.push_str(&issue.line.to_string());
        json.push_str(",\"matched\":\"");
        json.push_str(&escape_json(&issue.matched));
        json.push_str("\"}");
    }

    json.push_str("]}");
    json
}

#[wasm_bindgen]
pub fn wasm_compile_sass(source: &str, load_paths: &str, compressed: bool, quiet: bool) -> Result<String, String> {
    let mut options = grass::Options::default();
    if compressed {
        options = options.style(grass::OutputStyle::Compressed);
    } else {
        options = options.style(grass::OutputStyle::Expanded);
    }
    options = options.quiet(quiet);

    if !load_paths.is_empty() {
        let paths: Vec<std::path::PathBuf> = load_paths
            .split(';')
            .filter(|p| !p.is_empty())
            .map(std::path::PathBuf::from)
            .collect();
        options = options.load_paths(&paths);
    }

    grass::from_string(source.to_owned(), &options)
        .map_err(|e| e.to_string())
}

#[derive(serde::Serialize, serde::Deserialize, ts_rs::TS)]
#[ts(export)]
pub struct ThemeConfig {
    pub seed_color: String,
    pub dark_mode: bool,
    pub contrast_level: f64,
}

#[wasm_bindgen]
pub fn wasm_generate_theme_config(config_val: JsValue) -> Result<JsValue, JsValue> {
    let config: ThemeConfig = serde_wasm_bindgen::from_value(config_val)
        .map_err(|e| JsValue::from_str(&e.to_string()))?;
    serde_wasm_bindgen::to_value(&config)
        .map_err(|e| JsValue::from_str(&e.to_string()))
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

    #[test]
    fn test_ffi_color_space() {
        // Test color space FFI functions
        let argb = bun_rs_hct_to_argb(277.0, 40.0, 40.0);
        assert!(argb == 0xFF4D5A9A || argb == 0xFF4D5A99); // standard MCU high precision match
 
        let mut h = 0.0_f32;
        let mut c = 0.0_f32;
        let mut t = 0.0_f32;
        // SAFETY: testing with valid mutable pointers
        unsafe {
            bun_rs_argb_to_hct(0xFF6750A4, &mut h, &mut c, &mut t);
            assert!((297.0..=300.0).contains(&h));
            assert!((46.0..=49.0).contains(&c));
            assert!((39.0..=41.0).contains(&t));
        }
    }

    #[test]
    fn test_ffi_hct_tones() {
        let mut p = [0_u32; 13];
        // SAFETY: testing with valid pointer to 13 u32s
        unsafe {
            bun_rs_hct_tones(277.0, 40.0, 40.0, p.as_mut_ptr());
            assert_eq!(p[0], 0xFF000000); // tone 0
            assert_eq!(p[1], 0xFF021453); // tone 10 (exact high precision match)
            assert_eq!(p[12], 0xFFFFFFFF); // tone 100
        }
    }

    #[test]
    fn test_ffi_derive_scheme() {
        let mut c = [0_u32; 49];
        // SAFETY: testing with valid pointer to 49 u32s
        unsafe {
            bun_rs_derive_scheme(277.0, 40.0, false, c.as_mut_ptr());
            assert_eq!(c[20] >> 24, 0xFF); // valid alpha
            assert_eq!(c[21], 0xFFFFFFFF); // onPrimary light
        }
    }

    #[test]
    fn test_ffi_compile_sass() {
        let scss = b"a { b { color: red; } }";
        let mut err = false;
        // SAFETY: testing compile with valid pointer
        unsafe {
            let css_ptr = bun_rs_compile_sass(scss.as_ptr(), scss.len(), std::ptr::null(), 0, 0, false, &mut err);
            assert!(!err);
            let css_cstr = std::ffi::CStr::from_ptr(css_ptr);
            let css_str = css_cstr.to_str().unwrap();
            assert!(css_str.contains("a b"));
            assert!(css_str.contains("color: red"));
            bun_rs_free_string(css_ptr);
        }
    }

    #[test]
    fn test_ffi_validate_spec() {
        let code = r#"
            // Valid icon and curve
            const style = "transition: transform 300ms cubic-bezier(0.42, 1.67, 0.21, 0.9);";
            const icon = <md-icon>check</md-icon>;
            const button = <md-icon-button aria-label="Validate"></md-icon-button>;

            // Violations
            const bad_color = "color: #ff0077;";
            const bad_role = "--md-sys-color-invalid-primary";
            const bad_icon = <md-icon>non_existent_icon_name</md-icon>;
            const bad_curve = "transition: opacity 150ms cubic-bezier(0.1, 0.2, 0.3, 0.4);";
            const bad_btn = <md-icon-button></md-icon-button>;
        "#;
        let (score, issues) = validator::validate_m3_spec(code);
        assert!(score < 100);
        assert!(issues.len() >= 5);
    }

    #[test]
    fn test_wasm_exports() {
        let hct = wasm_argb_to_hct(0xFF6750A4);
        assert!((297.0..=300.0).contains(&hct[0]));
        
        let argb = wasm_hct_to_argb(hct[0], hct[1], hct[2]);
        assert_eq!(argb, 0xFF6750A4);

        let tones = wasm_hct_tones(277.0, 40.0);
        assert_eq!(tones.len(), 13);
        assert_eq!(tones[0], 0xFF000000);

        let scheme = wasm_derive_scheme(277.0, 40.0, false);
        assert_eq!(scheme.len(), 49);

        let validation = wasm_validate_spec("const style = 'color: #ff0077;';");
        assert!(validation.contains("m3/no-hardcoded-color"));

        let css = wasm_compile_sass("body { color: red; }", "", true, false).unwrap();
        assert!(css.contains("body{color:red}"));
    }
}



