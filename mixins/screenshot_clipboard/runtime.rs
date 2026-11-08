// Screenshot Clipboard Runtime
//
// Handles screenshot file processing, PNG decoding, DIB/BMP format conversion,
// and native Windows Clipboard integration for Horizon Minecraft 26.3.

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};

pub const ID: &str = "screenshot_clipboard";
pub const MOD_NAME: &str = "Screenshot Clipboard";
pub const MOD_VERSION: &str = "0.1.0";

/// Global state: mod enabled / disabled (defaults to true).
static ENABLED: AtomicBool = AtomicBool::new(true);

/// Tracks whether the setting has been loaded from storage at least once.
static INITIALIZED: AtomicBool = AtomicBool::new(false);

/// Monotonic sequence counter to guarantee that when multiple screenshots
/// occur rapidly, only the latest successfully completed capture wins in clipboard.
static LATEST_SEQ: AtomicU64 = AtomicU64::new(0);

/// Toast display duration in milliseconds (3.0 seconds).
pub const TOAST_DURATION_MS: u64 = 3000;
/// Time while progress bar fills from 0% to 100% (2.5 seconds).
pub const TOAST_FILL_MS: u64 = 2500;
/// Toast fade-out duration after fill completes (0.5 seconds).
pub const TOAST_FADE_MS: u64 = 500;

/// Timestamp in milliseconds when the active toast expires.
static TOAST_EXPIRY: AtomicU64 = AtomicU64::new(0);

/// Active state of the corner toast banner (defaults to true).
static TOAST_ENABLED: AtomicBool = AtomicBool::new(true);
static TOAST_INITIALIZED: AtomicBool = AtomicBool::new(false);

/// Represents the instantaneous visual state of the toast banner.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ToastState {
    /// Opacity between 0.0 and 1.0.
    pub alpha: f32,
    /// Progress bar completion between 0.0 and 1.0.
    pub progress: f32,
}

/// Returns current wall-clock timestamp in milliseconds.
pub fn current_timestamp_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

/// Triggers or refreshes the in-game Minecraft toast banner.
pub fn trigger_toast() {
    let now = current_timestamp_ms();
    TOAST_EXPIRY.store(now.saturating_add(TOAST_DURATION_MS), Ordering::SeqCst);
}

/// Computes current toast state (opacity and fill progress), or None if expired.
pub fn toast_state() -> Option<ToastState> {
    let expiry = TOAST_EXPIRY.load(Ordering::Relaxed);
    let now = current_timestamp_ms();
    if now >= expiry {
        return None;
    }
    let total_ms = TOAST_DURATION_MS;
    let remaining_ms = expiry.saturating_sub(now);
    let elapsed_ms = total_ms.saturating_sub(remaining_ms);

    let progress = (elapsed_ms as f32 / TOAST_FILL_MS as f32).clamp(0.0, 1.0);
    let alpha = if remaining_ms <= TOAST_FADE_MS {
        (remaining_ms as f32 / TOAST_FADE_MS as f32).clamp(0.0, 1.0)
    } else {
        1.0
    };

    Some(ToastState { alpha, progress })
}

/// Check if mod is enabled, loading default from persistence closure if uninitialized.
pub fn is_enabled(read_persisted: impl FnOnce() -> Option<bool>) -> bool {
    if !INITIALIZED.load(Ordering::Relaxed) {
        let val = read_persisted().unwrap_or(true);
        ENABLED.store(val, Ordering::SeqCst);
        INITIALIZED.store(true, Ordering::SeqCst);
        val
    } else {
        ENABLED.load(Ordering::SeqCst)
    }
}

/// Set active state live.
pub fn set_enabled(val: bool) {
    ENABLED.store(val, Ordering::SeqCst);
    INITIALIZED.store(true, Ordering::SeqCst);
}

/// Check if toast banner is enabled, loading default from persistence closure if uninitialized.
pub fn is_toast_enabled(read_persisted: impl FnOnce() -> Option<bool>) -> bool {
    if !TOAST_INITIALIZED.load(Ordering::Relaxed) {
        let val = read_persisted().unwrap_or(true);
        TOAST_ENABLED.store(val, Ordering::SeqCst);
        TOAST_INITIALIZED.store(true, Ordering::SeqCst);
        val
    } else {
        TOAST_ENABLED.load(Ordering::SeqCst)
    }
}

