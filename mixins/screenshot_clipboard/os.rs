// Screenshot Clipboard - OS Clipboard Integration (horizon-threads)
//
// Native Win32 clipboard integration for Horizon Minecraft.
// Interacts with user32.dll and kernel32.dll to set CF_DIB and registered "PNG" data.

#[cfg(target_os = "windows")]
use std::cell::Cell;
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
    fn IsWindow(hwnd: *mut c_void) -> i32;
    fn GetActiveWindow() -> *mut c_void;
    fn GetForegroundWindow() -> *mut c_void;
    fn GetWindowThreadProcessId(hwnd: *mut c_void, lpdw_process_id: *mut u32) -> u32;
    fn CreateWindowExA(
        dw_ex_style: u32,
        lp_class_name: *const u8,
        lp_window_name: *const u8,
        dw_style: u32,
        x: i32,
        y: i32,
        n_width: i32,
        n_height: i32,
        h_wnd_parent: *mut c_void,
        h_menu: *mut c_void,
        h_instance: *mut c_void,
        lp_param: *mut c_void,
    ) -> *mut c_void;
}

#[cfg(target_os = "windows")]
#[link(name = "kernel32")]
unsafe extern "system" {
    fn GetLastError() -> u32;
    fn GetCurrentProcessId() -> u32;
    fn GlobalAlloc(flags: u32, bytes: usize) -> *mut c_void;
    fn GlobalLock(h_mem: *mut c_void) -> *mut c_void;
    fn GlobalUnlock(h_mem: *mut c_void) -> i32;
    fn GlobalFree(h_mem: *mut c_void) -> *mut c_void;
}

/// RAII wrapper for moveable global memory blocks allocated via `GlobalAlloc`.
///
/// Ensures memory is freed on error, and ownership can be cleanly transferred
/// to the Windows clipboard manager upon successful `SetClipboardData`.
#[cfg(target_os = "windows")]
pub struct GlobalMemory {
    handle: *mut c_void,
}

#[cfg(target_os = "windows")]
impl GlobalMemory {
    pub fn copy_from_slice(data: &[u8]) -> Result<Self, String> {
        if data.is_empty() {
            return Err("empty data slice".into());
        }
        let handle = unsafe { GlobalAlloc(GMEM_MOVEABLE, data.len()) };
        if handle.is_null() {
            let err = unsafe { GetLastError() };
            return Err(format!("GlobalAlloc failed with Win32 error {err}"));
        }
        let ptr = unsafe { GlobalLock(handle) };
        if ptr.is_null() {
            let err = unsafe { GetLastError() };
            unsafe { GlobalFree(handle) };
            return Err(format!("GlobalLock failed with Win32 error {err}"));
        }
        unsafe {
            std::ptr::copy_nonoverlapping(data.as_ptr(), ptr.cast::<u8>(), data.len());
            GlobalUnlock(handle);
        }
        Ok(Self { handle })
    }

    /// Relinquishes ownership of the memory handle (transferred to Windows).
    pub fn into_raw(mut self) -> *mut c_void {
        let h = self.handle;
        self.handle = std::ptr::null_mut();
        h
    }

    pub fn raw(&self) -> *mut c_void {
        self.handle
    }
}

#[cfg(target_os = "windows")]
impl Drop for GlobalMemory {
    fn drop(&mut self) {
        if !self.handle.is_null() {
            unsafe { GlobalFree(self.handle) };
        }
    }
}

/// RAII guard ensuring `CloseClipboard` is called after successful `OpenClipboard`.
#[cfg(target_os = "windows")]
struct ClipboardGuard;

#[cfg(target_os = "windows")]
impl Drop for ClipboardGuard {
    fn drop(&mut self) {
        unsafe {
            CloseClipboard();
        }
    }
}

#[cfg(target_os = "windows")]
thread_local! {
    static THREAD_SERVICE_HWND: Cell<*mut c_void> = const { Cell::new(std::ptr::null_mut()) };
}

