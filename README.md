# Screenshot Clipboard

[![Minecraft](https://img.shields.io/badge/Minecraft-26.3-brightgreen.svg)](https://playhorizon.dev)
[![Horizon Native](https://img.shields.io/badge/Horizon-Native_Mod-orange.svg)](https://playhorizon.dev/docs/mod-code)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)
[![Build Status](https://img.shields.io/badge/build-passing-brightgreen.svg)](#)

A high-performance native Horizon mod for **Minecraft 26.3** that automatically copies every screenshot taken in-game directly into your Windows system clipboard, allowing instant `Ctrl+V` pasting into Discord, Telegram, web browsers, and image editors.

---

## Features

- **Instant System Clipboard Integration:**
  - Pressing `F2` (or your configured screenshot key) saves the image to disk as usual and immediately copies the pixel data into the Windows clipboard.
  - Paste directly (`Ctrl+V`) into Discord, Telegram, Slack, MS Paint, Photoshop, browsers, or documents without opening the screenshots folder.
- **In-Game Pixelated Minecraft Toast Banner:**
  - Displays a clean pixelated blue/gray banner with text (*"Скриншот сохранён в буфер обмена"* / *"Screenshot saved to clipboard"*) positioned neatly in the **bottom-right corner** of the screen.
  - Features an animated **thin green progress timer bar** (`#55FF55`) along the top edge that fills from 0% to 100% over 2.5 seconds, followed by a smooth 0.5-second fade-out.
  - Styled with classic 3D Minecraft bevel highlights, dark slate border, and diamond-cyan accent pip.
  - Automatic language detection (Russian / English fallback) based on in-game language settings.
- **Dual Native Clipboard Formats:**
  - **`CF_DIB` (Device Independent Bitmap):** Universal compatibility with native Windows applications (MS Paint, Office suite, classic image tools).
  - **`"PNG"` Registered Format:** Preferred by modern messengers and browsers (Discord, Telegram, Chrome/Firefox), preserving original quality.
- **Zero GPU Overhead & Asynchronous Processing:**
  - Reuses the game's existing framebuffer readback with zero additional GPU passes.
  - Image decoding and clipboard operations run asynchronously on a dedicated worker thread (`sc-clipboard`), ensuring zero frame drops or hitching even at high resolutions (4K/8K).
- **Monotonic Sequence Ordering:**
  - Rapid successive presses of `F2` are sequenced monotonically. A slower previous capture will never overwrite a more recent screenshot in the clipboard.
- **Full Vanilla Parity:**
  - Screenshots continue to be saved in `.minecraft/screenshots/` with standard naming conventions.
  - In-game chat notifications and clickable file links remain fully functional.
- **In-Game Configuration (Options → Mods → Screenshot Clipboard):**
  - **Button 0 (Master Toggle):** `Копирование скриншота: ВКЛ / ВЫКЛ` (`Screenshot Clipboard: ON / OFF`)
  - **Button 1 (Toast Toggle):** `Уведомление в углу: ВКЛ / ВЫКЛ` (`Corner Notification: ON / OFF`)
  - Choices are automatically persisted across game restarts.
- **Clean Fallback:**
  - Disabling the toggles or removing the `.hnmod` immediately restores pure vanilla behavior with zero residual effects.

---

## Technical Hook Architecture

Screenshot Clipboard integrates into Horizon via native mixins without modifying engine binaries:

| Injection Target | Type | Purpose |
|------------------|------|---------|
| `horizon_client_render::chat::Chat::add` | `HEAD` | Intercepts screenshot completion text (`ClickEvent::OpenFile`), extracts file path, and dispatches asynchronous clipboard copy. |
| `horizon_client::game::native_hooks::Game<'_>::native_gui_layer_post` | `HEAD` | Renders the Minecraft-style pixelated blue/gray bottom toast notification when active. |
| `horizon_client::game::native_hooks::Game<'_>::native_mods` | `HEAD` | Registers `screenshot_clipboard` (v0.1.0) into Horizon's active mod list. |
| `horizon_client::game::native_hooks::Game<'_>::native_settings` | `HEAD` | Renders the `Screenshot Clipboard: ON / OFF` button in Options → Mods. |
| `horizon_client::game::native_hooks::Game<'_>::native_settings_click` | `HEAD` | Toggles mod state on click and persists user preference. |

---

## Requirements & Compatibility

- **Minecraft Version:** `26.3`
- **Engine:** Horizon Nightly (`2ebf00a0281c592f79191e76fce0e7d522c6f425` or compatible)
- **Mixins Compiler:** `horizon-mixins` (`0d6b103c70633d8fd9443d8d0acd3aadaacedb06`)
- **Native Hooks Version:** `v1` (NATIVE_HOOKS_API >= 1)
- **OS:** Windows (uses Win32 Clipboard API: `user32.dll` and `kernel32.dll`)

---

## Installation

1. Download `screenshot_clipboard-0.1.0.hnmod` from the [Releases](https://github.com/maliferen/horizon-screenshot-clipboard/releases) tab.
2. Place the `.hnmod` file into your Horizon game instance directory:
   ```
   %APPDATA%\horizon\instances\<instance_name>\horizon-native-mods\
   ```
3. Launch the game through Horizon Launcher.
4. Verify the mod is active under **Options → Mods → Screenshot Clipboard**.

---

## Building from Source

### Prerequisites
- Rust stable toolchain (edition 2024 / rustc 1.95+)
- Python 3.11+

### Running Unit Tests
```bash
cargo test
cargo fmt -- --check
cargo clippy -- -D warnings
```

### Packaging `.hnmod`
```bash
python package.py --out builds/
```

### Official Horizon Compatibility Check
```bash
python native/tools/native.py check screenshot-clipboard
```

---

## License

This project is licensed under the [MIT License](LICENSE) — see the LICENSE file for details.
All custom mod code is independently developed by `maliferen`.