/// Set toast banner active state live.
pub fn set_toast_enabled(val: bool) {
    TOAST_ENABLED.store(val, Ordering::SeqCst);
    TOAST_INITIALIZED.store(true, Ordering::SeqCst);
}

/// Returns localized button label for master toggle.
pub fn settings_label(enabled: bool, is_ru: bool) -> String {
    if is_ru {
        if enabled {
            "Копирование скриншота: ВКЛ".to_string()
        } else {
            "Копирование скриншота: ВЫКЛ".to_string()
        }
    } else if enabled {
        "Screenshot Clipboard: ON".to_string()
    } else {
        "Screenshot Clipboard: OFF".to_string()
    }
}

/// Returns localized button label for corner toast notification toggle.
pub fn toast_settings_label(enabled: bool, is_ru: bool) -> String {
    if is_ru {
        if enabled {
            "Уведомление в углу: ВКЛ".to_string()
        } else {
            "Уведомление в углу: ВЫКЛ".to_string()
        }
    } else if enabled {
        "Corner Notification: ON".to_string()
    } else {
        "Corner Notification: OFF".to_string()
    }
}

/// Current sequence number.
pub fn current_seq() -> u64 {
    LATEST_SEQ.load(Ordering::SeqCst)
}

/// Next sequence number for an incoming capture.
pub fn next_seq() -> u64 {
    LATEST_SEQ.fetch_add(1, Ordering::SeqCst) + 1
}

// ---------------------------------------------------------------------------
// DIB (Device Independent Bitmap) Structures & Conversion
// ---------------------------------------------------------------------------

pub const CF_DIB: u32 = 8;
pub const BI_RGB: u32 = 0;

/// Windows BITMAPINFOHEADER structure (40 bytes).
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BitmapInfoHeader {
    pub bi_size: u32,             // 40
    pub bi_width: i32,            // Image width
    pub bi_height: i32,           // Image height (positive = bottom-up DIB)
    pub bi_planes: u16,           // 1
    pub bi_bit_count: u16,        // 32 bits per pixel
    pub bi_compression: u32,      // 0 = BI_RGB (uncompressed)
    pub bi_size_image: u32,       // Pixel data size in bytes
    pub bi_x_pels_per_meter: i32, // Horizontal resolution (e.g. 2835 = 72 DPI)
    pub bi_y_pels_per_meter: i32, // Vertical resolution
    pub bi_clr_used: u32,         // 0
    pub bi_clr_important: u32,    // 0
}

impl BitmapInfoHeader {
    pub const SIZE: usize = 40;

    pub fn new(width: u32, height: u32) -> Result<Self, &'static str> {
        if width == 0 || height == 0 {
            return Err("Image dimensions must be non-zero");
        }
        if width > 32768 || height > 32768 {
            return Err("Image dimensions exceed maximum supported limit");
        }
        let size_image = (width as u64)
            .checked_mul(height as u64)
            .and_then(|px| px.checked_mul(4))
            .ok_or("Image byte size overflow")?;

        if size_image > u32::MAX as u64 {
            return Err("Image byte size exceeds 32-bit addressable DIB limit");
        }

