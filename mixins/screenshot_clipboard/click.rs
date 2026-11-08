fn screenshot_clipboard_click(game: &mut crate::game::Game<'_>, mod_id: &str, button: usize) {
    use horizon_client_render::screenshot_clipboard_runtime as m;
    if mod_id == m::ID {
        match button {
            0 => {
                let current =
                    m::is_enabled(|| game.native_setting(m::ID, "enabled").map(|v| v != "false"));
                let next = !current;
                m::set_enabled(next);
                game.native_set_setting(m::ID, "enabled", if next { "true" } else { "false" });
            }
            1 => {
                let current = m::is_toast_enabled(|| {
                    game.native_setting(m::ID, "toast").map(|v| v != "false")
                });
                let next = !current;
                m::set_toast_enabled(next);
                game.native_set_setting(m::ID, "toast", if next { "true" } else { "false" });
            }
            _ => {}
        }
    }
}
