//! Pigeodoon: a small client for Odoo Discuss.
//!
//! The Odoo server's own web client does the chatting. This shell adds what a browser tab cannot:
//! a server picker, native notifications that keep working with the window closed, a menu-bar/tray
//! icon, an unread badge and settings in the style of a messaging app.

mod odoo;
mod settings;

use settings::Settings;
use std::sync::Mutex;
#[cfg(desktop)]
use tauri::{
    menu::{Menu, MenuItemBuilder, SubmenuBuilder},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
};
use tauri::{
    webview::{DownloadEvent, PageLoadEvent},
    AppHandle, Manager, State, Url, WebviewUrl, WebviewWindow, WebviewWindowBuilder, WindowEvent,
};
use tauri_plugin_notification::NotificationExt;
use tauri_plugin_opener::OpenerExt;

const INIT_JS: &str = include_str!("inject.js");

struct AppState {
    settings: Mutex<Settings>,
    unread: Mutex<i64>,
    /// True while the user is logging in (the login may bounce through Google, Microsoft, SAML...).
    auth_flow: Mutex<bool>,
}

// ---------------------------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------------------------

fn local_url(page: &str) -> Url {
    let base = if cfg!(any(target_os = "windows", target_os = "android")) { "http://tauri.localhost/" } else { "tauri://localhost/" };
    Url::parse(&format!("{base}{page}")).expect("valid local url")
}

fn discuss_url(settings: &Settings) -> Option<Url> {
    let server = settings.server_url.as_ref()?;
    Url::parse(&format!("{server}{}", settings.discuss_path)).ok()
}

fn same_origin(a: &Url, b: &Url) -> bool {
    a.scheme() == b.scheme() && a.host_str() == b.host_str() && a.port_or_known_default() == b.port_or_known_default()
}

fn current(app: &AppHandle) -> Settings {
    app.state::<AppState>().settings.lock().unwrap().clone()
}

fn main_window(app: &AppHandle) -> Option<WebviewWindow> {
    app.get_webview_window("main")
}

fn show_main(app: &AppHandle) {
    if let Some(win) = main_window(app) {
        let _ = win.show();
        let _ = win.unminimize();
        let _ = win.set_focus();
    }
}

fn go_to_discuss(app: &AppHandle) {
    if let (Some(win), Some(url)) = (main_window(app), discuss_url(&current(app))) {
        let _ = win.navigate(url);
    }
}

fn clear_unread(app: &AppHandle) {
    *app.state::<AppState>().unread.lock().unwrap() = 0;
    if let Some(win) = main_window(app) {
        let _ = win.set_badge_count(None);
    }
}

/// Tells the Odoo page which settings it has to follow itself (today: the sound).
fn push_config(app: &AppHandle) {
    let s = current(app);
    if let Some(win) = main_window(app) {
        let _ = win.eval(format!("window.__pigeodoon_cfg = {{ sound: {} }};", s.play_sound));
    }
}

fn valid_time(value: &str) -> bool {
    chrono::NaiveTime::parse_from_str(value, "%H:%M").is_ok()
}

fn truncate(text: &str, max: usize) -> String {
    text.chars().take(max).collect()
}

// ---------------------------------------------------------------------------------------------
// Commands
// ---------------------------------------------------------------------------------------------

#[tauri::command]
async fn check_server(url: String) -> Result<odoo::ServerInfo, String> {
    odoo::probe(&url).await
}

#[tauri::command]
fn connect(app: AppHandle, state: State<AppState>, url: String, discuss_path: String) -> Result<(), String> {
    let parsed = odoo::normalize(&url)?;
    if !discuss_path.starts_with('/') || discuss_path.starts_with("//") || discuss_path.contains("://") {
        return Err("Invalid Discuss address.".into());
    }
    {
        let mut s = state.settings.lock().unwrap();
        s.server_url = Some(parsed.as_str().trim_end_matches('/').to_string());
        s.discuss_path = discuss_path;
        settings::save(&app, &s)?;
    }
    go_to_discuss(&app);
    Ok(())
}

#[tauri::command]
fn get_settings(state: State<AppState>) -> Settings {
    state.settings.lock().unwrap().clone()
}

