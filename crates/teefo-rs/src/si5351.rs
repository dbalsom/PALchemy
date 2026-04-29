use crate::api::T48Error;
use crate::i2c::I2cBus;

pub const SI5351_BUS_BASE_ADDR: u8 = 0x60;
pub const SI5351_XTAL_FREQ: u64 = 25_000_000;
pub const SI5351_FREQ_MULT: u64 = 100;

pub const SI5351_PLL_VCO_MIN: u64 = 600_000_000;
pub const SI5351_PLL_VCO_MAX: u64 = 900_000_000;
pub const SI5351_MULTISYNTH_DIVBY4_FREQ: u64 = 150_000_000;
pub const SI5351_MULTISYNTH_MAX_FREQ: u64 = 225_000_000;
pub const SI5351_MULTISYNTH_MIN_FREQ: u64 = 500_000;
pub const SI5351_CLKOUT_MIN_FREQ: u64 = 4_000;

// Register Definitions
pub const SI5351_DEVICE_STATUS: u8 = 0;
pub const SI5351_OUTPUT_ENABLE_CTRL: u8 = 3;
pub const SI5351_PLLA_PARAMETERS: u8 = 26;
pub const SI5351_PLLB_PARAMETERS: u8 = 34;
pub const SI5351_CLK0_PARAMETERS: u8 = 42;
pub const SI5351_CLK1_PARAMETERS: u8 = 50;
pub const SI5351_CLK2_PARAMETERS: u8 = 58;
pub const SI5351_CLK0_CTRL: u8 = 16;
pub const SI5351_PLL_RESET: u8 = 177;
pub const SI5351_PLL_RESET_A: u8 = 1 << 5;
pub const SI5351_PLL_RESET_B: u8 = 1 << 7;
pub const SI5351_CRYSTAL_LOAD: u8 = 183;
pub const SI5351_CRYSTAL_LOAD_10PF: u8 = 3 << 6;

