// SPDX-License-Identifier: Apache-2.0
//! Windows GDI capture implementation.
//!
//! Follows the Microsoft Learn "Capturing an Image" example (Win32 C→Rust
//! port).  All GDI handles are wrapped in RAII guards so they are released
//! even on early-return error paths.
//!
//! # Safety invariants (module-wide)
//!
//! * Every `unsafe` block calls exactly one foreign function whose preconditions
//!   are documented inline.
//! * GDI handles are never stored in `&` references; ownership passes through
//!   the RAII guards.
//! * `HWND` values that arrive via `EnumWindows` callback are valid for the
//!   duration of the callback; after it returns they are only used as an
//!   opaque `usize` in `WindowInfo`.

#![deny(unsafe_op_in_unsafe_fn)]

use std::mem;

use windows::Win32::{
    Foundation::{HWND, LPARAM, RECT},
    Graphics::Gdi::{
        BitBlt, CreateCompatibleBitmap, CreateCompatibleDC, DeleteDC, DeleteObject, GetDC,
        GetDIBits, ReleaseDC, SelectObject, BITMAPINFO, BITMAPINFOHEADER, BI_RGB, DIB_RGB_COLORS,
        HBITMAP, HDC, HGDIOBJ, SRCCOPY,
    },
    UI::WindowsAndMessaging::{
        EnumWindows, GetDesktopWindow, GetSystemMetrics, GetWindowRect, GetWindowTextLengthW,
        GetWindowTextW, IsWindowVisible, SM_CXSCREEN, SM_CXVIRTUALSCREEN, SM_CYSCREEN,
        SM_CYVIRTUALSCREEN, SM_XVIRTUALSCREEN, SM_YVIRTUALSCREEN,
    },
};

use crate::{CaptureError, Result, WindowInfo};

// ─── RAII guards ─────────────────────────────────────────────────────────────

/// Owns a screen DC obtained from `GetDC(hwnd)`.  Releases it on drop via
/// `ReleaseDC(hwnd, hdc)`.
struct ScreenDc {
    hwnd: Option<HWND>,
    hdc: HDC,
}

impl ScreenDc {
    /// Acquire a screen DC.
    ///
    /// # Safety
    ///
    /// `hwnd` must be `None` (desktop) or a valid top-level window handle.
    /// The returned DC is device-context for that window's client area or, for
    /// `None`, the entire screen.
    unsafe fn acquire(hwnd: Option<HWND>) -> Result<Self> {
        // Safety: GetDC(None) always succeeds on Windows; GetDC(hwnd) succeeds
        // if hwnd is a valid window handle.  NULL return indicates failure.
        let hdc = unsafe { GetDC(hwnd) };
        if hdc.is_invalid() {
            return Err(CaptureError::Win32("GetDC returned NULL".to_owned()));
        }
        Ok(Self { hwnd, hdc })
    }
}

impl Drop for ScreenDc {
    fn drop(&mut self) {
        if !self.hdc.is_invalid() {
            // Safety: hdc was obtained from GetDC(self.hwnd) and has not been
            // released yet.  self.hwnd is the same handle passed to GetDC.
            unsafe { ReleaseDC(self.hwnd, self.hdc) };
        }
    }
}

/// Owns a memory-compatible DC created with `CreateCompatibleDC`.
struct MemDc {
    hdc: HDC,
}

impl MemDc {
    /// Create a memory DC compatible with `src`.
    ///
    /// # Safety
    ///
    /// `src` must be a valid, non-released device context.
    unsafe fn create(src: HDC) -> Result<Self> {
        // Safety: src is a valid screen DC provided by the caller.
        let hdc = unsafe { CreateCompatibleDC(Some(src)) };
        if hdc.is_invalid() {
            return Err(CaptureError::Win32("CreateCompatibleDC returned NULL".to_owned()));
        }
        Ok(Self { hdc })
    }
}

impl Drop for MemDc {
    fn drop(&mut self) {
        if !self.hdc.is_invalid() {
            // Safety: hdc was created by CreateCompatibleDC and has not been
            // deleted yet.
            let _ = unsafe { DeleteDC(self.hdc) };
        }
    }
}

