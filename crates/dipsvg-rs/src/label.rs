// dipsvg-rs: A Rust library for rendering chip definitions to SVG diagrams
// Copyright (C) 2026 Daniel Balsom
// SPDX-License-Identifier: MIT OR GPL-3.0-or-later

//! Pin-name label data used by chip diagrams.

use palcore::{PinDef, PinType};

/// Text to be rendered at the end of a pin.
///
/// A label can be supplied directly with [`ChipDiagram::with_pin_label`](crate::ChipDiagram::with_pin_label)
/// or derived from a TOML chip definition. Active-low labels are rendered with an overbar.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct PinLabel {
    /// Label text to display at end of pin.
    pub text: String,
    /// Whether the label should be drawn with an overbar indicating it is an active-low pin.
    pub active_low: bool,
}

impl PinLabel {
    /// Create a pin label with normal, non-active-low text.
    pub fn new(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            active_low: false,
        }
    }

    /// Return this label with active-low rendering enabled or disabled.
    pub fn active_low(mut self, active_low: bool) -> Self {
        self.active_low = active_low;
        self
    }

    pub(crate) fn from_pin_def(pin: usize, definition: &PinDef) -> Self {
        let raw_text = definition
            .name
            .clone()
            .unwrap_or_else(|| fallback_pin_label(pin, &definition.pin_type));
        let (text, slash_active_low) = raw_text
            .strip_prefix('/')
            .map(|text| (text.to_string(), true))
            .unwrap_or((raw_text, false));

        Self {
            text,
            active_low: definition.active_low || slash_active_low,
        }
    }
}

impl From<String> for PinLabel {
    fn from(value: String) -> Self {
        Self::new(value)
    }
}

impl From<&str> for PinLabel {
    fn from(value: &str) -> Self {
        Self::new(value)
    }
}

fn fallback_pin_label(pin: usize, pin_type: &PinType) -> String {
    match pin_type {
        PinType::Power => "VCC".to_string(),
        PinType::Ground => "GND".to_string(),
        PinType::Vpp => "VPP".to_string(),
        PinType::OutputEnable => "OE".to_string(),
        PinType::NotConnected => "NC".to_string(),
        PinType::Input | PinType::Output | PinType::InputOutput => format!("P{pin}"),
    }
}
