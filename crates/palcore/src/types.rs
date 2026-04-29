use std::fmt::Display;
use thiserror::Error;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct DumpResult {
    pub input_pins: Vec<u8>,
    pub output_pins: Vec<u8>,
    pub vectors: Vec<Vec<PinState>>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum PackageType {
    DIP,
    SOP,
    QFP,
    PLCC,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum PinMode {
    Input,
    Output,
    Vcc,
    Vpp,
    Gnd,
    NC,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum PinState {
    Low = 0,
    High = 1,
    Z,
}

impl Display for PinState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PinState::Low => write!(f, "Low"),
            PinState::High => write!(f, "High"),
            PinState::Z => write!(f, "Z"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, serde::Serialize, serde::Deserialize)]
pub struct DeviceCapabilities {
    pub pullup: bool,
    pub pulldown: bool,
    pub variable_voltage: bool,
    pub high_speed_clock: bool,
    pub custom_logic: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum UsbSpeed {
    Low,
    Full12Mbps,
    High480Mbps,
    Super5Gbps,
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum ConnectionType {
    Usb(UsbSpeed),
    SerialUart,
    UsbCdc(UsbSpeed),
    Ethernet,
    Wifi,
    Other(String),
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct DeviceInfo {
    pub name: String,
    pub num_pins: usize,
    pub capabilities: DeviceCapabilities,
    pub connection_type: ConnectionType,
    pub firmware_version: Option<String>,
    pub device_code: Option<String>,
    pub serial_number: Option<String>,
    pub manufacture_date: Option<String>,
    pub supply_voltage: Option<f32>,
    pub additional_info: std::collections::HashMap<String, String>,
}

#[derive(Debug, Error)]
pub enum GpioError {
    #[error("Device error: {0}")]
    Device(String),
    #[error("Pin out of range: {0}")]
    PinOutOfRange(usize),
    #[error("Capability not supported: {0}")]
    UnsupportedCapability(String),
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Other error: {0}")]
    Other(String),
}
