// dipsvg-rs: A Rust library for rendering chip definitions to SVG diagrams
// Copyright (C) 2026 Daniel Balsom
// SPDX-License-Identifier: MIT OR GPL-3.0-or-later

//! Visual style options for rendered chip diagrams.

use crate::types::ChipOrientation;

/// Visual styling and orientation settings for a [`ChipDiagram`](crate::ChipDiagram).
///
/// Geometry such as pin pitch, chip width, and leg shape belongs in
/// [`ChipGeometry`](crate::ChipGeometry). This type controls colors, shading behavior, label
/// orientation, and optional decorative effects.
#[derive(Debug, Clone, PartialEq)]
pub struct ChipDiagramStyle {
    /// Accessibility: Render with high-contrast colors and suppress nonessential decorative effects.
    pub high_contrast: bool,
    /// Base angle, in degrees, used for chip body bevel shading.
    pub shade_angle: f32,
    /// Whether to render a blurred chip body drop shadow below the chip.
    pub chip_body_drop_shadow: bool,
    /// Distance to offset the chip body drop shadow, in SVG user units.
    pub shadow_distance: f32,
    /// Whether text should remain upright when the chip is rendered in a rotated orientation.
    pub keep_labels_upright: bool,
    /// Pin-name label color used as the `light-dark()` light-theme value.
    pub pin_label_light_color: String,
    /// Pin-name label color used as the `light-dark()` dark-theme value.
    pub pin_label_dark_color: String,

    /// Orientation of the rendered chip, expressed as the notch/pin-one direction.
    pub orientation: ChipOrientation,
}

impl Default for ChipDiagramStyle {
    fn default() -> Self {
        Self {
            high_contrast: false,
            shade_angle: 45.0,
            chip_body_drop_shadow: false,
            shadow_distance: 15.0,
            keep_labels_upright: false,
            pin_label_light_color: "#0f172a".to_string(),
            pin_label_dark_color: "#f8fafc".to_string(),
            orientation: ChipOrientation::default(),
        }
    }
}

impl ChipDiagramStyle {
    /// Return this style with high-contrast rendering enabled or disabled.
    pub fn with_high_contrast(mut self, high_contrast: bool) -> Self {
        self.high_contrast = high_contrast;
        self
    }

    /// Return this style with a different base bevel shade angle, in degrees.
    pub fn with_shade_angle(mut self, shade_angle: f32) -> Self {
        self.shade_angle = shade_angle;
        self
    }

    /// Return this style with chip-body drop shadow rendering enabled or disabled.
    ///
    /// Accessibility: The renderer suppresses this shadow in high-contrast mode.
    pub fn with_chip_body_drop_shadow(mut self, chip_body_drop_shadow: bool) -> Self {
        self.chip_body_drop_shadow = chip_body_drop_shadow;
        self
    }

    /// Return this style with a different chip-body drop shadow offset distance.
    pub fn with_shadow_distance(mut self, shadow_distance: f32) -> Self {
        self.shadow_distance = shadow_distance;
        self
    }

    /// Return this style with upright-label compensation enabled or disabled.
    pub fn with_keep_labels_upright(mut self, keep_labels_upright: bool) -> Self {
        self.keep_labels_upright = keep_labels_upright;
        self
    }

    /// Return this style with a different light-theme pin-name label color.
    pub fn with_pin_label_light_color(mut self, color: impl Into<String>) -> Self {
        self.pin_label_light_color = color.into();
        self
    }

    /// Return this style with a different dark-theme pin-name label color.
    pub fn with_pin_label_dark_color(mut self, color: impl Into<String>) -> Self {
        self.pin_label_dark_color = color.into();
        self
    }

    /// Return this style with both theme-reactive pin-name label colors replaced.
    ///
    /// Normal rendering emits these colors through CSS `light-dark(light, dark)`. High-contrast
    /// rendering uses fixed high-contrast text colors instead.
    pub fn with_pin_label_theme_colors(
        mut self,
        light_color: impl Into<String>,
        dark_color: impl Into<String>,
    ) -> Self {
        self.pin_label_light_color = light_color.into();
        self.pin_label_dark_color = dark_color.into();
        self
    }

    /// Shade angle after compensating for the configured chip orientation.
    ///
    /// This keeps bevel lighting visually consistent when the package is rotated.
    pub fn effective_shade_angle(&self) -> f32 {
        match self.orientation {
            ChipOrientation::NotchUp => self.shade_angle,
            ChipOrientation::NotchLeft => self.shade_angle + 90.0,
            ChipOrientation::NotchRight => self.shade_angle - 90.0,
            ChipOrientation::NotchDown => self.shade_angle + 180.0,
        }
    }

    /// Return this style with a different chip orientation.
    pub fn with_orientation(mut self, orientation: ChipOrientation) -> Self {
        self.orientation = orientation;
        self
    }
}
