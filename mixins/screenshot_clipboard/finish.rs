fn screenshot_clipboard_finish(content: &horizon_client_core::text::Text) {
    use crate::screenshot_clipboard_runtime as m;
    if !m::is_enabled(|| None) {
        return;
    }
    for span in &content.spans {
        if let Some(horizon_client_core::text::ClickEvent::OpenFile(path_str)) = &span.style.click {
            if path_str.to_ascii_lowercase().ends_with(".png") {
                m::handle_screenshot_path(path_str);
                break;
            }
        }
    }
}
