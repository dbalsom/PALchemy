use std::collections::HashMap;

use palcore::{
    AppSettings, BackendEvent, BackendStatusEvent, ChipDef, LogLevel, LogSeverityEvent, MenuSelectionEvent,
    StatusResponse, ThemePreset,
};
use tauri::{
    menu::{CheckMenuItem, Menu, MenuItem, MenuItemKind, Submenu},
    AppHandle, Emitter, Manager,
};

use crate::{settings::SettingsStore, state::AppState};

const EVENT_MENU_SELECT_DEVICE: &str = "app_menu_select_device";
const EVENT_MENU_SELECT_CHIP: &str = "app_menu_select_chip";
const EVENT_MENU_SELECT_MODE: &str = "app_menu_select_mode";
const EVENT_MENU_DEVICE_INFO: &str = "app_menu_device_info";
const EVENT_MENU_ABOUT: &str = "app_menu_about";
const EVENT_SET_LOG_SEVERITY: &str = "app_set_log_severity";
const EVENT_BACKEND_STATUS: &str = "backend_event";
const MENU_THEME_WONG: &str = "options:theme:wong";
const MENU_THEME_CLASSIC: &str = "options:theme:classic";
const MENU_HIGH_CONTRAST: &str = "options:high_contrast";
const MENU_LARGE_TEXT: &str = "options:large_text";

pub fn create_main_menu(
    app: &AppHandle,
    chips: &HashMap<String, ChipDef>,
    settings: AppSettings,
) -> tauri::Result<Menu<tauri::Wry>> {
    let menu = Menu::new(app)?;

    #[cfg(target_os = "macos")]
    {
        let about_menu = Submenu::with_id(app, "about_menu", mnemonic("About", 'A'), true)?;
        let about_item = MenuItem::with_id(app, "menu:about", mnemonic("About PALchemy", 'A'), true, None::<&str>)?;
        about_menu.append(&about_item)?;
        menu.append(&about_menu)?;
    }

    let device_menu = Submenu::with_id(app, "device_menu", mnemonic("Device", 'D'), true)?;
    let connect_submenu = Submenu::with_id(app, "menu_connect", mnemonic("Connect", 'C'), true)?;
    for driver in palhal::available_drivers() {
        let id = format!("device:{driver}");
        let item = CheckMenuItem::with_id(app, id, driver, true, false, None::<&str>)?;
        connect_submenu.append(&item)?;
    }
    device_menu.append(&connect_submenu)?;

    let disconnect_item = MenuItem::with_id(app, "menu_disconnect", mnemonic("Disconnect", 'i'), false, None::<&str>)?;
    let device_info_item = MenuItem::with_id(
        app,
        "menu_device_info",
        mnemonic("Device Info", 'I'),
        false,
        None::<&str>,
    )?;
    device_menu.append(&disconnect_item)?;
    device_menu.append(&device_info_item)?;
    menu.append(&device_menu)?;

    let chips_submenu = Submenu::with_id(app, "chips_menu", mnemonic("Chips", 'C'), true)?;
    let mut sorted_chips = chips.values().collect::<Vec<_>>();
    sorted_chips.sort_by(|left, right| left.name.cmp(&right.name));
    for chip in sorted_chips {
        let id = format!("chip:{}", chip.name);
        let label = chip_label(chip);
        let item = CheckMenuItem::with_id(app, id, label, true, false, None::<&str>)?;
        chips_submenu.append(&item)?;
    }
    menu.append(&chips_submenu)?;

    let mode_menu = Submenu::with_id(app, "mode_menu", mnemonic("Mode", 'M'), true)?;
    let interact = CheckMenuItem::with_id(
        app,
        "mode:interact",
        mnemonic("Interactive Mode", 'I'),
        true,
        true,
        None::<&str>,
    )?;
    let dump = CheckMenuItem::with_id(
        app,
        "mode:dump",
        mnemonic("Combinatorial Dump", 'D'),
        true,
        false,
        None::<&str>,
    )?;
    let _ = interact.set_checked(true);
    mode_menu.append(&interact)?;
    mode_menu.append(&dump)?;
    menu.append(&mode_menu)?;

    let view_menu = Submenu::with_id(app, "view_menu", mnemonic("View", 'V'), true)?;
    let log_view = MenuItem::with_id(app, "menu:view_log", mnemonic("Log Output", 'L'), true, None::<&str>)?;
    view_menu.append(&log_view)?;
    menu.append(&view_menu)?;

    let options_menu = Submenu::with_id(app, "options_menu", mnemonic("Options", 'O'), true)?;
    let theme_menu = Submenu::with_id(app, "options_theme_menu", mnemonic("Theme", 'T'), true)?;
    let wong_theme = CheckMenuItem::with_id(
        app,
        MENU_THEME_WONG,
        mnemonic("Wong Accessible Palette", 'W'),
        true,
        settings.theme == ThemePreset::Wong,
        None::<&str>,
    )?;
    let classic_theme = CheckMenuItem::with_id(
        app,
        MENU_THEME_CLASSIC,
        mnemonic("Classic Palette", 'C'),
        true,
        settings.theme == ThemePreset::Classic,
        None::<&str>,
    )?;
    let high_contrast = CheckMenuItem::with_id(
        app,
        MENU_HIGH_CONTRAST,
        mnemonic("High Contrast", 'H'),
        true,
        settings.high_contrast,
        None::<&str>,
    )?;
    let large_text = CheckMenuItem::with_id(
        app,
        MENU_LARGE_TEXT,
        mnemonic("Large Text", 'L'),
        true,
        settings.large_text,
        None::<&str>,
    )?;
    theme_menu.append(&wong_theme)?;
    theme_menu.append(&classic_theme)?;
    options_menu.append(&theme_menu)?;
    options_menu.append(&high_contrast)?;
    options_menu.append(&large_text)?;
    menu.append(&options_menu)?;

    #[cfg(not(target_os = "macos"))]
    {
        let help_menu = Submenu::with_id(app, "help_menu", mnemonic("Help", 'H'), true)?;
        let about_item = MenuItem::with_id(app, "menu:about", mnemonic("About PALchemy", 'A'), true, None::<&str>)?;
        help_menu.append(&about_item)?;
        menu.append(&help_menu)?;
    }

    Ok(menu)
}

