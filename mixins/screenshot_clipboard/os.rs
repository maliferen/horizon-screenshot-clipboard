// Screenshot Clipboard - OS Clipboard Integration (horizon-threads)
//
// Native Win32 clipboard integration for Horizon Minecraft.
// Interacts with user32.dll and kernel32.dll to set CF_DIB and registered "PNG" data.

#[cfg(target_os = "windows")]
use std::ffi::c_void;

#[cfg(target_os = "windows")]
const CF_DIB: u32 = 8;
#[cfg(target_os = "windows")]
const GMEM_MOVEABLE: u32 = 0x0002;

#[cfg(target_os = "windows")]
#[link(name = "user32")]
unsafe extern "system" {
    fn OpenClipboard(hwnd: *mut c_void) -> i32;
    fn CloseClipboard() -> i32;
    fn EmptyClipboard() -> i32;
    fn SetClipboardData(format: u32, h_mem: *mut c_void) -> *mut c_void;
    fn RegisterClipboardFormatA(format_name: *const u8) -> u32;
}

#[cfg(target_os = "windows")]
#[link(name = "kernel32")]
unsafe extern "system" {
    fn GlobalAlloc(flags: u32, bytes: usize) -> *mut c_void;
    fn GlobalLock(h_mem: *mut c_void) -> *mut c_void;
    fn GlobalUnlock(h_mem: *mut c_void) -> i32;
    fn GlobalFree(h_mem: *mut c_void) -> *mut c_void;
}

#[cfg(target_os = "windows")]
unsafe fn allocate_global_copy(data: &[u8]) -> Result<*mut c_void, String> {
    let handle = unsafe { GlobalAlloc(GMEM_MOVEABLE, data.len()) };
    if handle.is_null() {
        return Err("GlobalAlloc returned null".into());
    }
    let ptr = unsafe { GlobalLock(handle) };
    if ptr.is_null() {
        unsafe { GlobalFree(handle) };
        return Err("GlobalLock returned null".into());
    }
    unsafe {
        std::ptr::copy_nonoverlapping(data.as_ptr(), ptr.cast::<u8>(), data.len());
        GlobalUnlock(handle);
    }
    Ok(handle)
}

/// Sets image data on the Windows clipboard.
///
/// Places both `CF_DIB` (for universal native Windows support like MS Paint, Word, etc.)
/// and registered `"PNG"` format (for Discord, Telegram, and web browsers).
#[cfg(target_os = "windows")]
pub fn set_clipboard_data(dib_data: &[u8], png_data: Option<&[u8]>) -> Result<(), String> {
    if dib_data.is_empty() {
        return Err("empty DIB data".into());
    }

    let png_format = unsafe { RegisterClipboardFormatA(b"PNG\0".as_ptr()) };

    let h_dib = unsafe { allocate_global_copy(dib_data)? };
    let h_png = if let Some(png) = png_data {
        unsafe { allocate_global_copy(png).ok() }
    } else {
        None
    };

    // Retry OpenClipboard up to 10 times with small backoff in case another app holds it briefly
    let mut opened = false;
    for _ in 0..10 {
        if unsafe { OpenClipboard(std::ptr::null_mut()) } != 0 {
            opened = true;
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(5));
    }

    if !opened {
        unsafe {
            GlobalFree(h_dib);
            if let Some(h) = h_png {
                GlobalFree(h);
            }
        }
        return Err("failed to open Windows clipboard".into());
    }

    unsafe {
        if EmptyClipboard() == 0 {
            GlobalFree(h_dib);
            if let Some(h) = h_png {
                GlobalFree(h);
            }
            CloseClipboard();
            return Err("failed to empty clipboard".into());
        }

        // Set CF_DIB (system owns memory on success)
        let set_dib = SetClipboardData(CF_DIB, h_dib);
        if set_dib.is_null() {
            GlobalFree(h_dib);
        }

        // Set "PNG" if available and registered
        if png_format != 0 {
            if let Some(h) = h_png {
                let set_png = SetClipboardData(png_format, h);
                if set_png.is_null() {
                    GlobalFree(h);
                }
            }
        }

        CloseClipboard();
    }

    Ok(())
}

/// Non-Windows fallback for headless testing and cross-compilation check.
#[cfg(not(target_os = "windows"))]
pub fn set_clipboard_data(_dib_data: &[u8], _png_data: Option<&[u8]>) -> Result<(), String> {
    Ok(())
}