        Ok(Self {
            bi_size: 40,
            bi_width: width as i32,
            bi_height: height as i32, // Positive = bottom-up
            bi_planes: 1,
            bi_bit_count: 32,
            bi_compression: BI_RGB,
            bi_size_image: size_image as u32,
            bi_x_pels_per_meter: 2835,
            bi_y_pels_per_meter: 2835,
            bi_clr_used: 0,
            bi_clr_important: 0,
        })
    }

    pub fn write_to_vec(&self, out: &mut Vec<u8>) {
        out.extend_from_slice(&self.bi_size.to_le_bytes());
        out.extend_from_slice(&self.bi_width.to_le_bytes());
        out.extend_from_slice(&self.bi_height.to_le_bytes());
        out.extend_from_slice(&self.bi_planes.to_le_bytes());
        out.extend_from_slice(&self.bi_bit_count.to_le_bytes());
        out.extend_from_slice(&self.bi_compression.to_le_bytes());
        out.extend_from_slice(&self.bi_size_image.to_le_bytes());
        out.extend_from_slice(&self.bi_x_pels_per_meter.to_le_bytes());
        out.extend_from_slice(&self.bi_y_pels_per_meter.to_le_bytes());
        out.extend_from_slice(&self.bi_clr_used.to_le_bytes());
        out.extend_from_slice(&self.bi_clr_important.to_le_bytes());
    }

    pub fn from_bytes(bytes: &[u8]) -> Result<Self, &'static str> {
        if bytes.len() < Self::SIZE {
            return Err("buffer too short for BitmapInfoHeader");
        }
        let bi_size = u32::from_le_bytes(bytes[0..4].try_into().unwrap());
        let bi_width = i32::from_le_bytes(bytes[4..8].try_into().unwrap());
        let bi_height = i32::from_le_bytes(bytes[8..12].try_into().unwrap());
        let bi_planes = u16::from_le_bytes(bytes[12..14].try_into().unwrap());
        let bi_bit_count = u16::from_le_bytes(bytes[14..16].try_into().unwrap());
        let bi_compression = u32::from_le_bytes(bytes[16..20].try_into().unwrap());
        let bi_size_image = u32::from_le_bytes(bytes[20..24].try_into().unwrap());
        let bi_x_pels_per_meter = i32::from_le_bytes(bytes[24..28].try_into().unwrap());
        let bi_y_pels_per_meter = i32::from_le_bytes(bytes[28..32].try_into().unwrap());
        let bi_clr_used = u32::from_le_bytes(bytes[32..36].try_into().unwrap());
        let bi_clr_important = u32::from_le_bytes(bytes[36..40].try_into().unwrap());

        Ok(Self {
            bi_size,
            bi_width,
            bi_height,
            bi_planes,
            bi_bit_count,
            bi_compression,
            bi_size_image,
            bi_x_pels_per_meter,
            bi_y_pels_per_meter,
            bi_clr_used,
            bi_clr_important,
        })
    }
}

/// Converts top-down RGBA pixels (standard Minecraft / PNG format)
/// into standard Windows bottom-up BGRA CF_DIB memory block.
pub fn rgba_to_bottom_up_dib(
    rgba: &[u8],
    width: u32,
    height: u32,
) -> Result<Vec<u8>, &'static str> {
    let header = BitmapInfoHeader::new(width, height)?;
    let w = width as usize;
    let h = height as usize;
    let expected_len = w
        .checked_mul(h)
        .and_then(|n| n.checked_mul(4))
        .ok_or("Dimension overflow")?;

    if rgba.len() < expected_len {
        return Err("Input RGBA pixel slice is smaller than width * height * 4");
    }

    let total_size = BitmapInfoHeader::SIZE
        .checked_add(header.bi_size_image as usize)
        .ok_or("Total size overflow")?;
    let mut dib = Vec::with_capacity(total_size);

    // 1. Write BITMAPINFOHEADER bytes
    header.write_to_vec(&mut dib);

    // 2. Write pixels bottom-up: from row (height - 1) down to row 0.
    // Each pixel converted from [R, G, B, A] to [B, G, R, 255] (fully opaque for universal pasting).
    for row in (0..h).rev() {
        let row_start = row * w * 4;
        for col in 0..w {
            let px_idx = row_start + col * 4;
            let r = rgba[px_idx];
            let g = rgba[px_idx + 1];
            let b = rgba[px_idx + 2];
            // Format for CF_DIB 32-bit: B, G, R, A
            dib.push(b);
            dib.push(g);
            dib.push(r);
            dib.push(255);
        }
    }

    Ok(dib)
}

