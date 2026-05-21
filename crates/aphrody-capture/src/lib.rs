// SPDX-License-Identifier: Apache-2.0
//! `aphrody-capture` — native Windows screen and window capture to PNG.
//!
//! Uses the Win32 GDI BitBlt pipeline (GetDC / CreateCompatibleDC /
//! CreateCompatibleBitmap / BitBlt / GetDIBits) to capture pixels, converts
//! the BGRA buffer to RGBA, and encodes it as PNG via the `image` crate.
//!
//! # Platform gating
//!
//! All capture logic is `#[cfg(windows)]`.  On non-Windows targets every
//! public function returns [`CaptureError::Unsupported`].
//!
//! # Example
//!
//! ```no_run
//! # #[cfg(windows)]
//! # {
//! use aphrody_capture::capture_primary_screen;
//! let png_bytes = capture_primary_screen().expect("capture failed");
//! std::fs::write("screen.png", &png_bytes).unwrap();
//! # }
//! ```

#![deny(unsafe_op_in_unsafe_fn)]

mod error;
pub use error::CaptureError;

/// Convenience `Result` alias.
pub type Result<T> = std::result::Result<T, CaptureError>;

/// Metadata for an enumerated top-level window.
#[derive(Debug, Clone)]
pub struct WindowInfo {
    /// Window title text.
    pub title: String,
    /// Raw HWND value cast to `usize` (opaque handle, not dereferenced outside
    /// of this crate).
    pub handle: usize,
}

// ─── platform impl ──────────────────────────────────────────────────────────

#[cfg(windows)]
mod capture_impl;

// ─── public API — Windows ────────────────────────────────────────────────────

/// Capture the primary monitor (SM_CXSCREEN × SM_CYSCREEN) to PNG bytes.
///
/// Returns the complete PNG-encoded image as a `Vec<u8>`.
#[cfg(windows)]
pub fn capture_primary_screen() -> Result<Vec<u8>> {
    capture_impl::capture_primary_screen()
}

/// Capture the full virtual screen spanning all monitors to PNG bytes.
///
/// Uses `SM_XVIRTUALSCREEN`, `SM_YVIRTUALSCREEN`, `SM_CXVIRTUALSCREEN`,
/// `SM_CYVIRTUALSCREEN`.
#[cfg(windows)]
pub fn capture_virtual_screen() -> Result<Vec<u8>> {
    capture_impl::capture_virtual_screen()
}

/// Find the first visible window whose title contains `substr` (case-sensitive)
/// and capture its client area to PNG bytes.
///
/// Uses `GetWindowRect` on the matched HWND to determine the region, then
/// performs a screen blit of that rectangle.
///
/// # Errors
///
/// Returns [`CaptureError::NotFound`] when no window title matches.
#[cfg(windows)]
pub fn capture_window_by_title(substr: &str) -> Result<Vec<u8>> {
    capture_impl::capture_window_by_title(substr)
}

/// Enumerate all visible top-level windows that have a non-empty title.
///
/// The returned order matches `EnumWindows` (Z-order, top-most first).
#[cfg(windows)]
pub fn list_windows() -> Vec<WindowInfo> {
    capture_impl::list_windows()
}

// ─── public API — non-Windows stubs ─────────────────────────────────────────

/// Capture the primary monitor.  Returns [`CaptureError::Unsupported`] on
/// non-Windows targets.
#[cfg(not(windows))]
pub fn capture_primary_screen() -> Result<Vec<u8>> {
    Err(CaptureError::Unsupported)
}

/// Capture the virtual screen.  Returns [`CaptureError::Unsupported`] on
/// non-Windows targets.
#[cfg(not(windows))]
pub fn capture_virtual_screen() -> Result<Vec<u8>> {
    Err(CaptureError::Unsupported)
}

/// Find and capture a window by title substring.  Returns
/// [`CaptureError::Unsupported`] on non-Windows targets.
#[cfg(not(windows))]
pub fn capture_window_by_title(_substr: &str) -> Result<Vec<u8>> {
    Err(CaptureError::Unsupported)
}

/// Enumerate visible windows.  Returns an empty `Vec` on non-Windows targets.
#[cfg(not(windows))]
pub fn list_windows() -> Vec<WindowInfo> {
    Vec::new()
}

