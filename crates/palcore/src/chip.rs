use crate::types::PackageType;
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, fs, path::Path};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChipDef {
    #[serde(default)]
    pub name: String,
    pub model: String,
    pub alias: Option<String>,
    pub source: Option<String>,
    pub model_description: String,
    pub app_description: Option<String>,
    pub class: String,
    pub pins: usize,
    #[serde(default)]
    pub width: Option<usize>,
    pub package: PackageType,
    pub voltage: f32,
    #[serde(default)]
    pub io_voltage: Option<f32>,
    #[serde(default)]
    pub vpp_voltage: f32,
    pub pinout: HashMap<String, PinDef>,
}

impl ChipDef {
    pub fn interactive_io_voltage(&self) -> f32 {
        const T48_IO_MIN_V: f32 = 2.35;
        const T48_IO_MAX_V: f32 = 3.45;
        const DEFAULT_IO_V: f32 = 3.3;

        self.io_voltage.unwrap_or_else(|| {
            if self.voltage > T48_IO_MAX_V {
                DEFAULT_IO_V
            } else {
                self.voltage.clamp(T48_IO_MIN_V, T48_IO_MAX_V)
            }
        })
    }

    pub fn normalize_name(self: &mut ChipDef) {
        if self.name.is_empty() {
            self.name = self.model.clone();
        }
    }

    pub fn display_name(self: &ChipDef) -> &str {
        if self.name.is_empty() { &self.model } else { &self.name }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PinDef {
    #[serde(rename = "type")]
    pub pin_type: PinType,
    pub name: Option<String>,
    #[serde(default)]
    pub active_low: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum PinType {
    Input,
    Output,
    #[serde(rename = "IO")]
    InputOutput,
    #[serde(rename = "OE")]
    OutputEnable,
    #[serde(rename = "VCC")]
    Power,
    #[serde(rename = "GND")]
    Ground,
    #[serde(rename = "VPP")]
    Vpp,
    #[serde(rename = "NC")]
    NotConnected,
}

impl ChipDef {
    pub fn load_from_dir<P: AsRef<Path>>(dir: P) -> std::io::Result<Vec<ChipDef>> {
        let mut chips = Vec::new();
        for entry in fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.extension().and_then(|s| s.to_str()) == Some("toml") {
                let contents = fs::read_to_string(&path)?;

                match toml::from_str::<ChipDef>(&contents) {
                    Ok(mut chip) => {
                        if chip.name.is_empty() {
                            chip.name = chip.model.clone();
                        }
                        chips.push(chip)
                    }
                    Err(e) => tracing::error!("Failed to parse {:?}: {}", path, e),
                }
            }
        }
        Ok(chips)
    }
}

#[cfg(test)]
mod tests {
    use super::{ChipDef, PinDef, PinType};
    use crate::PackageType;
    use std::collections::HashMap;

    fn test_chip(voltage: f32, io_voltage: Option<f32>) -> ChipDef {
        ChipDef {
            name: "Test".to_string(),
            model: "TEST".to_string(),
            alias: None,
            source: None,
            model_description: "Test chip".to_string(),
            app_description: None,
            class: "logic".to_string(),
            pins: 14,
            width: None,
            package: PackageType::DIP,
            voltage,
            io_voltage,
            vpp_voltage: 0.0,
            pinout: HashMap::from([(
                "1".to_string(),
                PinDef {
                    pin_type: PinType::Input,
                    name: None,
                    active_low: false,
                },
            )]),
        }
    }

    #[test]
    fn explicit_io_voltage_overrides_fallback() {
        let chip = test_chip(5.0, Some(2.5));
        assert_eq!(chip.interactive_io_voltage(), 2.5);
    }

    #[test]
    fn five_volt_chips_default_to_safe_io_reference() {
        let chip = test_chip(5.0, None);
        assert_eq!(chip.interactive_io_voltage(), 3.3);
    }
}
