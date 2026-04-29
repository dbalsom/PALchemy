pub mod api;
pub mod i2c;
pub mod si5351;

use api::{PackageType, PinState, T48Error};
use nusb::Interface;
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    time::Duration,
};
use tracing::{debug, error, info, trace};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum T48PinMode {
    DriveLow = 0,
    DriveHigh = 1,
    L = 2,
    H = 3,
    Clock = 4,
    Z = 5,
    X = 6,
    Gnd = 7,
    Vcc = 8,
}

pub const T48_USB_VID: u16 = 0xA466;
pub const T48_USB_PID: u16 = 0x0A53;
pub const T48_DEVTYPE: u8 = 7;
pub const T48_MAX_PINS: usize = 56;

const T48_CMD_QUERY: u8 = 0x00;
const T48_CONFIG_AND_READ: u8 = 0x28;
const T48_RESET_PINS: u8 = 0x2D;
const T48_SET_VCC_PINS: u8 = 0x2E;
const T48_SET_VPP_PINS: u8 = 0x2F;
const T48_SET_GND_PINS: u8 = 0x30;

pub const USB_TIMEOUT: Duration = Duration::from_secs(2);

pub struct T48 {
    interface: std::sync::Arc<Interface>,
    device_never_reset: bool,
    io_hw_modes: [T48PinMode; T48_MAX_PINS],
    hold: bool,
    pullup: bool,
    out_endpoints: Vec<u8>,
    in_endpoints: Vec<u8>,
    package: PackageType,
    chip_pins: usize,

    // Extracted Device Info
    pub firmware_version: Option<String>,
    pub device_code: Option<String>,
    pub serial_number: Option<String>,
    pub manufacture_date: Option<String>,
    pub usb_speed: u8,
    pub usb_supply_voltage: Option<f32>,
}

impl T48 {
    pub async fn open() -> Result<Self, T48Error> {
        info!("Listing USB devices...");
        let di = nusb::list_devices()
            .await
            .map_err(T48Error::Nusb)?
            .find(|d| d.vendor_id() == T48_USB_VID && d.product_id() == T48_USB_PID)
            .ok_or(T48Error::DeviceNotFound)?;

        debug!("Found T48 device: {:?}", di);
        info!("Opening T48 device...");
        let device = di.open().await.map_err(T48Error::Nusb)?;
        info!("Claiming interface 0...");
        let interface = std::sync::Arc::new(device.claim_interface(0).await.map_err(T48Error::Nusb)?);

        let mut out_endpoints = Vec::new();
        let mut in_endpoints = Vec::new();

        for desc in interface.descriptors() {
            for ep in desc.endpoints() {
                debug!("Endpoint: {:?}", ep);
                if ep.transfer_type() == nusb::descriptors::TransferType::Bulk {
                    if ep.direction() == nusb::transfer::Direction::Out {
                        out_endpoints.push(ep.address());
                    } else if ep.direction() == nusb::transfer::Direction::In {
                        in_endpoints.push(ep.address());
                    }
                }
            }
        }

        if out_endpoints.is_empty() || in_endpoints.is_empty() {
            return Err(T48Error::Protocol("Missing bulk endpoints".to_string()));
        }

        let mut t48 = Self {
            interface,
            device_never_reset: true,
            io_hw_modes: [T48PinMode::Z; T48_MAX_PINS],
            hold: false,
            pullup: false,
            out_endpoints,
            in_endpoints,
            package: PackageType::DIP,
            chip_pins: 40,
            firmware_version: None,
            device_code: None,
            serial_number: None,
            manufacture_date: None,
            usb_speed: 4, // 4 = unknown
            usb_supply_voltage: None,
        };

        t48.init().await?;
        info!("T48 initialization complete.");

        Ok(t48)
    }

    pub async fn probe_connection(&self) -> Result<(), T48Error> {
        let mut msg = vec![0u8; 80];
        msg[0] = T48_CMD_QUERY;

        let resp = self.transact(Some(&msg[0..5]), 80).await?;
        if resp.len() < 63 {
            return Err(T48Error::Protocol(format!("Query response too short: {}", resp.len())));
        }

        if resp[6] != T48_DEVTYPE {
            return Err(T48Error::Protocol(format!(
                "Device type mismatch: expected {}, got {}",
                T48_DEVTYPE, resp[6]
            )));
        }

        Ok(())
    }

