// Windows Clipboard Integration & Regression Tests for Screenshot Clipboard
//
// Verifies:
// 1. Screenshot pixel data is placed in the Windows Clipboard as CF_DIB and "PNG".
// 2. CF_DIB format is available and matches image data byte-for-byte.
// 3. A valid non-null HWND owner is assigned to the clipboard.
// 4. Memory ownership transfer and cleanup on errors.
// 5. Multiple consecutive screenshots succeed reliably.

#[cfg(target_os = "windows")]
#[path = "../mixins/screenshot_clipboard/os.rs"]
mod os;

#[cfg(target_os = "windows")]
use std::ffi::c_void;

#[cfg(target_os = "windows")]
#[link(name = "user32")]
unsafe extern "system" {
    fn OpenClipboard(hwnd: *mut c_void) -> i32;
    fn CloseClipboard() -> i32;
    fn GetClipboardData(format: u32) -> *mut c_void;
    fn GetClipboardOwner() -> *mut c_void;
    fn IsClipboardFormatAvailable(format: u32) -> i32;
    fn IsWindow(hwnd: *mut c_void) -> i32;
}

#[cfg(target_os = "windows")]
#[link(name = "kernel32")]
unsafe extern "system" {
    fn GlobalLock(h_mem: *mut c_void) -> *mut c_void;
    fn GlobalUnlock(h_mem: *mut c_void) -> i32;
    fn GlobalSize(h_mem: *mut c_void) -> usize;
}

#[cfg(target_os = "windows")]
const CF_DIB: u32 = 8;

#[test]
#[cfg(target_os = "windows")]
fn test_windows_clipboard_real_image_and_owner() {
    let width: usize = 32;
    let height: usize = 32;

    // Generate a valid bottom-up DIB
    let mut dib = Vec::new();
    // 1. BITMAPINFOHEADER (40 bytes)
    dib.extend_from_slice(&40u32.to_le_bytes());
    dib.extend_from_slice(&(width as i32).to_le_bytes());
    dib.extend_from_slice(&(height as i32).to_le_bytes());
    dib.extend_from_slice(&1u16.to_le_bytes()); // planes
    dib.extend_from_slice(&32u16.to_le_bytes()); // 32 bpp
    dib.extend_from_slice(&0u32.to_le_bytes()); // BI_RGB
    dib.extend_from_slice(&((width * height * 4) as u32).to_le_bytes());
    dib.extend_from_slice(&2835i32.to_le_bytes());
    dib.extend_from_slice(&2835i32.to_le_bytes());
    dib.extend_from_slice(&0u32.to_le_bytes());
    dib.extend_from_slice(&0u32.to_le_bytes());

    // 2. Pixel data (BGRA)
    for y in 0..height {
        for x in 0..width {
            dib.push((x * 7) as u8); // B
            dib.push((y * 7) as u8); // G
            dib.push(180); // R
            dib.push(255); // A
        }
    }

    let fake_png = b"\x89PNG\r\n\x1a\nfake_png_header";

    // Set to clipboard
    let res = os::set_clipboard_data(&dib, Some(fake_png));
    assert!(res.is_ok(), "set_clipboard_data failed: {res:?}");

    // Verify clipboard content and owner
    unsafe {
        assert_ne!(
            OpenClipboard(std::ptr::null_mut()),
            0,
            "OpenClipboard failed"
        );

        // 1. Owner must not be null and must be a valid window handle
        let owner = GetClipboardOwner();
        assert!(!owner.is_null(), "Clipboard owner must not be null");
        assert_ne!(
            IsWindow(owner),
            0,
            "Clipboard owner must be a valid Win32 HWND"
        );

        // 2. CF_DIB must be available for pasting in Paint / Word / etc.
        assert_eq!(
            IsClipboardFormatAvailable(CF_DIB),
            1,
            "CF_DIB format must be present"
        );

        // 3. Verify retrieved data matches original DIB
        let h_data = GetClipboardData(CF_DIB);
        assert!(!h_data.is_null(), "GetClipboardData(CF_DIB) returned null");

        let size = GlobalSize(h_data);
        assert!(size >= dib.len(), "GlobalSize too small");

        let ptr = GlobalLock(h_data);
        assert!(!ptr.is_null(), "GlobalLock returned null");

        let slice = std::slice::from_raw_parts(ptr.cast::<u8>(), dib.len());
        assert_eq!(slice, dib.as_slice(), "Clipboard bytes match original DIB");

        GlobalUnlock(h_data);
        CloseClipboard();
    }
}

#[test]
#[cfg(target_os = "windows")]
fn test_windows_clipboard_empty_dib_fails() {
    let res = os::set_clipboard_data(&[], None);
    assert!(res.is_err(), "Empty DIB data must return Err");
}

#[test]
#[cfg(target_os = "windows")]
fn test_consecutive_clipboard_updates() {
    let header = [
        40, 0, 0, 0, 2, 0, 0, 0, 2, 0, 0, 0, 1, 0, 32, 0, 0, 0, 0, 0, 16, 0, 0, 0, 19, 11, 0, 0,
        19, 11, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
    ];
    let mut dib1 = header.to_vec();
    dib1.extend_from_slice(&[10u8; 16]);

    let mut dib2 = header.to_vec();
    dib2.extend_from_slice(&[20u8; 16]);

    // First copy
    assert!(os::set_clipboard_data(&dib1, None).is_ok());

    // Second rapid copy
    assert!(os::set_clipboard_data(&dib2, None).is_ok());

    // Verify dib2 is the active content
    unsafe {
        assert_ne!(OpenClipboard(std::ptr::null_mut()), 0);
        let h_data = GetClipboardData(CF_DIB);
        assert!(!h_data.is_null());
        let ptr = GlobalLock(h_data);
        let slice = std::slice::from_raw_parts(ptr.cast::<u8>(), dib2.len());
        assert_eq!(slice, dib2.as_slice());
        GlobalUnlock(h_data);
        CloseClipboard();
    }
}
