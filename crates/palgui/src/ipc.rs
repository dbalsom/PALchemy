use palcore::{
    AppError, AppSettings, BuildInfo, ChipDef, ChipListItem, DeviceInfo, DumpResult, InteractResponse, LogEvent,
    PinCommand, StatusResponse,
};
use std::collections::HashMap;

#[cfg(feature = "csr")]
mod inner {
    use super::*;
    use serde::{Deserialize, Serialize};
    use wasm_bindgen::prelude::*;

    #[wasm_bindgen]
    extern "C" {
        #[wasm_bindgen(js_namespace = ["window", "__TAURI__", "core"], catch)]
        async fn invoke(cmd: &str, args: JsValue) -> Result<JsValue, JsValue>;
    }

    async fn call_ipc<A, R>(cmd: &str, args: A) -> Result<R, AppError>
    where
        A: Serialize,
        R: for<'de> Deserialize<'de>,
    {
        let js_args = serde_wasm_bindgen::to_value(&args).map_err(|error| AppError::Internal {
            message: error.to_string(),
        })?;

        match invoke(cmd, js_args).await {
            Ok(value) => serde_wasm_bindgen::from_value(value).map_err(|error| AppError::Internal {
                message: error.to_string(),
            }),
            Err(error) => {
                if let Ok(app_error) = serde_wasm_bindgen::from_value::<AppError>(error.clone()) {
                    Err(app_error)
                } else {
                    Err(AppError::Internal {
                        message: error.as_string().unwrap_or_else(|| "Unknown IPC error".to_string()),
                    })
                }
            }
        }
    }

    #[derive(Serialize)]
    struct NoArgs;

    pub async fn get_chips() -> Result<Vec<ChipListItem>, AppError> {
        call_ipc("get_chips", NoArgs).await
    }

    pub async fn get_chip(name: String) -> Result<Option<ChipDef>, AppError> {
        #[derive(Serialize)]
        struct Args {
            name: String,
        }

        call_ipc("get_chip", Args { name }).await
    }

    pub async fn get_devices() -> Result<Vec<String>, AppError> {
        call_ipc("get_devices", NoArgs).await
    }

    pub async fn get_status() -> Result<StatusResponse, AppError> {
        call_ipc("get_status", NoArgs).await
    }

    pub async fn connect_device(device_type: String) -> Result<String, AppError> {
        #[derive(Serialize)]
        #[serde(rename_all = "camelCase")]
        struct Args {
            device_type: String,
        }

        call_ipc("connect_device", Args { device_type }).await
    }

    pub async fn disconnect_device() -> Result<(), AppError> {
        call_ipc("disconnect_device", NoArgs).await
    }

    pub async fn get_device_info() -> Result<Option<DeviceInfo>, AppError> {
        call_ipc("get_device_info", NoArgs).await
    }

    pub async fn get_build_info() -> Result<BuildInfo, AppError> {
        call_ipc("get_build_info", NoArgs).await
    }

    pub async fn get_log_history() -> Result<Vec<LogEvent>, AppError> {
        call_ipc("get_log_history", NoArgs).await
    }

    pub async fn get_settings() -> Result<AppSettings, AppError> {
        call_ipc("get_settings", NoArgs).await
    }

    pub async fn update_settings(settings: AppSettings) -> Result<AppSettings, AppError> {
        #[derive(Serialize)]
        struct Args {
            settings: AppSettings,
        }

        call_ipc("update_settings", Args { settings }).await
    }

    pub async fn select_chip(name: String) -> Result<String, AppError> {
        #[derive(Serialize)]
        struct Args {
            name: String,
        }

        call_ipc("select_chip", Args { name }).await
    }

    pub async fn dump_chip(name: String) -> Result<DumpResult, AppError> {
        #[derive(Serialize)]
        struct Args {
            name: String,
        }

        call_ipc("dump_chip", Args { name }).await
    }

    pub async fn interact_chip(name: String, pins: HashMap<u8, PinCommand>) -> Result<InteractResponse, AppError> {
        #[derive(Serialize)]
        struct Args {
            name: String,
            pins: HashMap<u8, PinCommand>,
        }

        call_ipc("interact_chip", Args { name, pins }).await
    }

    pub async fn set_interactive_mode(active: bool) -> Result<(), AppError> {
        #[derive(Serialize)]
        struct Args {
            active: bool,
        }

        call_ipc("set_interactive_mode", Args { active }).await
    }

    pub async fn update_interactive_commands(pins: HashMap<u8, PinCommand>) -> Result<(), AppError> {
        #[derive(Serialize)]
        struct Args {
            pins: HashMap<u8, PinCommand>,
        }

        call_ipc("update_interactive_commands", Args { pins }).await
    }
}

#[cfg(feature = "csr")]
pub use inner::*;

#[cfg(not(feature = "csr"))]
mod stub {
    use super::*;

    fn native_error() -> AppError {
        AppError::Internal {
            message: "IPC is only available in CSR mode".to_string(),
        }
    }

    pub async fn get_chips() -> Result<Vec<ChipListItem>, AppError> {
        Err(native_error())
    }

    pub async fn get_chip(_name: String) -> Result<Option<ChipDef>, AppError> {
        Err(native_error())
    }

    pub async fn get_devices() -> Result<Vec<String>, AppError> {
        Err(native_error())
    }

    pub async fn get_status() -> Result<StatusResponse, AppError> {
        Err(native_error())
    }

    pub async fn connect_device(_device_type: String) -> Result<String, AppError> {
        Err(native_error())
    }

    pub async fn disconnect_device() -> Result<(), AppError> {
        Err(native_error())
    }

    pub async fn get_device_info() -> Result<Option<DeviceInfo>, AppError> {
        Err(native_error())
    }

    pub async fn get_build_info() -> Result<BuildInfo, AppError> {
        Err(native_error())
    }

    pub async fn get_log_history() -> Result<Vec<LogEvent>, AppError> {
        Err(native_error())
    }

    pub async fn get_settings() -> Result<AppSettings, AppError> {
        Err(native_error())
    }

    pub async fn update_settings(_settings: AppSettings) -> Result<AppSettings, AppError> {
        Err(native_error())
    }

    pub async fn select_chip(_name: String) -> Result<String, AppError> {
        Err(native_error())
    }

    pub async fn dump_chip(_name: String) -> Result<DumpResult, AppError> {
        Err(native_error())
    }

    pub async fn interact_chip(_name: String, _pins: HashMap<u8, PinCommand>) -> Result<InteractResponse, AppError> {
        Err(native_error())
    }

    pub async fn set_interactive_mode(_active: bool) -> Result<(), AppError> {
        Err(native_error())
    }

    pub async fn update_interactive_commands(_pins: HashMap<u8, PinCommand>) -> Result<(), AppError> {
        Err(native_error())
    }
}

#[cfg(not(feature = "csr"))]
pub use stub::*;