/// Converts top-down RGB pixels into standard Windows bottom-up BGRA CF_DIB memory block.
pub fn rgb_to_bottom_up_dib(rgb: &[u8], width: u32, height: u32) -> Result<Vec<u8>, &'static str> {
    let header = BitmapInfoHeader::new(width, height)?;
    let w = width as usize;
    let h = height as usize;
    let expected_len = w
        .checked_mul(h)
        .and_then(|n| n.checked_mul(3))
        .ok_or("Dimension overflow")?;

    if rgb.len() < expected_len {
        return Err("Input RGB pixel slice is smaller than width * height * 3");
    }

    let total_size = BitmapInfoHeader::SIZE
        .checked_add(header.bi_size_image as usize)
        .ok_or("Total size overflow")?;
    let mut dib = Vec::with_capacity(total_size);

    header.write_to_vec(&mut dib);

    for row in (0..h).rev() {
        let row_start = row * w * 3;
        for col in 0..w {
            let px_idx = row_start + col * 3;
            let r = rgb[px_idx];
            let g = rgb[px_idx + 1];
            let b = rgb[px_idx + 2];
            dib.push(b);
            dib.push(g);
            dib.push(r);
            dib.push(255);
        }
    }

    Ok(dib)
}

/// Decodes PNG bytes into `(width, height, dib_bytes)` suitable for Windows clipboard.
pub fn png_to_dib(png_bytes: &[u8]) -> Result<(u32, u32, Vec<u8>), String> {
    let cursor = std::io::Cursor::new(png_bytes);
    let decoder = png::Decoder::new(cursor);
    let mut reader = decoder
        .read_info()
        .map_err(|e| format!("PNG read error: {e}"))?;
    let info = reader.info().clone();
    let width = info.width;
    let height = info.height;

    let buf_size = reader
        .output_buffer_size()
        .unwrap_or((width as usize) * (height as usize) * 4);
    let mut raw_pixels = vec![0u8; buf_size];

    let output_info = reader
        .next_frame(&mut raw_pixels)
        .map_err(|e| format!("PNG decode error: {e}"))?;

    let dib = match output_info.color_type {
        png::ColorType::Rgba => rgba_to_bottom_up_dib(&raw_pixels, width, height)
            .map_err(|e| format!("RGBA to DIB error: {e}"))?,
        png::ColorType::Rgb => rgb_to_bottom_up_dib(&raw_pixels, width, height)
            .map_err(|e| format!("RGB to DIB error: {e}"))?,
        other => {
            return Err(format!(
                "Unsupported PNG color type for clipboard: {other:?}"
            ));
        }
    };

    Ok((width, height, dib))
}

// ---------------------------------------------------------------------------
// Native Windows 10/11 Clipboard Implementation
// ---------------------------------------------------------------------------

#[cfg(windows)]
mod os_backend {
    pub fn write_image_to_clipboard(dib_data: &[u8], png_data: &[u8]) -> Result<(), String> {
        #[cfg(not(feature = "standalone"))]
        {
            horizon_threads::screenshot_clipboard_os::set_clipboard_data(dib_data, Some(png_data))
        }
        #[cfg(feature = "standalone")]
        {
            let _ = (dib_data, png_data);
            Ok(())
        }
    }
}

#[cfg(not(windows))]
mod os_backend {
    pub fn write_image_to_clipboard(_dib_data: &[u8], _png_data: &[u8]) -> Result<(), String> {
        Err("Screenshot Clipboard currently natively supports Windows 10/11".into())
    }
}

/// High-level function that copies PNG file at `path` to system clipboard.
/// Ensures thread-safety and monotonic ordering under rapid screenshot events.
pub fn copy_screenshot_to_clipboard(path: &Path, seq: u64) -> Result<(u32, u32), String> {
    if !path.is_file() {
        return Err(format!(
            "Screenshot file does not exist: {}",
            path.display()
        ));
    }

    let png_bytes =
        std::fs::read(path).map_err(|e| format!("Could not read screenshot file: {e}"))?;
    let (width, height, dib_bytes) = png_to_dib(&png_bytes)?;

    // Monotonic guard: if a newer screenshot has finished while we were decoding, skip!
    if seq < current_seq() {
        log::info!(
            "Screenshot Clipboard: skipping seq {} in favor of newer capture {}",
            seq,
            current_seq()
        );
        return Ok((width, height));
    }

    os_backend::write_image_to_clipboard(&dib_bytes, &png_bytes)?;

    Ok((width, height))
}

