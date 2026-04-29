use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::{GpioError, PinState};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub enum LogLevel {
    Trace,
    Debug,
    Info,
    Warn,
    Error,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum LogSource {
    Backend,
    Frontend,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogEvent {
    pub level: LogLevel,
    pub source: LogSource,
    pub target: String,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChipListItem {
    pub name: String,
    pub model: String,
    pub alias: Option<String>,
    pub source: Option<String>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum SelectedMode {
    Interact,
    Dump,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum ConnectionState {
    Disconnected,
    Connected,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum InteractiveStatus {
    Stopped,
    Running,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum PinDirection {
    Input,
    Output,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StatusResponse {
    pub connection: ConnectionState,
    pub interactive: InteractiveStatus,
    pub selected_chip: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackendStatusEvent {
    pub status: StatusResponse,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectionFailureEvent {
    pub title: String,
    pub message: String,
    pub device_type: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum BackendEvent {
    Status(BackendStatusEvent),
    ConnectionFailure(ConnectionFailureEvent),
    OperationFailure(ConnectionFailureEvent),
    SettingsUpdated(AppSettings),
    Log(LogEvent),
    PinStateUpdate(PinStateUpdateEvent),
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ThemePreset {
    Classic,
    Wong,
}

impl ThemePreset {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Classic => "classic",
            Self::Wong => "wong",
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub struct AppSettings {
    pub theme: ThemePreset,
    pub high_contrast: bool,
    pub large_text: bool,
    pub interactive_poll_hz: u16,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            theme: ThemePreset::Wong,
            high_contrast: false,
            large_text: false,
            interactive_poll_hz: 30,
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum PinCommand {
    DriveHigh,
    DriveLow,
    Read,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InteractRequest {
    pub chip_name: String,
    pub pins: HashMap<u8, PinCommand>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InteractResponse {
    pub outputs: HashMap<u8, PinState>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PinStateUpdateEvent {
    pub outputs: HashMap<u8, PinState>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MenuSelectionEvent {
    pub value: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogSeverityEvent {
    pub level: LogLevel,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BuildInfo {
    pub pkg_version: String,
    pub pkg_name: String,
    pub pkg_description: String,
    pub pkg_authors: String,
    pub pkg_license: String,
    pub pkg_repository: String,
    pub target: String,
    pub host: String,
    pub profile: String,
    pub rustc_version: String,
    pub features: Vec<String>,
    pub git_version: Option<String>,
    pub git_dirty: Option<bool>,
    pub git_commit_hash: Option<String>,
    pub ci_platform: Option<String>,
    pub available_drivers: Vec<String>,
    pub loaded_chip_definitions: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, Error)]
pub enum AppError {
    #[error("Device is not connected")]
    DeviceNotConnected,
    #[error("Chip '{name}' is not available")]
    UnknownChip { name: String },
    #[error("Unsupported device '{device_type}'")]
    UnsupportedDevice { device_type: String },
    #[error("Interactive mode requires a selected chip")]
    InteractiveChipNotSelected,
    #[error("Interactive mode is already running")]
    InteractiveAlreadyRunning,
    #[error("Interactive mode is not running")]
    InteractiveNotRunning,
    #[error("{feature} is not implemented")]
    NotImplemented { feature: String },
    #[error("{message}")]
    Hardware { message: String },
    #[error("{message}")]
    InvalidState { message: String },
    #[error("{message}")]
    Internal { message: String },
}

impl From<GpioError> for AppError {
    fn from(value: GpioError) -> Self {
        match value {
            GpioError::Device(message) | GpioError::UnsupportedCapability(message) | GpioError::Other(message) => {
                Self::Hardware { message }
            }
            GpioError::PinOutOfRange(pin) => Self::Hardware {
                message: format!("Pin out of range: {pin}"),
            },
            GpioError::Io(error) => Self::Hardware {
                message: error.to_string(),
            },
        }
    }
}
