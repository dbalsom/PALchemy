use std::collections::HashMap;

use palcore::{
    AppError, AppSettings, BackendEvent, BuildInfo, ChipDef, ChipListItem, ConnectionFailureEvent, ConnectionState,
    DeviceInfo, DumpResult, InteractResponse, LogEvent, PinCommand, StatusResponse,
};
use tauri::{AppHandle, Emitter, State};

use crate::{built_info, logging::LogHistory, menu, settings::SettingsStore, state::AppState};

#[tauri::command]
pub async fn get_chips(state: State<'_, AppState>) -> Result<Vec<ChipListItem>, AppError> {
    Ok(state.chip_list())
}

#[tauri::command]
pub async fn get_chip(name: String, state: State<'_, AppState>) -> Result<Option<ChipDef>, AppError> {
    Ok(state.chip(&name))
}

#[tauri::command]
pub async fn get_devices() -> Result<Vec<String>, AppError> {
    Ok(palhal::available_drivers().into_iter().map(str::to_string).collect())
}

#[tauri::command]
pub async fn get_status(state: State<'_, AppState>, app: AppHandle) -> Result<StatusResponse, AppError> {
    let status = state.status().await;
    menu::set_device_connected(&app, status.connection == palcore::ConnectionState::Connected);
    Ok(status)
}

#[tauri::command]
pub async fn connect_device(
    device_type: String,
    state: State<'_, AppState>,
    app: AppHandle,
) -> Result<String, AppError> {
    tracing::info!("attempting device connection: {device_type}");

    match state.connect_device(device_type.clone()).await {
        Ok(message) => {
            tracing::info!("device connected successfully: {device_type}");
            let status = state.status().await;
            menu::sync_status(&app, &status);
            Ok(message)
        }
        Err(error) => {
            tracing::error!("device connection failed for {device_type}: {error}");
            let failure = BackendEvent::ConnectionFailure(ConnectionFailureEvent {
                title: "Device connection failed".to_string(),
                message: error.to_string(),
                device_type: Some(device_type),
            });
            let _ = app.emit("backend_event", failure);
            menu::sync_status(
                &app,
                &StatusResponse {
                    connection: ConnectionState::Disconnected,
                    interactive: palcore::InteractiveStatus::Stopped,
                    selected_chip: None,
                },
            );
            Err(error)
        }
    }
}

#[tauri::command]
pub async fn disconnect_device(state: State<'_, AppState>, app: AppHandle) -> Result<(), AppError> {
    state.disconnect_device().await?;
    let status = state.status().await;
    menu::sync_status(&app, &status);
    Ok(())
}

#[tauri::command]
pub async fn get_build_info(state: State<'_, AppState>) -> Result<BuildInfo, AppError> {
    Ok(BuildInfo {
        pkg_version: built_info::PKG_VERSION.to_string(),
        pkg_name: built_info::PKG_NAME.to_string(),
        pkg_description: built_info::PKG_DESCRIPTION.to_string(),
        pkg_authors: built_info::PKG_AUTHORS.to_string(),
        pkg_license: built_info::PKG_LICENSE.to_string(),
        pkg_repository: built_info::PKG_REPOSITORY.to_string(),
        target: built_info::TARGET.to_string(),
        host: built_info::HOST.to_string(),
        profile: built_info::PROFILE.to_string(),
        rustc_version: built_info::RUSTC_VERSION.to_string(),
        features: built_info::FEATURES.iter().map(|feature| feature.to_string()).collect(),
        git_version: built_info::GIT_VERSION.map(str::to_string),
        git_dirty: built_info::GIT_DIRTY,
        git_commit_hash: built_info::GIT_COMMIT_HASH.map(str::to_string),
        ci_platform: built_info::CI_PLATFORM.map(str::to_string),
        available_drivers: palhal::available_drivers().into_iter().map(str::to_string).collect(),
        loaded_chip_definitions: state.chip_list().len(),
    })
}

#[tauri::command]
pub async fn get_log_history(history: State<'_, LogHistory>) -> Result<Vec<LogEvent>, AppError> {
    Ok(history.snapshot().await)
}

#[tauri::command]
pub async fn get_settings(settings: State<'_, SettingsStore>) -> Result<AppSettings, AppError> {
    Ok(settings.get().await)
}

#[tauri::command]
pub async fn update_settings(
    settings: AppSettings,
    store: State<'_, SettingsStore>,
    app: AppHandle,
) -> Result<AppSettings, AppError> {
    let settings = store.update(settings).await?;
    menu::sync_settings(&app, settings);
    let _ = app.emit("backend_event", BackendEvent::SettingsUpdated(settings));
    Ok(settings)
}

#[tauri::command]
pub async fn get_device_info(state: State<'_, AppState>) -> Result<Option<DeviceInfo>, AppError> {
    Ok(state.device_info().await)
}

#[tauri::command]
pub async fn select_chip(name: String, state: State<'_, AppState>) -> Result<String, AppError> {
    state.select_chip(name).await
}

#[tauri::command]
pub async fn dump_chip(_name: String) -> Result<DumpResult, AppError> {
    Err(AppError::NotImplemented {
        feature: "combinatorial dump".to_string(),
    })
}

#[tauri::command]
pub async fn interact_chip(
    name: String,
    pins: HashMap<u8, PinCommand>,
    state: State<'_, AppState>,
) -> Result<InteractResponse, AppError> {
    state.interact_chip(name, pins).await
}

#[tauri::command]
pub async fn set_interactive_mode(
    active: bool,
    app: AppHandle,
    state: State<'_, AppState>,
    settings: State<'_, SettingsStore>,
) -> Result<(), AppError> {
    if active {
        let poll_hz = settings.get().await.interactive_poll_hz;
        state.start_interactive(app.clone(), poll_hz).await?;
    } else {
        state.stop_interactive().await?;
    }
    let status = state.status().await;
    menu::sync_status(&app, &status);
    Ok(())
}

#[tauri::command]
pub async fn update_interactive_commands(
    pins: HashMap<u8, PinCommand>,
    state: State<'_, AppState>,
) -> Result<(), AppError> {
    state.set_interactive_commands(pins).await
}