#[tauri::command]
fn save_settings(app: AppHandle, state: State<AppState>, new_settings: Settings) -> Result<(), String> {
    if !valid_time(&new_settings.quiet_from) || !valid_time(&new_settings.quiet_to) {
        return Err("Quiet hours must look like 22:00".into());
    }
    let mut s = state.settings.lock().unwrap();
    let previous_autostart = s.start_at_login;
    // The server is never changed from the settings form.
    let merged = Settings { server_url: s.server_url.clone(), discuss_path: s.discuss_path.clone(), ..new_settings };
    settings::save(&app, &merged)?;
    *s = merged.clone();
    drop(s);

    #[cfg(not(any(target_os = "android", target_os = "ios")))]
    if merged.start_at_login != previous_autostart {
        use tauri_plugin_autostart::ManagerExt;
        let manager = app.autolaunch();
        let _ = if merged.start_at_login { manager.enable() } else { manager.disable() };
    }
    #[cfg(any(target_os = "android", target_os = "ios"))]
    let _ = previous_autostart;

    push_config(&app);
    if !merged.show_badge {
        clear_unread(&app);
    }
    Ok(())
}

fn reset_to_connect_screen(app: &AppHandle, forget_server: bool) -> Result<(), String> {
    if forget_server {
        let state = app.state::<AppState>();
        let mut s = state.settings.lock().unwrap();
        s.server_url = None;
        settings::save(app, &s)?;
    }
    clear_unread(app);
    if let Some(win) = main_window(app) {
        let _ = win.clear_all_browsing_data();
        win.navigate(local_url("index.html")).map_err(|e| e.to_string())?;
        let _ = win.show();
        let _ = win.set_focus();
    }
    Ok(())
}

#[tauri::command]
fn disconnect(app: AppHandle) -> Result<(), String> {
    reset_to_connect_screen(&app, true)
}

#[tauri::command]
fn sign_out(app: AppHandle) -> Result<(), String> {
    let s = current(&app);
    let server = s.server_url.ok_or("Not connected")?;
    let target = Url::parse(&format!("{server}/web/session/logout?redirect={}", urlencode(&s.discuss_path))).map_err(|e| e.to_string())?;
    if let Some(win) = main_window(&app) {
        win.navigate(target).map_err(|e| e.to_string())?;
        let _ = win.show();
        let _ = win.set_focus();
    }
    clear_unread(&app);
    Ok(())
}

fn urlencode(text: &str) -> String {
    url::form_urlencoded::byte_serialize(text.as_bytes()).collect()
}

#[tauri::command]
fn open_settings(app: AppHandle) -> Result<(), String> {
    #[cfg(mobile)]
    {
        // A phone has a single window: show the settings page in it (it has a "Back to chat" button).
        let win = main_window(&app).ok_or("No window")?;
        return win.navigate(local_url("settings.html")).map_err(|e| e.to_string());
    }
    #[cfg(desktop)]
    {
        if let Some(win) = app.get_webview_window("settings") {
            let _ = win.show();
            let _ = win.set_focus();
            return Ok(());
        }
        WebviewWindowBuilder::new(&app, "settings", WebviewUrl::App("settings.html".into()))
            .title("Pigeodoon Settings")
            .inner_size(480.0, 640.0)
            .resizable(false)
            .build()
            .map(|_| ())
            .map_err(|e| e.to_string())
    }
}

/// Phones only: leave the settings page and go back to Discuss.
#[tauri::command]
fn back_to_chat(app: AppHandle) {
    go_to_discuss(&app);
}

/// Called by the Odoo page (through the notification shim in `inject.js`).
#[tauri::command]
fn show_notification(app: AppHandle, state: State<AppState>, title: String, body: String) -> Result<(), String> {
    let s = state.settings.lock().unwrap().clone();
    if !s.notifications {
        return Ok(());
    }
    {
        let mut unread = state.unread.lock().unwrap();
        *unread += 1;
        if s.show_badge {
            if let Some(win) = main_window(&app) {
                let _ = win.set_badge_count(Some(*unread));
            }
        }
    }
    let now = chrono::Local::now().format("%H:%M").to_string();
    if settings::in_quiet_hours(&s, &now) {
        return Ok(());
    }
    let title = if title.trim().is_empty() { "Pigeodoon".to_string() } else { truncate(&title, 120) };
    let body = if s.show_preview { truncate(&body, 400) } else { "New message".to_string() };
    app.notification().builder().title(title).body(body).show().map_err(|e| e.to_string())
}

#[tauri::command]
fn clear_badge(app: AppHandle) {
    clear_unread(&app);
}

#[tauri::command]
fn open_external(app: AppHandle, url: String) -> Result<(), String> {
    let parsed = Url::parse(&url).map_err(|e| e.to_string())?;
    if !matches!(parsed.scheme(), "http" | "https" | "mailto") {
        return Err("Only web and mail links can be opened.".into());
    }
    app.opener().open_url(parsed.as_str(), None::<&str>).map_err(|e| e.to_string())
}

// ---------------------------------------------------------------------------------------------
// Window and menus
// ---------------------------------------------------------------------------------------------

