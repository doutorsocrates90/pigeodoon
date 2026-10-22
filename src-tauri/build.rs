fn main() {
    // Only the commands listed here can be called from web pages; the capabilities in
    // `capabilities/` decide which window/origin may call which one.
    tauri_build::try_build(tauri_build::Attributes::new().app_manifest(
        tauri_build::AppManifest::new().commands(&[
            "check_server",
            "connect",
            "get_settings",
            "save_settings",
            "disconnect",
            "sign_out",
            "open_settings",
            "back_to_chat",
            "show_notification",
            "clear_badge",
            "open_external",
        ]),
    ))
    .expect("failed to run the Tauri build script");
}