pub const SI5351_CLK_POWERDOWN: u8 = 1 << 7;
pub const SI5351_CLK_INTEGER_MODE: u8 = 1 << 6;
pub const SI5351_CLK_PLL_SELECT: u8 = 1 << 5;
pub const SI5351_OUTPUT_CLK_DIVBY4: u8 = 3 << 2;
pub const SI5351_OUTPUT_CLK_DIV_SHIFT: u8 = 4;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Si5351Clock {
    Clk0 = 0,
    Clk1 = 1,
    Clk2 = 2,
    Clk3 = 3,
    Clk4 = 4,
    Clk5 = 5,
    Clk6 = 6,
    Clk7 = 7,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Si5351Pll {
    Plla,
    Pllb,
}

#[derive(Default, Debug)]
pub struct Si5351RegSet {
    pub p1: u32,
    pub p2: u32,
    pub p3: u32,
}

pub struct Si5351 {
    addr: u8,
    plla_freq: u64,
    pllb_freq: u64,
}

impl Si5351 {
    pub fn new(addr: u8) -> Self {
        Self {
            addr,
            plla_freq: 0,
            pllb_freq: 0,
        }
    }

    pub async fn init(&mut self, bus: &mut I2cBus<'_>) -> Result<bool, T48Error> {
        bus.start().await?;
        let nack = bus.write_byte(self.addr << 1).await?;
        bus.stop().await?;

        if nack {
            return Ok(false);
        }

        loop {
            let status = bus.read_reg(self.addr, SI5351_DEVICE_STATUS).await?;
            if (status & (1 << 7)) == 0 {
                break;
            }
        }

        bus.write_reg(self.addr, SI5351_CRYSTAL_LOAD, SI5351_CRYSTAL_LOAD_10PF | 0b00010010)
            .await?;

        Ok(true)
    }

    pub async fn set_freq(&mut self, bus: &mut I2cBus<'_>, freq_hz: u64, clk: Si5351Clock) -> Result<(), T48Error> {
        let mut freq = freq_hz * SI5351_FREQ_MULT;

        // Simplified: Always use PLLA for CLK0-2 for now
        let pll = Si5351Pll::Plla;
        let pll_freq = 900_000_000 * SI5351_FREQ_MULT; // 900 MHz

        self.set_pll(bus, pll_freq, pll).await?;

        // Select R divider
        let r_div = self.select_r_div(&mut freq);

        // Calculate multisynth
        let mut ms_reg = Si5351RegSet::default();
        self.multisynth_calc(freq, pll_freq, &mut ms_reg);

        let mut int_mode = 0;
        let mut div_by_4 = 0;
        if freq >= SI5351_MULTISYNTH_DIVBY4_FREQ * SI5351_FREQ_MULT {
            div_by_4 = 1;
            int_mode = 1;
        }

        self.set_ms(bus, clk, ms_reg, r_div, div_by_4).await?;
        self.reset_pll(bus, pll).await?;

        // Enable output
        let mut ctrl = bus.read_reg(self.addr, SI5351_CLK0_CTRL + clk as u8).await?;
        ctrl &= !SI5351_CLK_POWERDOWN;
        if int_mode != 0 {
            ctrl |= SI5351_CLK_INTEGER_MODE;
        } else {
            ctrl &= !SI5351_CLK_INTEGER_MODE;
        }
        bus.write_reg(self.addr, SI5351_CLK0_CTRL + clk as u8, ctrl).await?;

        let mut oe = bus.read_reg(self.addr, SI5351_OUTPUT_ENABLE_CTRL).await?;
        oe &= !(1 << clk as u8);
        bus.write_reg(self.addr, SI5351_OUTPUT_ENABLE_CTRL, oe).await?;

        Ok(())
    }

    pub async fn set_pll(&mut self, bus: &mut I2cBus<'_>, pll_freq: u64, pll: Si5351Pll) -> Result<(), T48Error> {
        let mut pll_reg = Si5351RegSet::default();
        self.pll_calc(pll_freq, &mut pll_reg);

        let reg = match pll {
            Si5351Pll::Plla => SI5351_PLLA_PARAMETERS,
            Si5351Pll::Pllb => SI5351_PLLB_PARAMETERS,
        };

        let mut params = [0u8; 8];
        params[0] = ((pll_reg.p3 >> 8) & 0xFF) as u8;
        params[1] = (pll_reg.p3 & 0xFF) as u8;
        params[2] = ((pll_reg.p1 >> 16) & 0x03) as u8;
        params[3] = ((pll_reg.p1 >> 8) & 0xFF) as u8;
        params[4] = (pll_reg.p1 & 0xFF) as u8;
        params[5] = (((pll_reg.p3 >> 12) & 0xF0) | ((pll_reg.p2 >> 16) & 0x0F)) as u8;
        params[6] = ((pll_reg.p2 >> 8) & 0xFF) as u8;
        params[7] = (pll_reg.p2 & 0xFF) as u8;

        self.write_bulk(bus, reg, &params).await?;

        match pll {
            Si5351Pll::Plla => self.plla_freq = pll_freq,
            Si5351Pll::Pllb => self.pllb_freq = pll_freq,
        }
        Ok(())
    }

    pub async fn set_ms(
        &self,
        bus: &mut I2cBus<'_>,
        clk: Si5351Clock,
        ms_reg: Si5351RegSet,
        r_div: u8,
        div_by_4: u8,
    ) -> Result<(), T48Error> {
        let reg = SI5351_CLK0_PARAMETERS + (clk as u8 * 8);

        let mut params = [0u8; 8];
        params[0] = ((ms_reg.p3 >> 8) & 0xFF) as u8;
        params[1] = (ms_reg.p3 & 0xFF) as u8;
        params[2] = ((ms_reg.p1 >> 16) & 0x03) as u8;
        if div_by_4 != 0 {
            params[2] |= SI5351_OUTPUT_CLK_DIVBY4;
        }
        params[2] |= (r_div & 0x07) << SI5351_OUTPUT_CLK_DIV_SHIFT;

        params[3] = ((ms_reg.p1 >> 8) & 0xFF) as u8;
        params[4] = (ms_reg.p1 & 0xFF) as u8;
        params[5] = (((ms_reg.p3 >> 12) & 0xF0) | ((ms_reg.p2 >> 16) & 0x0F)) as u8;
        params[6] = ((ms_reg.p2 >> 8) & 0xFF) as u8;
        params[7] = (ms_reg.p2 & 0xFF) as u8;

        self.write_bulk(bus, reg, &params).await?;
        Ok(())
    }

    fn pll_calc(&self, pll_freq: u64, reg: &mut Si5351RegSet) {
        let ref_freq = SI5351_XTAL_FREQ * SI5351_FREQ_MULT;
        let a = pll_freq / ref_freq;
        let b = ((pll_freq % ref_freq) * 1_000_000) / ref_freq;
        let c = 1_000_000;

        reg.p1 = (128 * a + ((128 * b) / c) - 512) as u32;
        reg.p2 = (128 * b - c * ((128 * b) / c)) as u32;
        reg.p3 = c as u32;
    }

    fn multisynth_calc(&self, freq: u64, pll_freq: u64, reg: &mut Si5351RegSet) {
        let a = pll_freq / freq;
        let b = ((pll_freq % freq) * 1_000_000) / freq;
        let c = 1_000_000;

        reg.p1 = (128 * a + ((128 * b) / c) - 512) as u32;
        reg.p2 = (128 * b - c * ((128 * b) / c)) as u32;
        reg.p3 = c as u32;
    }

    fn select_r_div(&self, freq: &mut u64) -> u8 {
        let mut r_div = 0;
        let min_freq = SI5351_CLKOUT_MIN_FREQ * SI5351_FREQ_MULT;

        if *freq >= min_freq && *freq < min_freq * 2 {
            r_div = 7; // div 128
            *freq *= 128;
        } else if *freq < min_freq * 4 {
            r_div = 6; // div 64
            *freq *= 64;
        } else if *freq < min_freq * 8 {
            r_div = 5; // div 32
            *freq *= 32;
        } else if *freq < min_freq * 16 {
            r_div = 4; // div 16
            *freq *= 16;
        } else if *freq < min_freq * 32 {
            r_div = 3; // div 8
            *freq *= 8;
        } else if *freq < min_freq * 64 {
            r_div = 2; // div 4
            *freq *= 4;
        } else if *freq < min_freq * 128 {
            r_div = 1; // div 2
            *freq *= 2;
        }
        r_div
    }

    pub async fn reset_pll(&self, bus: &mut I2cBus<'_>, pll: Si5351Pll) -> Result<(), T48Error> {
        let val = match pll {
            Si5351Pll::Plla => SI5351_PLL_RESET_A,
            Si5351Pll::Pllb => SI5351_PLL_RESET_B,
        };
        bus.write_reg(self.addr, SI5351_PLL_RESET, val).await
    }

    pub async fn write_bulk(&self, bus: &mut I2cBus<'_>, reg: u8, data: &[u8]) -> Result<(), T48Error> {
        bus.start().await?;
        bus.write_byte(self.addr << 1).await?;
        bus.write_byte(reg).await?;
        for &byte in data {
            bus.write_byte(byte).await?;
        }
        bus.stop().await?;
        Ok(())
    }
}