/// Asynchronous trigger invoked when a screenshot file path is received.
pub fn handle_screenshot_path(path_str: &str) {
    if !ENABLED.load(Ordering::SeqCst) {
        return;
    }
    if TOAST_ENABLED.load(Ordering::SeqCst) {
        trigger_toast();
    }
    let path = PathBuf::from(path_str);
    let seq = next_seq();
    std::thread::Builder::new()
        .name("sc-clipboard".into())
        .spawn(move || match copy_screenshot_to_clipboard(&path, seq) {
            Ok((w, h)) => {
                log::info!(
                    "Screenshot Clipboard: copied {} to clipboard ({}x{})",
                    path.display(),
                    w,
                    h
                );
            }
            Err(e) => {
                log::warn!("Screenshot Clipboard error: {e}");
            }
        })
        .ok();
}

// ---------------------------------------------------------------------------
// Unit Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_toast_trigger_and_decay() {
        // Reset expiry
        TOAST_EXPIRY.store(0, Ordering::SeqCst);
        assert!(toast_state().is_none());

        trigger_toast();
        let state = toast_state().expect("toast should be active");
        assert!(state.alpha > 0.0 && state.alpha <= 1.0);
        assert!(state.progress >= 0.0 && state.progress <= 1.0);

        // Manually simulate expiration
        TOAST_EXPIRY.store(1, Ordering::SeqCst);
        assert!(toast_state().is_none());
    }

    #[test]
    fn test_settings_toggle() {
        set_enabled(true);
        assert!(is_enabled(|| Some(false)));
        assert_eq!(settings_label(true, false), "Screenshot Clipboard: ON");
        assert_eq!(settings_label(true, true), "Копирование скриншота: ВКЛ");

        set_enabled(false);
        assert!(!is_enabled(|| Some(true)));
        assert_eq!(settings_label(false, false), "Screenshot Clipboard: OFF");
        assert_eq!(settings_label(false, true), "Копирование скриншота: ВЫКЛ");
    }

    #[test]
    fn test_toast_settings_toggle() {
        set_toast_enabled(true);
        assert!(is_toast_enabled(|| Some(false)));
        assert_eq!(toast_settings_label(true, false), "Corner Notification: ON");
        assert_eq!(toast_settings_label(true, true), "Уведомление в углу: ВКЛ");

        set_toast_enabled(false);
        assert!(!is_toast_enabled(|| Some(true)));
        assert_eq!(
            toast_settings_label(false, false),
            "Corner Notification: OFF"
        );
        assert_eq!(
            toast_settings_label(false, true),
            "Уведомление в углу: ВЫКЛ"
        );
    }

    #[test]
    fn test_monotonic_sequence() {
        let s1 = next_seq();
        let s2 = next_seq();
        assert!(s2 > s1);
        assert_eq!(current_seq(), s2);
    }

    #[test]
    fn test_bitmap_info_header_validity() {
        let h = BitmapInfoHeader::new(1920, 1080).expect("1080p header");
        assert_eq!(h.bi_size, 40);
        assert_eq!(h.bi_width, 1920);
        assert_eq!(h.bi_height, 1080); // Bottom-up
        assert_eq!(h.bi_planes, 1);
        assert_eq!(h.bi_bit_count, 32);
        assert_eq!(h.bi_compression, BI_RGB);
        assert_eq!(h.bi_size_image, 1920 * 1080 * 4);
    }

    #[test]
    fn test_bitmap_info_header_invalid_dimensions() {
        assert!(BitmapInfoHeader::new(0, 1080).is_err());
        assert!(BitmapInfoHeader::new(1920, 0).is_err());
        assert!(BitmapInfoHeader::new(40000, 40000).is_err());
    }

    #[test]
    fn test_rgba_to_bottom_up_dib_channels_and_orientation() {
        // 2x2 image:
        // Top row (row 0): Red (255, 0, 0, 255), Green (0, 255, 0, 255)
        // Bottom row (row 1): Blue (0, 0, 255, 255), White (255, 255, 255, 255)
        let rgba = vec![
            255, 0, 0, 255, // (0,0) Red
            0, 255, 0, 255, // (1,0) Green
            0, 0, 255, 255, // (0,1) Blue
            255, 255, 255, 255, // (1,1) White
        ];

        let dib = rgba_to_bottom_up_dib(&rgba, 2, 2).expect("DIB conversion");
        assert_eq!(dib.len(), 40 + 2 * 2 * 4);

        // Header check
        let header = BitmapInfoHeader::from_bytes(&dib).expect("valid header");
        assert_eq!(header.bi_width, 2);
        assert_eq!(header.bi_height, 2);

        // Pixels start at byte 40.
        // Row 0 of DIB must be bottom row of image (row 1: Blue, White):
        let pixels = &dib[40..];

        // Pixel (0,0) in DIB (Blue): B=255, G=0, R=0, A=255
        assert_eq!(&pixels[0..4], &[255, 0, 0, 255]);
        // Pixel (1,0) in DIB (White): B=255, G=255, R=255, A=255
        assert_eq!(&pixels[4..8], &[255, 255, 255, 255]);

        // Row 1 of DIB must be top row of image (row 0: Red, Green):
        // Pixel (0,1) in DIB (Red): B=0, G=0, R=255, A=255
        assert_eq!(&pixels[8..12], &[0, 0, 255, 255]);
        // Pixel (1,1) in DIB (Green): B=0, G=255, R=0, A=255
        assert_eq!(&pixels[12..16], &[0, 255, 0, 255]);
    }

    #[test]
    fn test_rgb_to_bottom_up_dib_channels() {
        let rgb = vec![
            255, 0, 0, // Red
            0, 255, 0, // Green
        ];
        let dib = rgb_to_bottom_up_dib(&rgb, 2, 1).expect("RGB to DIB");
        assert_eq!(dib.len(), 40 + 2 * 1 * 4);
        let pixels = &dib[40..];
        // Red in BGRA: B=0, G=0, R=255, A=255
        assert_eq!(&pixels[0..4], &[0, 0, 255, 255]);
        // Green in BGRA: B=0, G=255, R=0, A=255
        assert_eq!(&pixels[4..8], &[0, 255, 0, 255]);
    }

    #[test]
    fn test_empty_or_truncated_buffer_fails() {
        assert!(rgba_to_bottom_up_dib(&[], 2, 2).is_err());
        assert!(rgba_to_bottom_up_dib(&[255; 15], 2, 2).is_err()); // Needs 16 bytes
        assert!(rgb_to_bottom_up_dib(&[255; 5], 2, 1).is_err()); // Needs 6 bytes
    }

    #[test]
    fn test_large_resolution_dimensions() {
        // 8K UHD: 7680 x 4320
        let h = BitmapInfoHeader::new(7680, 4320).expect("8K UHD header");
        assert_eq!(h.bi_width, 7680);
        assert_eq!(h.bi_height, 4320);
        assert_eq!(h.bi_size_image, 7680 * 4320 * 4);
    }

    #[test]
    fn test_png_to_dib_roundtrip() {
        let width = 4;
        let height = 4;
        let mut png_data = Vec::new();
        {
            let mut encoder = png::Encoder::new(&mut png_data, width, height);
            encoder.set_color(png::ColorType::Rgba);
            encoder.set_depth(png::BitDepth::Eight);
            let mut writer = encoder.write_header().expect("write header");
            let mut test_pixels = Vec::new();
            for y in 0..height {
                for x in 0..width {
                    test_pixels.extend_from_slice(&[x as u8 * 50, y as u8 * 50, 128, 255]);
                }
            }
            writer.write_image_data(&test_pixels).expect("write data");
        }

        let (w, h, dib) = png_to_dib(&png_data).expect("png_to_dib");
        assert_eq!(w, width);
        assert_eq!(h, height);
        assert_eq!(dib.len(), 40 + (width * height * 4) as usize);
    }

    #[test]
    fn test_corrupt_png_returns_err() {
        let corrupt = b"not a png image at all";
        assert!(png_to_dib(corrupt).is_err());
    }

    #[test]
    fn test_rapid_consecutive_screenshots_ordering() {
        let seq1 = next_seq();
        let seq2 = next_seq();
        let seq3 = next_seq();

        assert!(seq1 < seq2);
        assert!(seq2 < seq3);

        // An earlier task with seq1 will see current_seq() >= seq3 and bail out safely
        assert!(seq1 < current_seq());
    }
}