    // Public accessor methods for use by palhal
    // ---------------------------------------------------------------------------

    /// Set the package type and pin count directly.
    pub fn set_package_raw(&mut self, package: PackageType, num_pins: usize) {
        self.package = package;
        self.chip_pins = num_pins;
    }

    /// Returns `true` if the device has never been reset (pins not initialized).
    pub fn is_never_reset(&self) -> bool {
        self.device_never_reset
    }

    /// Returns the current hold state.
    pub fn is_hold(&self) -> bool {
        self.hold
    }

    /// Sets the hold state. When `true`, config_and_read() calls are deferred.
    pub fn set_hold(&mut self, hold: bool) {
        self.hold = hold;
    }

    /// Returns the chip pin count.
    pub fn chip_pin_count(&self) -> usize {
        self.chip_pins
    }

    /// Sets the hardware mode for a specific IO slot index (0-based socket index).
    pub fn set_io_hw_mode(&mut self, socket_index: usize, mode: T48PinMode) {
        self.io_hw_modes[socket_index] = mode;
    }

    // USB transport
    // ---------------------------------------------------------------------------

    async fn transact(&self, out_data: Option<&[u8]>, in_len: usize) -> Result<Vec<u8>, T48Error> {
        if let Some(out) = out_data {
            trace!("Sending output to T48...");
            let ep = self
                .interface
                .endpoint::<nusb::transfer::Bulk, nusb::transfer::Out>(self.out_endpoints[0])
                .map_err(T48Error::Nusb)?;

            let mut writer = ep.writer(4096);

            tokio::time::timeout(USB_TIMEOUT, writer.write_all(out))
                .await
                .map_err(|_| T48Error::UsbTimeout("Bulk OUT write_all timed out"))?
                .map_err(T48Error::Usb)?;

            tokio::time::timeout(USB_TIMEOUT, writer.flush())
                .await
                .map_err(|_| T48Error::UsbTimeout("Bulk OUT flush timed out"))?
                .map_err(T48Error::Usb)?;
        }

        if in_len > 0 {
            trace!("Receiving input from T48...");
            let ep = self
                .interface
                .endpoint::<nusb::transfer::Bulk, nusb::transfer::In>(self.in_endpoints[0])
                .map_err(T48Error::Nusb)?;

            let mut reader = ep.reader(4096);
            let mut buf = vec![0u8; in_len];

            let n = tokio::time::timeout(USB_TIMEOUT, reader.read(&mut buf))
                .await
                .map_err(|_| T48Error::UsbTimeout("Bulk IN read timed out"))?
                .map_err(T48Error::Usb)?;

            buf.truncate(n);
            Ok(buf)
        } else {
            Ok(Vec::new())
        }
    }

    // Device initialization
    // ---------------------------------------------------------------------------

    async fn init(&mut self) -> Result<(), T48Error> {
        debug!("Sending query command to T48...");
        let mut msg = vec![0u8; 80];
        msg[0] = T48_CMD_QUERY;

        let resp = self.transact(Some(&msg[0..5]), 80).await?;
        debug!("T48 query successful.");

        if resp.len() < 63 {
            return Err(T48Error::Protocol(format!("Query response too short: {}", resp.len())));
        }

        if resp[6] != T48_DEVTYPE {
            return Err(T48Error::Protocol(format!(
                "Device type mismatch: expected {}, got {}",
                T48_DEVTYPE, resp[6]
            )));
        }

        self.firmware_version = Some(format!("{}.{:02}", resp[5], resp[4]));

        let mut code_str = String::new();
        for &b in &resp[24..32] {
            if b == 0 {
                break;
            }
            code_str.push(b as char);
        }
        self.device_code = Some(code_str);

        let mut serial_str = String::new();
        for &b in &resp[32..56] {
            if b == 0 {
                break;
            }
            serial_str.push(b as char);
        }
        self.serial_number = Some(serial_str);

        let mut date_str = String::new();
        for &b in &resp[8..24] {
            if b == 0 {
                break;
            }
            date_str.push(b as char);
        }
        self.manufacture_date = Some(date_str);

        self.usb_speed = resp[60];

        let req_voltage = u32::from_le_bytes(resp[56..60].try_into().unwrap());
        self.usb_supply_voltage = Some((req_voltage as f32) * 0xccf7 as f32 / 0x27000 as f32 / 100.0);

        debug!("Device Type: {} (T48)", T48_DEVTYPE);
        debug!("Firmware Version: {:?}", self.firmware_version);
        debug!("Device Code: {:?}", self.device_code);
        debug!("Serial: {:?}", self.serial_number);
        debug!("Manufacture Date: {:?}", self.manufacture_date);
        debug!("USB Speed Code: {}", self.usb_speed);
        debug!("USB Supply Voltage: {:?}", self.usb_supply_voltage);

        Ok(())
    }