/// Owns a GDI bitmap created with `CreateCompatibleBitmap`.
struct CompatBitmap {
    hbm: HBITMAP,
}

impl CompatBitmap {
    /// Create a bitmap compatible with `src_dc` of dimensions `w × h`.
    ///
    /// # Safety
    ///
    /// `src_dc` must be the *screen* DC (NOT the memory DC) so the bitmap
    /// acquires colour depth information from the display device.  `w` and `h`
    /// must be positive.
    unsafe fn create(src_dc: HDC, w: i32, h: i32) -> Result<Self> {
        // Safety: src_dc is a valid screen DC; w/h are caller-validated > 0.
        let hbm = unsafe { CreateCompatibleBitmap(src_dc, w, h) };
        if hbm.is_invalid() {
            return Err(CaptureError::Win32(
                "CreateCompatibleBitmap returned NULL".to_owned(),
            ));
        }
        Ok(Self { hbm })
    }
}

impl Drop for CompatBitmap {
    fn drop(&mut self) {
        if !self.hbm.is_invalid() {
            // Safety: hbm was created by CreateCompatibleBitmap and has not
            // been deleted yet.  HGDIOBJ conversion is safe (same pointer).
            let _ = unsafe { DeleteObject(HGDIOBJ::from(self.hbm)) };
        }
    }
}

// ─── Core blit helper ────────────────────────────────────────────────────────

/// Capture a rectangle of the screen starting at `(x, y)` with size `w × h`
/// and return it as PNG-encoded bytes.
///
/// # Safety
///
/// `screen_dc` must be a valid screen DC for the entire virtual desktop
/// (i.e. obtained from `GetDC(None)`).  `x`, `y` must be in virtual screen
/// coordinates.  `w` and `h` must be > 0.
unsafe fn blit_region_to_png(screen_dc: &ScreenDc, x: i32, y: i32, w: i32, h: i32) -> Result<Vec<u8>> {
    // 1. Create a memory DC compatible with the screen DC.
    // Safety: screen_dc.hdc is a valid screen DC.
    let mem_dc = unsafe { MemDc::create(screen_dc.hdc)? };

    // 2. Create a colour bitmap compatible with the screen DC (NOT the memory
    //    DC — using the memory DC would yield a 1-bit monochrome bitmap).
    // Safety: screen_dc.hdc is a valid screen DC; w/h are > 0.
    let bm = unsafe { CompatBitmap::create(screen_dc.hdc, w, h)? };

    // 3. Select the bitmap into the memory DC.
    //    SelectObject returns the previously selected object; we discard it
    //    because the memory DC was freshly created and holds only a stock
    //    1×1 monochrome bitmap we do not need to restore.
    //
    // Safety: mem_dc.hdc and bm.hbm are valid and compatible handles.
    unsafe { SelectObject(mem_dc.hdc, HGDIOBJ::from(bm.hbm)) };

    // 4. Blit the requested region from the screen DC into the memory DC.
    //
    // Safety: mem_dc.hdc is the destination (valid memory DC), screen_dc.hdc
    // is the source (valid screen DC), coordinates are in-bounds for the
    // virtual desktop, SRCCOPY is a valid ROP code.
    unsafe {
        BitBlt(mem_dc.hdc, 0, 0, w, h, Some(screen_dc.hdc), x, y, SRCCOPY)
            .map_err(|e| CaptureError::Win32(format!("BitBlt failed: {e}")))?;
    }

    // 5. Set up a BITMAPINFO for a top-down 32-bpp RGB DIB.
    //    biHeight < 0 requests a top-down (natural scan-line order) bitmap.
    let mut bmi = BITMAPINFO {
        bmiHeader: BITMAPINFOHEADER {
            biSize: mem::size_of::<BITMAPINFOHEADER>() as u32,
            biWidth: w,
            biHeight: -h, // negative = top-down
            biPlanes: 1,
            biBitCount: 32,
            biCompression: BI_RGB.0, // 0 = uncompressed RGB
            biSizeImage: 0,
            biXPelsPerMeter: 0,
            biYPelsPerMeter: 0,
            biClrUsed: 0,
            biClrImportant: 0,
        },
        ..Default::default()
    };

    // 6. Allocate the pixel buffer (4 bytes per pixel, BGRA layout).
    let pixel_count = (w as usize).saturating_mul(h as usize);
    let mut buf: Vec<u8> = vec![0u8; pixel_count.saturating_mul(4)];

    // 7. Retrieve DIB bits from the memory DC into our buffer.
    //
    // Safety:
    //   * mem_dc.hdc is a valid memory DC with bm.hbm selected into it.
    //   * bm.hbm is the selected HBITMAP.
    //   * buf has exactly (w * h * 4) bytes — correct for 32-bpp DIB.
    //   * &mut bmi outlives the call.
    let rows = unsafe {
        GetDIBits(
            mem_dc.hdc,
            bm.hbm,
            0,
            h as u32,
            Some(buf.as_mut_ptr().cast()),
            &mut bmi,
            DIB_RGB_COLORS,
        )
    };
    if rows == 0 {
        return Err(CaptureError::Win32("GetDIBits returned 0 scan lines".to_owned()));
    }

    // 8. The buffer is BGRA top-down; convert to RGBA for the `image` crate.
    crate::bgra_to_rgba(&mut buf);

    // 9. Encode as PNG.
    encode_rgba_to_png(&buf, w as u32, h as u32)
}

