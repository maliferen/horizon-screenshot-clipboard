fn screenshot_clipboard_settings(
    game: &mut crate::game::Game<'_>,
    mod_id: &str,
    buttons: &mut Vec<String>,
) {
    use horizon_client_render::screenshot_clipboard_runtime as m;
    if mod_id == m::ID {
        let is_ru = game.settings.language.starts_with("ru");
        let enabled = m::is_enabled(|| game.native_setting(m::ID, "enabled").map(|v| v != "false"));
        buttons.push(m::settings_label(enabled, is_ru));

        let toast_enabled =
            m::is_toast_enabled(|| game.native_setting(m::ID, "toast").map(|v| v != "false"));
        buttons.push(m::toast_settings_label(toast_enabled, is_ru));
    }
}