/// Obtains a valid clipboard owner `HWND` belonging to current process.
///
/// Prioritizes existing active/foreground windows belonging to this process.
/// Falls back to a persistent message-only utility window (`HWND_MESSAGE`)
/// created specifically for the current thread's lifetime.
#[cfg(target_os = "windows")]
unsafe fn get_or_create_owner_hwnd() -> Result<*mut c_void, String> {
    let current_pid = unsafe { GetCurrentProcessId() };

    for candidate in unsafe { [GetActiveWindow(), GetForegroundWindow()] } {
        if !candidate.is_null() && unsafe { IsWindow(candidate) } != 0 {
            let mut pid = 0u32;
            unsafe { GetWindowThreadProcessId(candidate, &mut pid) };
            if pid == current_pid {
                return Ok(candidate);
            }
        }
    }

    let existing = THREAD_SERVICE_HWND.with(|cell| cell.get());
    if !existing.is_null() && unsafe { IsWindow(existing) } != 0 {
        return Ok(existing);
    }

    // HWND_MESSAGE is ((HWND)-3)
    let hwnd_message = (-3isize) as *mut c_void;
    let hwnd = unsafe {
        CreateWindowExA(
            0,
            c"STATIC".as_ptr().cast(),
            c"HorizonScreenshotClipboard".as_ptr().cast(),
            0,
            0,
            0,
            0,
            0,
            hwnd_message,
            std::ptr::null_mut(),
            std::ptr::null_mut(),
            std::ptr::null_mut(),
        )
    };

    if hwnd.is_null() {
        let err = unsafe { GetLastError() };
        return Err(format!(
            "CreateWindowExA failed to create service window (Win32 error {err})"
        ));
    }

    THREAD_SERVICE_HWND.with(|cell| cell.set(hwnd));
    Ok(hwnd)
}

/// Sets image data on the Windows clipboard.
///
/// Places both `CF_DIB` (for universal native Windows support like MS Paint, Word, etc.)
/// and registered `"PNG"` format (for Discord, Telegram, and web browsers).
///
/// Follows Microsoft Win32 requirements:
/// - Uses a valid `HWND` owner for `OpenClipboard` to prevent `EmptyClipboard` null-owner failures.
/// - Validates returns of `OpenClipboard`, `EmptyClipboard`, and `SetClipboardData`.
/// - Guarantees `CloseClipboard` via RAII guard.
/// - Properly transfers memory ownership to Windows on success, and frees on error.
#[cfg(target_os = "windows")]
pub fn set_clipboard_data(dib_data: &[u8], png_data: Option<&[u8]>) -> Result<(), String> {
    if dib_data.is_empty() {
        return Err("empty DIB data".into());
    }

    // 1. Allocate and copy global memory blocks before touching the clipboard
    let mem_dib = GlobalMemory::copy_from_slice(dib_data)?;
    let mem_png = if let Some(png) = png_data {
        if !png.is_empty() {
            Some(GlobalMemory::copy_from_slice(png)?)
        } else {
            None
        }
    } else {
        None
    };

    let png_format = unsafe { RegisterClipboardFormatA(c"PNG".as_ptr().cast()) };

    // 2. Obtain a valid window handle belonging to this process
    let hwnd_owner = unsafe { get_or_create_owner_hwnd()? };

    // 3. Retry OpenClipboard up to 10 times with small backoff in case another app holds it briefly
    let mut opened = false;
    let mut last_err = 0u32;
    for _ in 0..10 {
        if unsafe { OpenClipboard(hwnd_owner) } != 0 {
            opened = true;
            break;
        }
        last_err = unsafe { GetLastError() };
        std::thread::sleep(std::time::Duration::from_millis(5));
    }

    if !opened {
        return Err(format!(
            "failed to open Windows clipboard (Win32 error {last_err})"
        ));
    }

    // Guard guarantees CloseClipboard is called on any exit path
    let _guard = ClipboardGuard;

    // 4. Empty clipboard and assign ownership to hwnd_owner
    if unsafe { EmptyClipboard() } == 0 {
        let err = unsafe { GetLastError() };
        return Err(format!(
            "failed to empty Windows clipboard (Win32 error {err})"
        ));
    }

    // 5. Set CF_DIB (primary format for native Windows paste)
    let dib_handle = mem_dib.raw();
    let set_dib = unsafe { SetClipboardData(CF_DIB, dib_handle) };
    if set_dib.is_null() {
        let err = unsafe { GetLastError() };
        return Err(format!(
            "SetClipboardData(CF_DIB) failed (Win32 error {err})"
        ));
    }
    // Successfully transferred ownership of mem_dib to Windows
    let _ = mem_dib.into_raw();

    // 6. Set "PNG" if available and registered
    if let (true, Some(png_mem)) = (png_format != 0, mem_png) {
        let png_handle = png_mem.raw();
        let set_png = unsafe { SetClipboardData(png_format, png_handle) };
        if !set_png.is_null() {
            // Successfully transferred ownership of png_mem to Windows
            let _ = png_mem.into_raw();
        }
        // If PNG registration/setting fails, png_mem is freed on drop; CF_DIB remains valid
    }

    Ok(())
}

/// Non-Windows fallback for headless testing and cross-compilation check.
#[cfg(not(target_os = "windows"))]
pub fn set_clipboard_data(_dib_data: &[u8], _png_data: Option<&[u8]>) -> Result<(), String> {
    Ok(())
}