// ─── PNG encoding ────────────────────────────────────────────────────────────

fn encode_rgba_to_png(rgba: &[u8], w: u32, h: u32) -> Result<Vec<u8>> {
    use image::{ImageFormat, RgbaImage};
    let img = RgbaImage::from_raw(w, h, rgba.to_vec())
        .ok_or_else(|| CaptureError::Encode("RgbaImage::from_raw: dimensions mismatch".to_owned()))?;
    let mut out = Vec::new();
    img.write_to(&mut std::io::Cursor::new(&mut out), ImageFormat::Png)
        .map_err(|e| CaptureError::Encode(e.to_string()))?;
    Ok(out)
}

// ─── Public capture functions ─────────────────────────────────────────────────

pub(crate) fn capture_primary_screen() -> Result<Vec<u8>> {
    // Safety: GetSystemMetrics is always safe to call; SM_* constants are valid.
    let w = unsafe { GetSystemMetrics(SM_CXSCREEN) };
    let h = unsafe { GetSystemMetrics(SM_CYSCREEN) };
    if w <= 0 || h <= 0 {
        return Err(CaptureError::Win32(
            "GetSystemMetrics(SM_CXSCREEN/SM_CYSCREEN) returned non-positive value".to_owned(),
        ));
    }
    // Safety: GetDC(None) acquires the screen DC for the entire desktop.
    let dc = unsafe { ScreenDc::acquire(None)? };
    // Safety: dc.hdc is a valid screen DC; 0,0 is the origin; w,h > 0.
    unsafe { blit_region_to_png(&dc, 0, 0, w, h) }
}

pub(crate) fn capture_virtual_screen() -> Result<Vec<u8>> {
    // Safety: GetSystemMetrics is always safe to call.
    let x = unsafe { GetSystemMetrics(SM_XVIRTUALSCREEN) };
    let y = unsafe { GetSystemMetrics(SM_YVIRTUALSCREEN) };
    let w = unsafe { GetSystemMetrics(SM_CXVIRTUALSCREEN) };
    let h = unsafe { GetSystemMetrics(SM_CYVIRTUALSCREEN) };
    if w <= 0 || h <= 0 {
        return Err(CaptureError::Win32(
            "GetSystemMetrics(SM_CXVIRTUALSCREEN/SM_CYVIRTUALSCREEN) returned non-positive value"
                .to_owned(),
        ));
    }
    // Safety: GetDC(None) acquires the screen DC.
    let dc = unsafe { ScreenDc::acquire(None)? };
    // Safety: dc.hdc is valid; x,y are virtual-screen offsets; w,h > 0.
    unsafe { blit_region_to_png(&dc, x, y, w, h) }
}