    // Pin configuration and reading
    // ---------------------------------------------------------------------------

    pub async fn config_and_read(&mut self) -> Result<Vec<u8>, T48Error> {
        let mut msg = vec![0u8; 32];
        msg[0] = T48_CONFIG_AND_READ;
        msg[1] = if self.pullup { 0x80 } else { 0 };
        msg[2] = 40;
        msg[4] = 1;

        for i in 0..40 {
            let mode = self.io_hw_modes[i] as u8;
            msg[8 + (i >> 1)] |= mode << (if (i & 1) != 0 { 4 } else { 0 });
        }

        let resp = self.transact(Some(&msg), 32).await?;

        if resp.len() < 32 {
            return Err(T48Error::Protocol("config_and_read response too short".to_string()));
        }

        if resp[1] != 0 {
            error!("Overcurrent protection triggered!");
            return Err(T48Error::Overcurrent);
        }

        let mut values = vec![0u8; 40];
        for i in 0..40 {
            values[i] = (resp[8 + (i >> 1)] >> (if (i & 1) != 0 { 4 } else { 0 })) & 0xf;
        }

        self.hold = false;

        Ok(values)
    }

    pub async fn set_hw_pin_mode(&mut self, chip_pin: usize, hw_mode: T48PinMode) -> Result<(), T48Error> {
        if self.device_never_reset {
            return Err(T48Error::State("Device must be reset before setting pins".to_string()));
        }

        let pin = self.map_pin(chip_pin as u8)? as usize;
        self.io_hw_modes[pin - 1] = hw_mode;

        if !self.hold {
            self.config_and_read().await?;
        }

        Ok(())
    }

    pub async fn read_pins(&mut self) -> Result<Vec<PinState>, T48Error> {
        if self.device_never_reset {
            return Err(T48Error::State("Device must be reset before reading pins".to_string()));
        }

        let raw_values = self.config_and_read().await?;
        Ok(raw_values
            .into_iter()
            .map(|v| if (v & 1) != 0 { PinState::High } else { PinState::Low })
            .collect())
    }

    pub fn map_pin(&self, pin: u8) -> Result<u8, T48Error> {
        if pin == 0 || pin as usize > 56 {
            return Err(T48Error::PinOutOfRange(pin));
        }

        match self.package {
            PackageType::DIP => {
                let n = self.chip_pins;
                if n > 40 {
                    return Ok(pin);
                }

                if (pin as usize) <= n / 2 {
                    Ok(pin)
                } else if (pin as usize) <= n {
                    let socket_total = 40;
                    let offset = n - (pin as usize);
                    Ok((socket_total - offset) as u8)
                } else {
                    Ok(pin)
                }
            }
            _ => Ok(pin),
        }
    }

    // Power/voltage pin configuration
    // ---------------------------------------------------------------------------

    async fn set_pins(
        &mut self,
        pins: &[u8],
        voltage: u8,
        pin_info: &[(bool, u8, u8)],
        msg_type: u8,
        _pin_type: &str,
    ) -> Result<(), T48Error> {
        let mut msg = vec![0u8; 48];
        msg[0] = msg_type;

        for &chip_pin in pins {
            let pin = self.map_pin(chip_pin)?;
            info!("Mapping chip pin {} to socket pin {}", chip_pin, pin);

            let info = &pin_info[(pin - 1) as usize];
            if !info.0 {
                return Err(T48Error::InvalidPinForFunction(pin, _pin_type.to_string()));
            }

            msg[8 + info.1 as usize] |= 1 << info.2;
        }

        msg[22] = voltage;
        self.transact(Some(&msg), 0).await?;

        Ok(())
    }

