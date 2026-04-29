use crate::{
    T48, T48PinMode,
    api::{PinState, T48Error},
};

/// A bit-banging I2C bus implementation for the T48.
pub struct I2cBus<'a> {
    t48: &'a mut T48,
    sda_pin: usize,
    scl_pin: usize,
}

impl<'a> I2cBus<'a> {
    pub fn new(t48: &'a mut T48, sda: usize, scl: usize) -> Self {
        Self {
            t48,
            sda_pin: sda,
            scl_pin: scl,
        }
    }

    async fn set_scl(&mut self, val: bool) -> Result<(), T48Error> {
        let hw_mode = if val {
            T48PinMode::DriveHigh
        } else {
            T48PinMode::DriveLow
        };
        self.t48.set_hw_pin_mode(self.scl_pin, hw_mode).await
    }

    async fn set_sda(&mut self, val: bool) -> Result<(), T48Error> {
        let hw_mode = if val {
            T48PinMode::DriveHigh
        } else {
            T48PinMode::DriveLow
        };
        self.t48.set_hw_pin_mode(self.sda_pin, hw_mode).await
    }

    async fn set_sda_input(&mut self) -> Result<(), T48Error> {
        self.t48.set_hw_pin_mode(self.sda_pin, T48PinMode::Z).await
    }

    /// Set both pins at once to reduce USB transactions
    async fn set_pins(&mut self, scl: bool, sda: bool) -> Result<(), T48Error> {
        let scl_socket = self.t48.map_pin(self.scl_pin as u8)? as usize;
        let sda_socket = self.t48.map_pin(self.sda_pin as u8)? as usize;
        self.t48.io_hw_modes[scl_socket - 1] = if scl {
            T48PinMode::DriveHigh
        } else {
            T48PinMode::DriveLow
        };
        self.t48.io_hw_modes[sda_socket - 1] = if sda {
            T48PinMode::DriveHigh
        } else {
            T48PinMode::DriveLow
        };
        self.t48.config_and_read().await?;
        Ok(())
    }

    pub async fn start(&mut self) -> Result<(), T48Error> {
        self.set_pins(true, true).await?;
        self.set_sda(false).await?;
        Ok(())
    }

    pub async fn stop(&mut self) -> Result<(), T48Error> {
        self.set_pins(true, false).await?;
        self.set_sda(true).await?;
        Ok(())
    }

    pub async fn write_byte(&mut self, data: u8) -> Result<bool, T48Error> {
        for i in (0..8).rev() {
            self.set_scl(false).await?;
            self.set_sda((data >> i) & 1 != 0).await?;
            self.set_scl(true).await?;
        }

        self.set_scl(false).await?;
        self.set_sda_input().await?;
        self.set_scl(true).await?;

        // Read ACK
        let res = self.t48.read_pins().await?;
        let ack_pin = self.t48.map_pin(self.sda_pin as u8)? as usize;
        let ack = res[ack_pin - 1] == PinState::High;

        Ok(ack) // Note: In I2C, ACK is low (false), NACK is high (true)
    }

    pub async fn read_byte(&mut self, ack: bool) -> Result<u8, T48Error> {
        self.set_sda_input().await?;

        let mut data = 0u8;
        for _ in 0..8 {
            self.set_scl(false).await?;
            let res = self.t48.read_pins().await?;
            let sda_socket = self.t48.map_pin(self.sda_pin as u8)? as usize;
            data = (data << 1) | (if res[sda_socket - 1] == PinState::High { 1 } else { 0 });
            self.set_scl(true).await?;
        }

        // Send ACK/NACK
        // ack=true (ACK) -> SDA low, ack=false (NACK) -> SDA high
        self.set_sda(!ack).await?;
        self.set_scl(true).await?;
        self.set_scl(false).await?;

        Ok(data)
    }

    pub async fn write_reg(&mut self, addr: u8, reg: u8, val: u8) -> Result<(), T48Error> {
        self.start().await?;
        self.write_byte(addr << 1).await?;
        self.write_byte(reg).await?;
        self.write_byte(val).await?;
        self.stop().await?;
        Ok(())
    }

    pub async fn read_reg(&mut self, addr: u8, reg: u8) -> Result<u8, T48Error> {
        self.start().await?;
        self.write_byte(addr << 1).await?;
        self.write_byte(reg).await?;
        self.stop().await?;

        self.start().await?;
        self.write_byte((addr << 1) | 1).await?;
        let val = self.read_byte(false).await?; // NACK last byte
        self.stop().await?;

        Ok(val)
    }
}
