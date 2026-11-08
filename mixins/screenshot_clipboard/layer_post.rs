fn screenshot_clipboard_layer_post(
    game: &mut crate::game::Game<'_>,
    layer: &str,
    batch: &mut crate::hud::HudBatch,
) {
    use horizon_client_render::screenshot_clipboard_runtime as m;

    // Drain and process any completed background clipboard worker events on the main thread
    m::poll_worker_events();

    if layer != "minecraft:chat" {
        return;
    }
    if !m::is_enabled(|| game.native_setting(m::ID, "enabled").map(|v| v != "false")) {
        return;
    }
    if !m::is_toast_enabled(|| game.native_setting(m::ID, "toast").map(|v| v != "false")) {
        return;
    }
    let Some(state) = m::toast_state() else {
        return;
    };

    let is_ru = game.settings.language.starts_with("ru");
    let (line1, line2) = if is_ru {
        ("Скриншот сохранён", "в буфер обмена")
    } else {
        ("Screenshot saved", "to clipboard")
    };

    let text_w1 = game.native_text_width(line1);
    let text_w2 = game.native_text_width(line2);
    let text_w = text_w1.max(text_w2);
    let box_w = (text_w + 16.0).ceil();
    let box_h = 25.0;

    let screen_w = batch.width;
    let screen_h = batch.height;

    // Positioned in the bottom-right corner with 10px margins
    let margin_right = 10.0;
    let margin_bottom = 10.0;
    let x = (screen_w - box_w - margin_right).round();
    let y = (screen_h - box_h - margin_bottom).round();

    let make_argb = |base_a: u32, rgb: u32| -> u32 {
        let scaled_a = ((base_a as f32) * state.alpha).clamp(0.0, 255.0) as u32;
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

    // 5. Thin 1px green progress timer bar along the top edge
    if state.progress > 0.0 {
        let max_bar_w = box_w - 4.0;
        let bar_w = (max_bar_w * state.progress).clamp(1.0, max_bar_w);
        // Vibrant Minecraft Green (0x55FF55)
        game.native_fill(
            batch,
            [x + 2.0, y + 1.0, bar_w, 1.0],
            make_argb(0xFF, 0x55FF55),
        );
    }

    // 6. Left accent pip (2px wide indicator in diamond cyan)
    game.native_fill(
        batch,
        [x + 3.0, y + 3.0, 2.0, box_h - 6.0],
        make_argb(0xFF, 0x38BDF8),
    );

    // 7. Two lines of text in Minecraft font with drop shadow
    let text_x = x + 8.0;
    let text_y1 = y + 4.0;
    let text_y2 = y + 14.0;
    game.native_text(batch, line1, [text_x, text_y1], 0xFFFFFF, true);
    game.native_text(batch, line2, [text_x, text_y2], 0xFFFFFF, true);
}
