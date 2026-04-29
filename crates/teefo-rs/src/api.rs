use thiserror::Error;

// Re-export shared types from palcore
pub use palcore::{PackageType, PinMode, PinState};

#[derive(Debug, Error)]
pub enum T48Error {
    #[error("T48 device not found")]
    DeviceNotFound,

    #[error("USB error: {0}")]
    Usb(#[from] std::io::Error),

    #[error("NUSB error: {0}")]
    Nusb(#[from] nusb::Error),

    #[error("Protocol error: {0}")]
    Protocol(String),

    #[error("Overcurrent protection triggered")]
    Overcurrent,

    #[error("Pin out of range: {0}")]
    PinOutOfRange(u8),

    #[error("Pin {0} invalid for function {1}")]
    InvalidPinForFunction(u8, String),

    #[error("State error: {0}")]
    State(String),

    #[error("Value out of range")]
    OutOfRange,

    #[error("Voltage out of range: {0}")]
    VoltageOutOfRange(f32),

    #[error("USB timeout")]
    UsbTimeout(&'static str),
}