    pub async fn reset_pins(&mut self, gnd_pins: &[u8], vcc_pins: &[u8], vcc_voltage: f32) -> Result<(), T48Error> {
        let mut msg = vec![0u8; 10];
        msg[0] = T48_RESET_PINS;
        self.transact(Some(&msg), 0).await?;

        debug!("Setting GND pins {:?} to 0V", gnd_pins);
        self.set_gnd_pins(gnd_pins).await?;

        debug!("Setting VCC pins {:?} to {}V", vcc_pins, vcc_voltage);
        self.set_vcc_pins(vcc_pins, vcc_voltage).await?;

        for mode in self.io_hw_modes.iter_mut() {
            *mode = T48PinMode::Z;
        }

        self.device_never_reset = false;

        Ok(())
    }

    async fn set_gnd_pins(&mut self, pins: &[u8]) -> Result<(), T48Error> {
        let pin_info: [(bool, u8, u8); 56] = [
            (true, 0, 7),
            (true, 0, 6),
            (true, 0, 5),
            (true, 0, 4),
            (true, 0, 3),
            (true, 0, 2),
            (true, 0, 1),
            (true, 0, 0),
            (true, 1, 7),
            (true, 1, 6),
            (true, 1, 5),
            (true, 1, 4),
            (true, 1, 3),
            (true, 1, 2),
            (true, 1, 1),
            (true, 1, 0),
            (false, 0, 0),
            (false, 2, 7),
            (false, 0, 0),
            (false, 2, 6),
            (false, 0, 0),
            (false, 0, 0),
            (false, 0, 0),
            (false, 0, 0),
            (true, 2, 5),
            (false, 0, 0),
            (true, 2, 4),
            (false, 0, 0),
            (true, 2, 3),
            (true, 2, 2),
            (true, 2, 1),
            (true, 2, 0),
            (true, 3, 7),
            (true, 3, 6),
            (true, 3, 5),
            (true, 3, 4),
            (true, 3, 3),
            (true, 3, 2),
            (true, 3, 1),
            (true, 3, 0),
            (false, 0, 0),
            (false, 0, 0),
            (false, 0, 0),
            (false, 0, 0),
            (false, 0, 0),
            (true, 8, 0),
            (false, 0, 0),
            (true, 8, 1),
            (false, 0, 0),
            (false, 0, 0),
            (false, 0, 0),
            (false, 0, 0),
            (false, 0, 0),
            (false, 0, 0),
            (false, 0, 0),
            (true, 8, 2),
        ];

        self.set_pins(pins, 0, &pin_info, T48_SET_GND_PINS, "GND").await
    }

    async fn set_vcc_pins(&mut self, pins: &[u8], voltage: f32) -> Result<(), T48Error> {
        let pin_info: [(bool, u8, u8); 56] = [
            (true, 0, 0),
            (true, 0, 1),
            (true, 0, 2),
            (true, 0, 3),
            (true, 0, 4),
            (true, 0, 5),
            (true, 0, 6),
            (true, 0, 7),
            (true, 1, 7),
            (true, 1, 6),
            (true, 1, 5),
            (true, 1, 4),
            (true, 1, 3),
            (true, 1, 2),
            (true, 1, 1),
            (true, 1, 0),
            (false, 0, 0),
            (false, 0, 0),
            (false, 0, 0),
            (false, 0, 0),
            (false, 0, 0),
            (false, 0, 0),
            (false, 0, 0),
            (false, 0, 0),
            (true, 2, 0),
            (true, 2, 1),
            (true, 2, 2),
            (true, 2, 3),
            (true, 2, 4),
            (true, 2, 5),
            (true, 2, 6),
            (true, 2, 7),
            (true, 3, 0),
            (true, 3, 1),
            (true, 3, 2),
            (true, 3, 3),
            (true, 3, 4),
            (true, 3, 5),
            (true, 3, 6),
            (true, 3, 7),
            (false, 0, 0),
            (false, 0, 0),
            (false, 0, 0),
            (false, 0, 0),
            (false, 0, 0),
            (false, 0, 0),
            (false, 0, 0),
            (false, 0, 0),
            (false, 0, 0),
            (false, 0, 0),
            (false, 0, 0),
            (false, 0, 0),
            (true, 8, 0),
            (true, 8, 1),
            (false, 0, 0),
            (false, 0, 0),
        ];

        if pins.is_empty() {
            info!("Clearing VCC pin assignments");
            return self.set_pins(pins, 0, &pin_info, T48_SET_VCC_PINS, "VCC").await;
        }

        let vcc_min = 1.8;
        let vcc_max = 6.5;

        let v = ((voltage - vcc_min) / (vcc_max - vcc_min) * 62.0 + 0.5) as i32;

        info!("Setting VCC to {}V (code: {})", voltage, v);
        if v < 1 || v > 63 {
            return Err(T48Error::VoltageOutOfRange(voltage));
        }

        self.set_pins(pins, v as u8, &pin_info, T48_SET_VCC_PINS, "VCC").await
    }

