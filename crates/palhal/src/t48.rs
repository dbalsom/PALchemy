use async_trait::async_trait;
use palcore::{DeviceCapabilities, DeviceInfo, DumpResult, GpioError, PackageType, PinMode, PinState};
pub use teefo_rs::T48;
use teefo_rs::{T48_MAX_PINS, api::T48Error};

use crate::GpioProvider;

fn t48_to_gpio_err(err: T48Error) -> GpioError {
    match err {
        T48Error::PinOutOfRange(p) => GpioError::PinOutOfRange(p as usize),
        T48Error::Usb(e) => GpioError::Io(e),
        T48Error::Protocol(msg) => GpioError::Device(format!("Protocol error: {}", msg)),
        T48Error::Overcurrent => GpioError::Device("Overcurrent protection triggered".into()),
        T48Error::DeviceNotFound => GpioError::Device("Device not found".into()),
        _ => GpioError::Device(err.to_string()),
    }
}

#[async_trait]
impl GpioProvider for T48 {
    fn info(&self) -> DeviceInfo {
        let speed = match self.usb_speed {
            0 => palcore::UsbSpeed::Full12Mbps,
            1 => palcore::UsbSpeed::High480Mbps,
            3 => palcore::UsbSpeed::Super5Gbps,
            _ => palcore::UsbSpeed::Unknown,
        };

        DeviceInfo {
            name: "T48".to_string(),
            num_pins: T48_MAX_PINS,
            capabilities: DeviceCapabilities {
                pullup: true,
                pulldown: false,
                variable_voltage: true,
                high_speed_clock: false,
                custom_logic: false,
            },
            connection_type: palcore::ConnectionType::Usb(speed),
            firmware_version: self.firmware_version.clone(),
            device_code: self.device_code.clone(),
            serial_number: self.serial_number.clone(),
            manufacture_date: self.manufacture_date.clone(),
            supply_voltage: self.usb_supply_voltage,
            additional_info: std::collections::HashMap::new(),
        }
    }

    async fn set_package(&mut self, package: PackageType, num_pins: usize) -> Result<(), GpioError> {
        self.set_package_raw(package, num_pins);
        Ok(())
    }

    async fn check_connection(&mut self) -> Result<(), GpioError> {
        self.probe_connection().await.map_err(t48_to_gpio_err)
    }

    async fn set_power_pins(&mut self, vcc_pins: &[u8], gnd_pins: &[u8], voltage_v: f32) -> Result<(), GpioError> {
        self.reset_pins(gnd_pins, vcc_pins, voltage_v)
            .await
            .map_err(t48_to_gpio_err)
    }

    async fn power_off(&mut self) -> Result<(), GpioError> {
        self.reset_pins(&[], &[], 0.0).await.map_err(t48_to_gpio_err)?;
        self.set_vpp_pins(&[], 0.0).await.map_err(t48_to_gpio_err)?;
        Ok(())
    }

    async fn set_vpp_pins(&mut self, pins: &[u8], voltage_v: f32) -> Result<(), GpioError> {
        T48::set_vpp_pins(self, pins, voltage_v).await.map_err(t48_to_gpio_err)
    }

    async fn set_io_voltage(&mut self, voltage_v: f32) -> Result<(), GpioError> {
        T48::set_io_voltage(self, voltage_v).await.map_err(t48_to_gpio_err)
    }

    async fn set_gpios_config(&mut self, pins: &[(usize, PinMode, PinState)]) -> Result<(), GpioError> {
        if self.is_never_reset() {
            return Err(t48_to_gpio_err(T48Error::State(
                "Device must be reset before setting pins".to_string(),
            )));
        }

        for &(pin, mode, state) in pins {
            let socket_pin = self.map_pin(pin as u8).map_err(t48_to_gpio_err)? as usize;
            let hw_mode = match (mode, state) {
                (PinMode::Input, _) => teefo_rs::T48PinMode::Z,
                (PinMode::Output, PinState::High) => teefo_rs::T48PinMode::DriveHigh,
                (PinMode::Output, PinState::Low) => teefo_rs::T48PinMode::DriveLow,
                (PinMode::Output, PinState::Z) => teefo_rs::T48PinMode::Z,
                _ => teefo_rs::T48PinMode::Z,
            };
            self.set_io_hw_mode(socket_pin - 1, hw_mode);
        }

        if !self.is_hold() {
            self.config_and_read().await.map_err(t48_to_gpio_err)?;
        }
        Ok(())
    }

    async fn read_gpios(&mut self) -> Result<Vec<PinState>, GpioError> {
        let socket_states = self.read_pins().await.map_err(t48_to_gpio_err)?;
        let chip_pins = self.chip_pin_count();
        let mut chip_states = vec![PinState::Z; chip_pins];
        for chip_pin in 1..=chip_pins {
            let socket_pin = self.map_pin(chip_pin as u8).map_err(t48_to_gpio_err)? as usize;
            if socket_pin <= socket_states.len() {
                chip_states[chip_pin - 1] = socket_states[socket_pin - 1];
            }
        }
        Ok(chip_states)
    }

    async fn set_clock_pins(&mut self, _pins: &[usize], _freq_hz: u32) -> Result<(), GpioError> {
        Err(GpioError::UnsupportedCapability(
            "T48 does not natively generate clock signals.".into(),
        ))
    }
}

/// Combinatorial dump using direct T48 pin manipulation for performance.
pub async fn dump_combinatorial_t48(t48: &mut T48, inputs: &[u8], outputs: &[u8]) -> Result<DumpResult, GpioError> {
    let num_inputs = inputs.len() as u32;
    let mut vectors = Vec::with_capacity(1 << num_inputs);

    if num_inputs > 20 {
        return Err(GpioError::Other("Too many input pins for exhaustive dump".into()));
    }

    // Setup pins to Z
    let mut modes_to_set = Vec::new();
    for &pin in inputs {
        modes_to_set.push((pin as usize, PinMode::Input, PinState::Z));
    }
    for &pin in outputs {
        modes_to_set.push((pin as usize, PinMode::Input, PinState::Z));
    }

    // Use GpioProvider trait to set initial config
    <T48 as GpioProvider>::set_gpios_config(t48, &modes_to_set).await?;

    t48.set_hold(true);

    for i in 0..(1 << num_inputs) {
        for (idx, &pin) in inputs.iter().enumerate() {
            let bit = (i >> idx) & 1;
            let socket_pin = t48.map_pin(pin).map_err(t48_to_gpio_err)?;
            let mode = if bit == 1 {
                teefo_rs::T48PinMode::DriveHigh
            } else {
                teefo_rs::T48PinMode::DriveLow
            };
            t48.set_io_hw_mode(socket_pin as usize - 1, mode);
        }

        let res = t48.read_pins().await.map_err(t48_to_gpio_err)?;

        let mut out_state = Vec::with_capacity(outputs.len());
        for &pin in outputs {
            out_state.push(res[pin as usize - 1]);
        }
        vectors.push(out_state);
    }

    t48.set_hold(false);

    Ok(DumpResult {
        input_pins: inputs.to_vec(),
        output_pins: outputs.to_vec(),
        vectors,
    })
}