/// Decides where the main window may go. Own pages and the Odoo server: yes. Login detours through an
/// identity provider: yes, while logging in. Anything else (links in messages): the system browser.
fn navigation_allowed(app: &AppHandle, url: &Url) -> bool {
    if matches!(url.scheme(), "tauri" | "about" | "data" | "blob") || url.host_str() == Some("tauri.localhost") {
        return true;
    }
    let state = app.state::<AppState>();
    let settings = state.settings.lock().unwrap().clone();
    let Some(server) = settings.server_url.as_deref().and_then(|s| Url::parse(s).ok()) else { return false };

    if same_origin(url, &server) {
        let path = url.path();
        let auth = path.starts_with("/web/login")
            || path.starts_with("/web/signup")
            || path.starts_with("/web/reset_password")
            || path.starts_with("/auth")
            || path.starts_with("/oauth")
            || path.contains("saml");
        *state.auth_flow.lock().unwrap() = auth;
        // After logging in Odoo lands on its apps screen: take the user to Discuss instead.
        let home = matches!(path, "/" | "/odoo" | "/odoo/" | "/web" | "/web/") && url.fragment().is_none() && url.query().is_none();
        if home {
            let handle = app.clone();
            std::thread::spawn(move || go_to_discuss(&handle));
            return false;
        }
        return true;
    }
    if *state.auth_flow.lock().unwrap() {
        return true;
    }
    let _ = app.opener().open_url(url.as_str(), None::<&str>);
    false
}

fn create_main_window(app: &AppHandle) -> tauri::Result<WebviewWindow> {
    let settings = current(app);
    let target = match discuss_url(&settings) {
        Some(url) => WebviewUrl::External(url),
        None => WebviewUrl::App("index.html".into()),
    };
    let nav_app = app.clone();
    let load_app = app.clone();
    let download_app = app.clone();
    WebviewWindowBuilder::new(app, "main", target)
        .title("Pigeodoon")
        .inner_size(1100.0, 760.0)
        .min_inner_size(420.0, 480.0)
        .initialization_script(INIT_JS)
        .disable_drag_drop_handler()
        .on_navigation(move |url| navigation_allowed(&nav_app, url))
        .on_page_load(move |_webview, payload| {
            if matches!(payload.event(), PageLoadEvent::Finished) {
                push_config(&load_app);
            }
        })
        .on_download(move |_webview, event| match event {
            DownloadEvent::Requested { destination, .. } => {
                if let (Ok(dir), Some(name)) = (download_app.path().download_dir(), destination.file_name().map(|n| n.to_owned())) {
                    *destination = unique_path(dir.join(name));
                }
                true
            }
            DownloadEvent::Finished { path, success, .. } => {
                if success {
                    let name = path.as_ref().and_then(|p| p.file_name()).map(|n| n.to_string_lossy().to_string()).unwrap_or_default();
                    let _ = download_app.notification().builder().title("Download finished").body(name).show();
                }
                true
            }
            _ => true,
        })
        .build()
}

/// `report.pdf` -> `report (1).pdf` when the name is taken.
fn unique_path(path: std::path::PathBuf) -> std::path::PathBuf {
    if !path.exists() {
        return path;
    }
    let stem = path.file_stem().map(|s| s.to_string_lossy().to_string()).unwrap_or_default();
    let ext = path.extension().map(|e| format!(".{}", e.to_string_lossy())).unwrap_or_default();
    (1..1000)
        .map(|i| path.with_file_name(format!("{stem} ({i}){ext}")))
        .find(|p| !p.exists())
        .unwrap_or(path)
}

#[cfg(desktop)]
fn build_menu(app: &AppHandle) -> tauri::Result<Menu<tauri::Wry>> {
    let settings_item = MenuItemBuilder::with_id("settings", "Settings…").accelerator("CmdOrCtrl+,").build(app)?;
    let change_server = MenuItemBuilder::with_id("change_server", "Change server…").build(app)?;
    let sign_out = MenuItemBuilder::with_id("sign_out", "Sign out").build(app)?;
    let reload = MenuItemBuilder::with_id("reload", "Reload").accelerator("CmdOrCtrl+R").build(app)?;
    let discuss = MenuItemBuilder::with_id("discuss", "Back to Discuss").accelerator("CmdOrCtrl+D").build(app)?;
    let back = MenuItemBuilder::with_id("back", "Back").accelerator("CmdOrCtrl+[").build(app)?;
    let forward = MenuItemBuilder::with_id("forward", "Forward").accelerator("CmdOrCtrl+]").build(app)?;

    let app_menu = SubmenuBuilder::new(app, "Pigeodoon")
        .about(None)
        .separator()
        .item(&settings_item)
        .item(&change_server)
        .item(&sign_out)
        .separator()
        .quit()
        .build()?;
    let edit_menu = SubmenuBuilder::new(app, "Edit").undo().redo().separator().cut().copy().paste().select_all().build()?;
    let view_menu = SubmenuBuilder::new(app, "View").item(&discuss).item(&reload).separator().item(&back).item(&forward).build()?;
    let window_menu = SubmenuBuilder::new(app, "Window").minimize().close_window().build()?;
    Menu::with_items(app, &[&app_menu, &edit_menu, &view_menu, &window_menu])
}