    pub async fn set_vpp_pins(&mut self, pins: &[u8], voltage: f32) -> Result<(), T48Error> {
        let pin_info: [(bool, u8, u8); 56] = [
            (true, 0, 7),
            (true, 0, 6),
            (true, 0, 5),
            (true, 0, 4),
            (false, 0, 0),
            (false, 0, 0),
            (false, 0, 0),
            (false, 0, 0),
            (true, 0, 3),
            (true, 0, 2),
            (false, 0, 0),
            (false, 0, 0),
            (false, 0, 0),
            (false, 0, 0),
            (false, 0, 0),
            (false, 0, 0),
            (false, 0, 0),
            (false, 0, 0),
            (false, 0, 0),
            (false, 0, 0),
            (false, 0, 0),
            (false, 0, 0),
            (false, 0, 0),
            (false, 0, 0),
            (false, 0, 0),
            (false, 0, 0),
            (false, 0, 0),
            (false, 0, 0),
            (false, 0, 0),
            (true, 0, 1),
            (true, 0, 0),
            (true, 1, 0),
            (true, 1, 1),
            (true, 1, 2),
            (false, 0, 0),
            (true, 1, 3),
            (true, 1, 4),
            (true, 1, 5),
            (true, 1, 6),
            (true, 1, 7),
            (true, 4, 0),
            (false, 0, 0),
            (false, 0, 0),
            (false, 0, 0),
            (false, 0, 0),
            (false, 0, 0),
            (false, 0, 0),
            (false, 0, 0),
            (false, 0, 0),
            (false, 0, 0),
            (false, 0, 0),
            (false, 0, 0),
            (false, 0, 0),
            (false, 0, 0),
            (false, 0, 0),
            (false, 0, 0),
        ];

        if pins.is_empty() {
            info!("Clearing VPP pin assignments");
            return self.set_pins(pins, 0, &pin_info, T48_SET_VPP_PINS, "VPP").await;
        }

        let vpp_min = 9.0;
        let vpp_max = 25.0;
        let v = (((voltage - vpp_min) / (vpp_max - vpp_min)) * 63.0 + 0.5) as i32;

        info!("Setting VPP to {}V (code: {})", voltage, v);
        if v < 0 || v > 63 {
            return Err(T48Error::VoltageOutOfRange(voltage));
        }

        self.set_pins(pins, v as u8, &pin_info, T48_SET_VPP_PINS, "VPP").await
    }

    pub async fn set_io_voltage(&mut self, voltage: f32) -> Result<(), T48Error> {
        let v_min = 2.35;
        let v_max = 3.45;
        let v = (((voltage - v_min) / (v_max - v_min)) * 4.0 + 0.5) as i32;

        info!("Setting IO voltage to {}V (code: {})", voltage, v);
        if v < 0 || v > 4 {
            return Err(T48Error::VoltageOutOfRange(voltage));
        }

        let mut msg = vec![0u8; 48];
        msg[0] = T48_SET_VPP_PINS;
        msg[1] = 2;
        msg[8] = v as u8;

        self.transact(Some(&msg), 0).await?;
        Ok(())
    }
}
