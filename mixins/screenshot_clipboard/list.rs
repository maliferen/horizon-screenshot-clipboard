fn screenshot_clipboard_list(mods: &mut Vec<crate::game::native_hooks::NativeMod>) {
    use horizon_client_render::screenshot_clipboard_runtime as m;
    mods.push(crate::game::native_hooks::NativeMod {
        id: m::ID.into(),
        title: format!("{} {}", m::MOD_NAME, m::MOD_VERSION),
        description: "Automatically copies F2 screenshots to Windows clipboard.".into(),
        keys: Vec::new(),
    });
}