pub fn handle_menu_event(app: &AppHandle, event: tauri::menu::MenuEvent) {
    let id = event.id().as_ref();

    if let Some(device_name) = id.strip_prefix("device:") {
        enforce_exclusive_check(app, "menu_connect", id);
        emit_main_window(
            app,
            EVENT_MENU_SELECT_DEVICE,
            MenuSelectionEvent {
                value: device_name.to_string(),
            },
        );
        return;
    }

    if let Some(chip_name) = id.strip_prefix("chip:") {
        enforce_exclusive_check(app, "chips_menu", id);
        emit_main_window(
            app,
            EVENT_MENU_SELECT_CHIP,
            MenuSelectionEvent {
                value: chip_name.to_string(),
            },
        );
        return;
    }

    if let Some(mode_name) = id.strip_prefix("mode:") {
        enforce_exclusive_check(app, "mode_menu", id);
        emit_main_window(
            app,
            EVENT_MENU_SELECT_MODE,
            MenuSelectionEvent {
                value: mode_name.to_string(),
            },
        );
        return;
    }

    if let Some(level) = id.strip_prefix("log_severity:") {
        enforce_exclusive_check(app, "severity_menu", id);
        emit_log_windows(
            app,
            EVENT_SET_LOG_SEVERITY,
            LogSeverityEvent {
                level: parse_log_level(level),
            },
        );
        return;
    }

    if id == MENU_THEME_WONG || id == MENU_THEME_CLASSIC || id == MENU_HIGH_CONTRAST || id == MENU_LARGE_TEXT {
        handle_settings_menu_event(app, id);
        return;
    }

    match id {
        "menu_disconnect" => {
            tracing::info!("menu_disconnect selected");
            request_disconnect(app);
        }
        "menu_device_info" => {
            emit_main_window(
                app,
                EVENT_MENU_DEVICE_INFO,
                MenuSelectionEvent {
                    value: "device_info".to_string(),
                },
            );
        }
        "menu:about" => {
            emit_main_window(
                app,
                EVENT_MENU_ABOUT,
                MenuSelectionEvent {
                    value: "about".to_string(),
                },
            );
        }
        "menu:view_log" => {
            open_log_window(app);
        }
        _ => {}
    }
}

pub fn set_device_connected(app: &AppHandle, connected: bool) {
    set_menu_item_enabled(app, "menu_disconnect", connected);
    set_menu_item_enabled(app, "menu_device_info", connected);
    set_menu_item_enabled(app, "menu_connect", !connected);
    if !connected {
        enforce_exclusive_check(app, "menu_connect", "");
    }
}

pub fn sync_status(app: &AppHandle, status: &StatusResponse) {
    set_device_connected(app, status.connection == palcore::ConnectionState::Connected);
    if let Err(error) = app.emit(
        EVENT_BACKEND_STATUS,
        BackendEvent::Status(BackendStatusEvent { status: status.clone() }),
    ) {
        tracing::error!("failed to emit {EVENT_BACKEND_STATUS}: {error}");
    }
}