#[cfg(desktop)]
fn handle_menu(app: &AppHandle, id: &str) {
    match id {
        "settings" => {
            let _ = open_settings(app.clone());
        }
        "change_server" => {
            let _ = reset_to_connect_screen(app, true);
        }
        "sign_out" => {
            let _ = sign_out(app.clone());
        }
        "discuss" => go_to_discuss(app),
        "reload" => {
            if let Some(win) = main_window(app) {
                let _ = win.eval("location.reload()");
            }
        }
        "back" => {
            if let Some(win) = main_window(app) {
                let _ = win.eval("history.back()");
            }
        }
        "forward" => {
            if let Some(win) = main_window(app) {
                let _ = win.eval("history.forward()");
            }
        }
        "open" => show_main(app),
        "quit" => app.exit(0),
        _ => {}
    }
}

#[cfg(desktop)]
fn build_tray(app: &AppHandle) -> tauri::Result<()> {
    let open = MenuItemBuilder::with_id("open", "Open Pigeodoon").build(app)?;
    let settings_item = MenuItemBuilder::with_id("settings", "Settings…").build(app)?;
    let quit = MenuItemBuilder::with_id("quit", "Quit Pigeodoon").build(app)?;
    let menu = Menu::with_items(app, &[&open, &settings_item, &quit])?;
    let mut builder = TrayIconBuilder::with_id("pigeodoon")
        .tooltip("Pigeodoon")
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_menu_event(|app, event| handle_menu(app, event.id().as_ref()))
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click { button: MouseButton::Left, button_state: MouseButtonState::Up, .. } = event {
                show_main(tray.app_handle());
            }
        });
    if let Some(icon) = app.default_window_icon() {
        builder = builder.icon(icon.clone());
    }
    builder.build(app)?;
    Ok(())
}

// ---------------------------------------------------------------------------------------------
// Entry point
// ---------------------------------------------------------------------------------------------

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let builder = tauri::Builder::default();

    #[cfg(not(any(target_os = "android", target_os = "ios")))]
    let builder = builder
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| show_main(app)))
        .plugin(tauri_plugin_autostart::init(tauri_plugin_autostart::MacosLauncher::LaunchAgent, Some(vec!["--minimized"])));

    let app = builder
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![
            check_server,
            connect,
            get_settings,
            save_settings,
            disconnect,
            sign_out,
            open_settings,
            back_to_chat,
            show_notification,
            clear_badge,
            open_external
        ])
        .setup(|app| {
            let handle = app.handle().clone();
            app.manage(AppState { settings: Mutex::new(settings::load(&handle)), unread: Mutex::new(0), auth_flow: Mutex::new(false) });

            #[cfg(not(any(target_os = "android", target_os = "ios")))]
            {
                app.set_menu(build_menu(&handle)?)?;
                app.on_menu_event(|app, event| handle_menu(app, event.id().as_ref()));
                build_tray(&handle)?;
            }

            #[cfg(mobile)]
            {
                let _ = handle.notification().request_permission();
            }

            let window = create_main_window(&handle)?;
            let started_minimized = std::env::args().any(|a| a == "--minimized");
            if started_minimized && current(&handle).server_url.is_some() {
                let _ = window.hide();
            }
            Ok(())
        })
        .on_window_event(|window, event| match event {
            WindowEvent::CloseRequested { api, .. } if window.label() == "main" => {
                let keep = window.app_handle().state::<AppState>().settings.lock().unwrap().keep_running;
                if keep {
                    api.prevent_close();
                    let _ = window.hide();
                }
            }
            WindowEvent::Focused(true) if window.label() == "main" => clear_unread(window.app_handle()),
            _ => {}
        })
        .build(tauri::generate_context!())
        .expect("error while building Pigeodoon");

    app.run(|_app, _event| {
        #[cfg(target_os = "macos")]
        if let tauri::RunEvent::Reopen { .. } = _event {
            show_main(_app);
        }
    });
}
