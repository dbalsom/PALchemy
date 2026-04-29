mod commands;
mod logging;
mod menu;
mod settings;
mod state;

pub mod built_info {
    include!(concat!(env!("OUT_DIR"), "/built.rs"));
}

use std::collections::HashMap;

use palcore::ChipDef;
use settings::SettingsStore;
use state::AppState;
use tauri::Manager;
use tokio::sync::broadcast;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

pub fn build_version() -> String {
    if let Some(commit) = built_info::GIT_COMMIT_HASH_SHORT {
        let dirty_suffix = if built_info::GIT_DIRTY.unwrap_or(false) {
            "-dirty"
        } else {
            ""
        };
        format!("{}-{commit}{dirty_suffix}", built_info::PKG_VERSION)
    } else {
        built_info::PKG_VERSION.to_string()
    }
}

fn main() {
    let env_filter = tracing_subscriber::EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| "palapp=debug,palhal=debug,palcore=debug".into());
    let (log_sender, _) = broadcast::channel(512);
    let log_history = logging::LogHistory::new(logging::MAX_LOG_HISTORY);

    tracing_subscriber::registry()
        .with(env_filter)
        .with(tracing_subscriber::fmt::layer())
        .with(logging::TauriLogLayer::new(log_sender.clone(), log_history.clone()))
        .init();

    let chips = load_chips();
    tracing::info!("loaded {} chip definitions", chips.len());

    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_clipboard_manager::init())
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_dialog::init())
        .manage(AppState::new(chips.clone()))
        .manage(log_history)
        .setup(move |app| {
            let settings_store = SettingsStore::load(&app.handle())?;
            let settings = settings_store.clone().get_blocking();
            app.manage(settings_store);
            logging::spawn_log_relay(app.handle().clone(), log_sender.subscribe());

            if let Some(window) = app.handle().get_webview_window("main") {
                let _ = window.set_title(&format!("PALchemy - Build {}", build_version()));
            }

            let menu = menu::create_main_menu(&app.handle(), &chips, settings)?;
            app.set_menu(menu)?;
            Ok(())
        })
        .on_menu_event(|app, event| {
            menu::handle_menu_event(app, event);
        })
        .invoke_handler(tauri::generate_handler![
            commands::get_chips,
            commands::get_chip,
            commands::get_devices,
            commands::get_status,
            commands::connect_device,
            commands::disconnect_device,
            commands::get_device_info,
            commands::select_chip,
            commands::dump_chip,
            commands::interact_chip,
            commands::set_interactive_mode,
            commands::update_interactive_commands,
            commands::get_build_info,
            commands::get_log_history,
            commands::get_settings,
            commands::update_settings
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

fn load_chips() -> HashMap<String, ChipDef> {
    let mut chips = HashMap::new();
    for path in ["chips", "../chips", "../../chips"] {
        if let Ok(definitions) = ChipDef::load_from_dir(path) {
            for chip in definitions {
                chips.insert(chip.name.clone(), chip);
            }
            break;
        }
    }
    chips
}