pub fn sync_settings(app: &AppHandle, settings: AppSettings) {
    enforce_exclusive_check(app, "options_theme_menu", theme_menu_id(settings.theme));
    set_menu_item_checked(app, MENU_HIGH_CONTRAST, settings.high_contrast);
    set_menu_item_checked(app, MENU_LARGE_TEXT, settings.large_text);
}

fn chip_label(chip: &ChipDef) -> String {
    let label = if chip.name == chip.model {
        chip.model.clone()
    } else {
        format!("{} ({})", chip.name, chip.model)
    };

    if let Some(source) = &chip.source {
        format!("{label} - {source}")
    } else {
        label
    }
}

fn mnemonic(label: &str, key: char) -> String {
    #[cfg(target_os = "windows")]
    {
        let lower = key.to_ascii_lowercase();
        if let Some((index, ch)) = label.char_indices().find(|(_, ch)| ch.to_ascii_lowercase() == lower) {
            let mut result = String::with_capacity(label.len() + 1);
            result.push_str(&label[..index]);
            result.push('&');
            result.push(ch);
            result.push_str(&label[index + ch.len_utf8()..]);
            result
        } else {
            label.to_string()
        }
    }

    #[cfg(not(target_os = "windows"))]
    {
        let _ = key;
        label.to_string()
    }
}

fn theme_menu_id(theme: ThemePreset) -> &'static str {
    match theme {
        ThemePreset::Classic => MENU_THEME_CLASSIC,
        ThemePreset::Wong => MENU_THEME_WONG,
    }
}

fn handle_settings_menu_event(app: &AppHandle, id: &str) {
    let app_handle = app.clone();
    let store = app.state::<SettingsStore>().inner().clone();
    let selected_id = id.to_string();
    tauri::async_runtime::spawn(async move {
        let mut settings = store.get().await;
        match selected_id.as_str() {
            MENU_THEME_WONG => settings.theme = ThemePreset::Wong,
            MENU_THEME_CLASSIC => settings.theme = ThemePreset::Classic,
            MENU_HIGH_CONTRAST => settings.high_contrast = !settings.high_contrast,
            MENU_LARGE_TEXT => settings.large_text = !settings.large_text,
            _ => return,
        }

        match store.update(settings).await {
            Ok(saved) => {
                sync_settings(&app_handle, saved);
                if let Err(error) = app_handle.emit(EVENT_BACKEND_STATUS, BackendEvent::SettingsUpdated(saved)) {
                    tracing::error!("failed to emit {EVENT_BACKEND_STATUS}: {error}");
                }
            }
            Err(error) => {
                tracing::error!("failed to update settings from menu: {error}");
                sync_settings(&app_handle, store.get().await);
            }
        }
    });
}

fn open_log_window(app: &AppHandle) {
    if let Some(window) = app.get_webview_window("log_window") {
        let _ = window.set_focus();
        return;
    }

    let menu = create_log_menu(app).ok();
    let mut builder = tauri::WebviewWindowBuilder::new(app, "log_window", tauri::WebviewUrl::App("/log".into()))
        .title("Status Monitor")
        .inner_size(800.0, 600.0);

    if let Some(menu) = menu {
        builder = builder.menu(menu);
    }

    if let Err(error) = builder.build() {
        tracing::error!("failed to create log window: {error}");
    }
}

fn create_log_menu(app: &AppHandle) -> tauri::Result<Menu<tauri::Wry>> {
    let menu = Menu::new(app)?;
    let severity_menu = Submenu::with_id(app, "severity_menu", "Severity", true)?;
    let default_level = default_log_level();
    let trace = CheckMenuItem::with_id(
        app,
        "log_severity:trace",
        "Trace",
        true,
        default_level == LogLevel::Trace,
        None::<&str>,
    )?;
    let debug = CheckMenuItem::with_id(
        app,
        "log_severity:debug",
        "Debug",
        true,
        default_level == LogLevel::Debug,
        None::<&str>,
    )?;
    let info = CheckMenuItem::with_id(
        app,
        "log_severity:info",
        "Info",
        true,
        default_level == LogLevel::Info,
        None::<&str>,
    )?;
    let warn = CheckMenuItem::with_id(app, "log_severity:warn", "Warn", true, false, None::<&str>)?;
    let error = CheckMenuItem::with_id(app, "log_severity:error", "Error", true, false, None::<&str>)?;
    severity_menu.append(&trace)?;
    severity_menu.append(&debug)?;
    severity_menu.append(&info)?;
    severity_menu.append(&warn)?;
    severity_menu.append(&error)?;
    menu.append(&severity_menu)?;
    Ok(menu)
}

fn default_log_level() -> LogLevel {
    if cfg!(debug_assertions) {
        LogLevel::Debug
    } else {
        LogLevel::Info
    }
}