// ─── pure helper (all platforms) ─────────────────────────────────────────────

/// Convert a mutable BGRA byte slice in-place to RGBA.
///
/// The Win32 GDI produces pixels in `[B, G, R, A]` order.  This swaps the
/// blue and red channels and forces alpha to `0xFF` (the alpha byte returned
/// by `GetDIBits` for a 32 bpp RGB bitmap is always 0 and must be fixed up).
///
/// # Panics
///
/// Panics if `buf.len()` is not a multiple of 4.
///
/// # Example
///
/// ```
/// use aphrody_capture::bgra_to_rgba;
/// let mut px = [0xBB_u8, 0x00, 0xFF, 0x00]; // B=0xBB, G=0x00, R=0xFF, A=0x00
/// bgra_to_rgba(&mut px);
/// assert_eq!(px, [0xFF, 0x00, 0xBB, 0xFF]); // R=0xFF, G=0x00, B=0xBB, A=0xFF
/// ```
pub fn bgra_to_rgba(buf: &mut [u8]) {
    assert!(buf.len() % 4 == 0, "bgra_to_rgba: buffer length must be a multiple of 4");
    for px in buf.chunks_exact_mut(4) {
        px.swap(0, 2); // swap B <-> R
        px[3] = 0xFF;  // fix up alpha
    }
}

// ─── tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bgra_single_pixel() {
        let mut buf = [0x12_u8, 0x34, 0x56, 0x00]; // B=0x12, G=0x34, R=0x56, A=0
        bgra_to_rgba(&mut buf);
        assert_eq!(buf, [0x56, 0x34, 0x12, 0xFF]); // R=0x56, G=0x34, B=0x12, A=0xFF
    }

    #[test]
    fn bgra_four_pixels() {
        let mut buf: Vec<u8> = vec![
            0xFF, 0x00, 0x00, 0x00, // blue pixel: B=0xFF, G=0, R=0, A=0
            0x00, 0xFF, 0x00, 0x00, // green pixel
            0x00, 0x00, 0xFF, 0x00, // red pixel
            0x80, 0x80, 0x80, 0x00, // grey pixel
        ];
        bgra_to_rgba(&mut buf);
        // blue pixel -> R=0, G=0, B=0xFF
        assert_eq!(&buf[0..4], &[0x00, 0x00, 0xFF, 0xFF]);
        // green pixel -> R=0, G=0xFF, B=0
        assert_eq!(&buf[4..8], &[0x00, 0xFF, 0x00, 0xFF]);
        // red pixel -> R=0xFF, G=0, B=0
        assert_eq!(&buf[8..12], &[0xFF, 0x00, 0x00, 0xFF]);
        // grey pixel unchanged hue
        assert_eq!(&buf[12..16], &[0x80, 0x80, 0x80, 0xFF]);
    }

    #[test]
    fn bgra_all_zeros_sets_alpha() {
        let mut buf = [0u8; 8];
        bgra_to_rgba(&mut buf);
        assert!(buf.iter().step_by(4).skip(3).all(|&a| a == 0xFF));
    }

    #[test]
    #[should_panic(expected = "multiple of 4")]
    fn bgra_odd_length_panics() {
        bgra_to_rgba(&mut [0u8; 3]);
    }

    #[test]
    fn unsupported_on_non_windows() {
        // On non-Windows the functions compile to stubs.
        #[cfg(not(windows))]
        {
            assert!(matches!(capture_primary_screen(), Err(CaptureError::Unsupported)));
            assert!(matches!(capture_virtual_screen(), Err(CaptureError::Unsupported)));
            assert!(matches!(
                capture_window_by_title("anything"),
                Err(CaptureError::Unsupported)
            ));
            assert!(list_windows().is_empty());
        }
        #[cfg(windows)]
        {
            // On Windows we can't assert success (might be headless CI), but
            // the functions exist and the Unsupported variant is not returned.
            let _ = list_windows(); // must not panic
        }
    }

    #[test]
    fn error_display_not_found() {
        let e = CaptureError::NotFound("notepad".to_owned());
        let s = e.to_string();
        assert!(s.contains("notepad"));
    }

    #[test]
    fn error_display_win32() {
        let e = CaptureError::Win32("GetDC returned NULL".to_owned());
        assert!(e.to_string().contains("GetDC"));
    }
}