pub(crate) fn capture_window_by_title(substr: &str) -> Result<Vec<u8>> {
    let hwnd = find_window_by_title_substr(substr)
        .ok_or_else(|| CaptureError::NotFound(substr.to_owned()))?;

    let mut rect = RECT::default();
    // Safety: hwnd was found by EnumWindows and is a valid top-level window.
    // GetWindowRect stores the window rectangle (screen coordinates) in rect.
    unsafe {
        GetWindowRect(hwnd, &mut rect)
            .map_err(|e| CaptureError::Win32(format!("GetWindowRect failed: {e}")))?;
    }

    let x = rect.left;
    let y = rect.top;
    let w = rect.right - rect.left;
    let h = rect.bottom - rect.top;

    if w <= 0 || h <= 0 {
        return Err(CaptureError::Win32(format!(
            "window '{substr}' has zero or negative dimensions ({w}×{h})"
        )));
    }

    // Capture from the screen DC (includes window decorations + any composited
    // content visible through the window rectangle).
    // Safety: GetDC(None) acquires the screen DC.
    let dc = unsafe { ScreenDc::acquire(None)? };
    // Safety: dc.hdc is valid; x,y are screen coordinates; w,h > 0.
    unsafe { blit_region_to_png(&dc, x, y, w, h) }
}

// ─── Window enumeration ───────────────────────────────────────────────────────

/// Collect all visible top-level windows with a non-empty title.
pub(crate) fn list_windows() -> Vec<WindowInfo> {
    let mut out: Vec<WindowInfo> = Vec::new();
    let ptr = &mut out as *mut Vec<WindowInfo>;

    // Safety: EnumWindows iterates every top-level window and calls our
    // callback for each.  The LPARAM carries a raw pointer to `out`; it is
    // valid for the entire duration of the EnumWindows call because `out` is
    // on the current stack frame and EnumWindows is synchronous.
    let _ = unsafe {
        EnumWindows(
            Some(enum_windows_callback),
            LPARAM(ptr as isize),
        )
    };

    out
}

/// `EnumWindows` callback: collect visible windows with non-empty titles.
///
/// # Safety
///
/// Called by the OS.  `lparam` must be a valid `*mut Vec<WindowInfo>`.
/// `hwnd` is a valid window handle for the duration of this call.
unsafe extern "system" fn enum_windows_callback(hwnd: HWND, lparam: LPARAM) -> windows::core::BOOL {
    // Safety: lparam was set to a `*mut Vec<WindowInfo>` in list_windows().
    let out = unsafe { &mut *(lparam.0 as *mut Vec<WindowInfo>) };

    // Skip invisible windows.
    // Safety: hwnd is a valid window handle provided by EnumWindows.
    let visible = unsafe { IsWindowVisible(hwnd) };
    if !visible.as_bool() {
        return true.into();
    }

    // Query title length (excludes NUL terminator).
    // Safety: hwnd is valid.
    let len = unsafe { GetWindowTextLengthW(hwnd) };
    if len <= 0 {
        return true.into();
    }

    // Retrieve the title text.
    let mut title_buf: Vec<u16> = vec![0u16; (len as usize) + 1];
    // Safety: hwnd is valid; title_buf has capacity len+1.
    let copied = unsafe { GetWindowTextW(hwnd, &mut title_buf) };
    if copied <= 0 {
        return true.into();
    }
    title_buf.truncate(copied as usize);
    let title = String::from_utf16_lossy(&title_buf);

    out.push(WindowInfo {
        title,
        handle: hwnd.0 as usize,
    });

    true.into() // continue enumeration
}

/// Find the first visible top-level window whose title contains `substr`.
fn find_window_by_title_substr(substr: &str) -> Option<HWND> {
    // Re-use list_windows() for simplicity; avoids a second EnumWindows
    // callback implementation.
    list_windows()
        .into_iter()
        .find(|w| w.title.contains(substr))
        .map(|w| HWND(w.handle as *mut _))
}

// suppress unused import warning for GetDesktopWindow (available for future use)
#[allow(dead_code)]
fn _desktop_window() -> HWND {
    // Safety: GetDesktopWindow always succeeds and returns the desktop HWND.
    unsafe { GetDesktopWindow() }
}
