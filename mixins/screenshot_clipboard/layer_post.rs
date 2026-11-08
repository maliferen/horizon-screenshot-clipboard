fn screenshot_clipboard_layer_post(
    game: &mut crate::game::Game<'_>,
    layer: &str,
    batch: &mut crate::hud::HudBatch,
) {
    if layer != "minecraft:chat" {
        return;
    }
    use horizon_client_render::screenshot_clipboard_runtime as m;
    if !m::is_enabled(|| game.native_setting(m::ID, "enabled").map(|v| v != "false")) {
        return;
    }
    let Some(alpha) = m::toast_alpha() else {
        return;
    };

    let is_ru = game.settings.language.starts_with("ru");
    let text = if is_ru {
        "Скриншот сохранён в буфер обмена"
    } else {
        "Screenshot saved to clipboard"
    };

    let text_w = game.native_text_width(text);
    let pad_x = 8.0;
    let box_w = (text_w + pad_x * 2.0 + 6.0).ceil();
    let box_h = 16.0;

    let screen_w = batch.width;
    let screen_h = batch.height;

    // Centered horizontally, positioned in the lower area above the hotbar / action bar
    let x = ((screen_w - box_w) / 2.0).round();
    let y = (screen_h - 68.0).round();

    let make_argb = |base_a: u32, rgb: u32| -> u32 {
        let scaled_a = ((base_a as f32) * alpha).clamp(0.0, 255.0) as u32;
        (scaled_a << 24) | (rgb & 0x00FFFFFF)
    };

    // 1. Dark outer border (1px drop shadow / outline in deep charcoal)
    game.native_fill(batch, [x, y, box_w, box_h], make_argb(0xE8, 0x0B111A));

    // 2. Beveled highlight - Top and Left 1px inner line (slate-blue highlight)
    game.native_fill(
        batch,
        [x + 1.0, y + 1.0, box_w - 2.0, 1.0],
        make_argb(0xF0, 0x506A88),
    );
    game.native_fill(
        batch,
        [x + 1.0, y + 1.0, 1.0, box_h - 2.0],
        make_argb(0xF0, 0x506A88),
    );

    // 3. Beveled shadow - Bottom and Right 1px inner line (dark shadow)
    game.native_fill(
        batch,
        [x + 1.0, y + box_h - 2.0, box_w - 2.0, 1.0],
        make_argb(0xF0, 0x141E2B),
    );
    game.native_fill(
        batch,
        [x + box_w - 2.0, y + 1.0, 1.0, box_h - 2.0],
        make_argb(0xF0, 0x141E2B),
    );

    // 4. Main pixelated surface fill (classic deep blue-gray Minecraft panel)
    game.native_fill(
        batch,
        [x + 2.0, y + 2.0, box_w - 4.0, box_h - 4.0],
        make_argb(0xE8, 0x1E293B),
    );

    // 5. Left accent pip (2px wide indicator in diamond cyan)
    game.native_fill(
        batch,
        [x + 3.0, y + 3.0, 2.0, box_h - 6.0],
        make_argb(0xFF, 0x38BDF8),
    );

    // 6. Text in Minecraft font with drop shadow
    let text_x = x + pad_x + 3.0;
    let text_y = y + 4.0;
    game.native_text(batch, text, [text_x, text_y], 0xFFFFFF, true);
}
