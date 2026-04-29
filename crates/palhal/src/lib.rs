pub mod t48;

use async_trait::async_trait;
use palcore::{DeviceCapabilities, DeviceInfo, GpioError, PackageType, PinMode, PinState};

/// Hardware abstraction trait for GPIO-capable programmer devices.
#[async_trait]
pub trait GpioProvider: Send + Sync {
    /// Returns the capabilities of the device
    fn capabilities(&self) -> DeviceCapabilities {
        self.info().capabilities
    }

    /// Queries basic device information
    fn info(&self) -> DeviceInfo;

    /// Verifies that the device connection is still alive.
    async fn check_connection(&mut self) -> Result<(), GpioError> {
        Ok(())
    }

    /// Configures the package type and number of pins for the selected chip.
    /// This is used to map chip pins to physical socket pins.
    async fn set_package(&mut self, package: PackageType, num_pins: usize) -> Result<(), GpioError>;

    /// Sets the power (VCC) and ground (GND) pins.
    /// May not preserve the state of other pins!
    async fn set_power_pins(&mut self, vcc_pins: &[u8], gnd_pins: &[u8], voltage_v: f32) -> Result<(), GpioError>;

    /// Turns off all power to the socket
    async fn power_off(&mut self) -> Result<(), GpioError>;

    /// Sets the programming voltage (VPP) pins and voltage
    async fn set_vpp_pins(&mut self, pins: &[u8], voltage_v: f32) -> Result<(), GpioError>;

    /// Sets the IO voltage (logic threshold reference)
    async fn set_io_voltage(&mut self, voltage_v: f32) -> Result<(), GpioError>;

    /// Sets the mode and state of a group of pins
    async fn set_gpios_config(&mut self, pins: &[(usize, PinMode, PinState)]) -> Result<(), GpioError>;

    /// Reads the current states of all pins
    async fn read_gpios(&mut self) -> Result<Vec<PinState>, GpioError>;

    /// Configures specific pins to output a clock signal at a given frequency in Hz
    async fn set_clock_pins(&mut self, pins: &[usize], freq_hz: u32) -> Result<(), GpioError>;
}

/// Returns a list of available hardware driver names
pub fn available_drivers() -> Vec<&'static str> {
    vec!["T48"]
}

/// Attempts to connect to a specific hardware device by name
pub async fn connect_device(device_type: &str) -> Result<Box<dyn GpioProvider>, GpioError> {
    match device_type.to_lowercase().as_str() {
        "t48" => {
            let t48 = t48::T48::open()
                .await
                .map_err(|e| GpioError::Device(format!("Failed to open T48: {}", e)))?;
            Ok(Box::new(t48))
        }
        _ => Err(GpioError::Device(format!(
            "Unknown or unsupported device type: {}",
            device_type
        ))),
    }
}