fn parse_log_level(value: &str) -> LogLevel {
    match value {
        "trace" => LogLevel::Trace,
        "debug" => LogLevel::Debug,
        "warn" => LogLevel::Warn,
        "error" => LogLevel::Error,
        _ => LogLevel::Info,
    }
}

fn enforce_exclusive_check(app: &AppHandle, submenu_id: &str, target_id: &str) {
    let menu = if submenu_id == "severity_menu" {
        app.get_webview_window("log_window").and_then(|window| window.menu())
    } else {
        app.menu()
    };

    let Some(menu) = menu else {
        return;
    };

    if let Some(submenu) =
        find_menu_item(menu.items().unwrap_or_default(), submenu_id).and_then(|item| item.as_submenu().cloned())
    {
        if let Ok(items) = submenu.items() {
            for item in items {
                if let Some(check_item) = item.as_check_menuitem() {
                    let _ = check_item.set_checked(check_item.id().as_ref() == target_id);
                }
            }
        }
    }
}

fn set_menu_item_enabled(app: &AppHandle, id: &str, enabled: bool) {
    if let Some(menu) = app.menu() {
        set_item_enabled(menu.items().unwrap_or_default(), id, enabled);
    }

    for (_, window) in app.webview_windows() {
        if let Some(menu) = window.menu() {
            set_item_enabled(menu.items().unwrap_or_default(), id, enabled);
        }
    }
}

fn set_menu_item_checked(app: &AppHandle, id: &str, checked: bool) {
    if let Some(menu) = app.menu() {
        set_item_checked(menu.items().unwrap_or_default(), id, checked);
    }

    for (_, window) in app.webview_windows() {
        if let Some(menu) = window.menu() {
            set_item_checked(menu.items().unwrap_or_default(), id, checked);
        }
    }
}

fn set_item_enabled(items: Vec<MenuItemKind<tauri::Wry>>, id: &str, enabled: bool) -> bool {
    if let Some(item) = find_menu_item(items, id) {
        if let Some(menu_item) = item.as_menuitem() {
            let _ = menu_item.set_enabled(enabled);
        } else if let Some(submenu) = item.as_submenu() {
            let _ = submenu.set_enabled(enabled);
        } else if let Some(check_item) = item.as_check_menuitem() {
            let _ = check_item.set_enabled(enabled);
        } else if let Some(icon_item) = item.as_icon_menuitem() {
            let _ = icon_item.set_enabled(enabled);
        }
        true
    } else {
        false
    }
}

fn set_item_checked(items: Vec<MenuItemKind<tauri::Wry>>, id: &str, checked: bool) -> bool {
    if let Some(item) = find_menu_item(items, id) {
        if let Some(check_item) = item.as_check_menuitem() {
            let _ = check_item.set_checked(checked);
            return true;
        }
    }

    false
}

fn find_menu_item(items: Vec<MenuItemKind<tauri::Wry>>, id: &str) -> Option<MenuItemKind<tauri::Wry>> {
    for item in items {
        if item.id().as_ref() == id {
            return Some(item);
        }

        if let Some(submenu) = item.as_submenu() {
            if let Ok(children) = submenu.items() {
                if let Some(found) = find_menu_item(children, id) {
                    return Some(found);
                }
            }
        }
    }

    None
}

fn emit_main_window<S: serde::Serialize + Clone>(app: &AppHandle, event: &str, payload: S) {
    if let Some(window) = app.get_webview_window("main") {
        if let Err(error) = window.emit(event, payload) {
            tracing::error!("failed to emit {event} to main window: {error}");
        }
    } else {
        tracing::error!("main window not found while emitting {event}");
    }
}

fn emit_log_windows<S: serde::Serialize + Clone>(app: &AppHandle, event: &str, payload: S) {
    let mut emitted = false;
    for (label, window) in app.webview_windows() {
        if label == "log_window" {
            emitted = true;
            if let Err(error) = window.emit(event, payload.clone()) {
                tracing::error!("failed to emit {event} to log window: {error}");
            }
        }
    }

    if !emitted {
        tracing::debug!("no log window open for {event}");
    }
}

fn request_disconnect(app: &AppHandle) {
    let app_handle = app.clone();
    let state = app.state::<AppState>().inner().clone();
    tauri::async_runtime::spawn(async move {
        tracing::info!("dispatching disconnect request from menu");
        match state.disconnect_device().await {
            Ok(()) => {
                let status = state.status().await;
                sync_status(&app_handle, &status);
                tracing::info!("disconnect confirmed by backend");
            }
            Err(error) => {
                tracing::error!("disconnect failed: {error}");
            }
        }
    });
}
