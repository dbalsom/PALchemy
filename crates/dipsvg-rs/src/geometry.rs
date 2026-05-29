// dipsvg-rs: A Rust library for rendering chip definitions to SVG diagrams
// Copyright (C) 2026 Daniel Balsom
// SPDX-License-Identifier: MIT OR GPL-3.0-or-later

use palcore::ChipDef;

/// Width reserved for each external pin-label column, in SVG user units.
pub const PIN_LABEL_COLUMN_WIDTH: usize = 160;
/// Gap between the package outline and each external pin-label column, in SVG user units.
pub const PIN_LABEL_GAP: usize = 20;

/// A [`ChipGeometry`] struct defines the geometric parameters that drive the layout of a chip SVG.
/// These are "transparent" properties; colors and text options are instead defined in
/// [`ChipDiagramStyle`](crate::ChipDiagramStyle).
///
/// All measurements are SVG "user units".
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ChipGeometry {
    /// Total number of pins in the chip package.
    pub pin_count: usize,
    /// Vertical distance between neighboring pin centers on the same side.
    pub pin_pitch: usize,
    /// Distance from either end of the chip body to the nearest pin on that side.
    pub pin_inset: usize,
    /// Width of the chip body, excluding pin legs.
    pub chip_width: usize,
    /// Length of each pin leg outside the chip body, including shoulder.
    pub pin_length: usize,
    /// Distance from the chip body edge to the start of pin shoulder.
    pub pin_shoulder_length: usize,
    /// Width of the wide shoulder section where the pin meets the body.
    pub pin_shoulder_width: usize,
    /// Leg width after the shoulder taper.
    pub leg_start_width: usize,
    /// Leg width at the far end of the pin.
    pub leg_end_width: usize,
    /// Corner radius for the chip body.
    pub chip_corner_radius: usize,
    /// Radius of the chip's orientation notch. Use zero to disable the notch.
    pub notch_radius: usize,
    /// Inset used for the chip-body bevel facets.
    pub bevel_inset: usize,
}

impl Default for ChipGeometry {
    fn default() -> Self {
        Self {
            pin_count: 20,
            pin_pitch: 44,
            pin_inset: 30,
            chip_width: 150,
            pin_length: 13,
            leg_start_width: 12,
            leg_end_width: 7,
            pin_shoulder_length: 8,
            pin_shoulder_width: 30,
            chip_corner_radius: 6,
            notch_radius: 12,
            bevel_inset: 3,
        }
    }
}

impl ChipGeometry {
    /// Build geometry from a parsed chip definition.
    ///
    /// This copies the chip pin count and applies any package width supplied by the chip TOML.
    pub fn from_chip(chip: &ChipDef) -> Self {
        Self::default()
            .with_pin_count(chip.pins)
            .with_chip_width_option(chip.width)
    }

    /// Apply the specified chip body width to the current geometry.
    pub fn with_chip_width(mut self, chip_width: usize) -> Self {
        self.chip_width = chip_width;
        self
    }

    /// Apply an optional chip body width, leaving the current width unchanged for `None`.
    pub fn with_chip_width_option(mut self, chip_width: Option<usize>) -> Self {
        if let Some(chip_width) = chip_width {
            self.chip_width = chip_width;
        }
        self
    }

    /// Apply a different pin count to the current geometry.
    pub fn with_pin_count(mut self, pin_count: usize) -> Self {
        self.pin_count = pin_count;
        self
    }

    /// Apply an optional notch radius, disabling the notch for `None`.
    pub fn with_notch_radius(mut self, notch_radius: Option<f32>) -> Self {
        if let Some(radius) = notch_radius {
            self.notch_radius = radius as usize;
        } else {
            self.notch_radius = 0;
        }
        self
    }

    /// Number of pins on each side of the DIP package.
    pub fn pins_per_side(&self) -> usize {
        self.pin_count / 2
    }

    /// Total SVG package height before orientation transforms.
    pub fn chip_height(&self) -> usize {
        self.pins_per_side() * self.pin_pitch + self.pin_inset * 2
    }

    /// Width of the package drawing, including pin legs but excluding external pin labels.
    pub fn svg_width(&self) -> usize {
        self.chip_width + self.pin_length * 2
    }

    /// Width of the full diagram when external pin labels are present.
    pub fn labeled_svg_width(&self) -> usize {
        self.svg_width() + PIN_LABEL_COLUMN_WIDTH * 2 + PIN_LABEL_GAP * 2
    }

    /// Y coordinate for the center of the pin at `index` on either side.
    ///
    /// `index` is zero-based within one side of the package.
    pub fn pin_center_y(&self, index: usize) -> usize {
        self.pin_inset + (index * self.pin_pitch) + self.pin_pitch / 2
    }
}
