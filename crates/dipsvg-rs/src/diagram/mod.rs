// dipsvg-rs: A Rust library for rendering chip definitions to SVG diagrams
// Copyright (C) 2026 Daniel Balsom
// SPDX-License-Identifier: MIT OR GPL-3.0-or-later

use std::{fs, path::Path};

use palcore::{ChipDef, PackageType};

use crate::geometry::{ChipGeometry, PIN_LABEL_COLUMN_WIDTH, PIN_LABEL_GAP};
use crate::label::PinLabel;
use crate::render::ChipRenderer;
use crate::style::ChipDiagramStyle;
use crate::types::ChipOrientation;
use crate::{chip_from_toml, DipSvgError};

#[derive(Debug, Clone, PartialEq)]
pub struct ChipDiagramOptions {
    pub geometry: ChipGeometry,
    pub style: ChipDiagramStyle,
}

impl Default for ChipDiagramOptions {
    fn default() -> Self {
        Self {
            geometry: ChipGeometry::default(),
            style: ChipDiagramStyle::default(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct ChipDiagram {
    pub(crate) name: String,
    pub(crate) alias: Option<String>,
    pub(crate) pin_labels: Vec<Option<PinLabel>>,
    pub(crate) chip_width: Option<usize>,
    pub(crate) geometry: ChipGeometry,
    pub(crate) style: ChipDiagramStyle,
}

impl ChipDiagram {
    pub fn new(name: impl Into<String>, geometry: ChipGeometry) -> Self {
        Self {
            name: name.into(),
            alias: None,
            pin_labels: vec![None; geometry.pin_count],
            chip_width: None,
            geometry,
            style: ChipDiagramStyle::default(),
        }
    }

    pub fn from_chip(chip: &ChipDef) -> Result<Self, DipSvgError> {
        if chip.package != PackageType::DIP {
            return Err(DipSvgError::UnsupportedPackage { package: chip.package });
        }

        let mut diagram = Self::new(chip.display_name(), ChipGeometry::from_chip(chip))
            .with_alias_option(chip.alias.clone())
            .with_chip_width_option(chip.width);
        for pin in 1..=chip.pins {
            if let Some(definition) = chip.pinout.get(&pin.to_string()) {
                diagram = diagram.with_pin_label(pin, PinLabel::from_pin_def(pin, definition));
            }
        }

        Ok(diagram)
    }

    pub fn from_toml(input: impl AsRef<str>) -> Result<Self, DipSvgError> {
        let chip = chip_from_toml(input)?;
        Self::from_chip(&chip)
    }

    pub fn from_toml_file<P: AsRef<Path>>(path: P) -> Result<Self, DipSvgError> {
        let input = fs::read_to_string(path)?;
        Self::from_toml(input)
    }

    pub fn with_alias(mut self, alias: impl Into<String>) -> Self {
        self.alias = Some(alias.into());
        self
    }

    pub fn with_alias_option(mut self, alias: Option<String>) -> Self {
        self.alias = alias;
        self
    }

    pub fn without_alias(mut self) -> Self {
        self.alias = None;
        self
    }

    pub fn with_pin_count(mut self, pin_count: usize) -> Self {
        self.geometry = self.geometry.with_pin_count(pin_count);
        self.pin_labels.resize(pin_count, None);
        self
    }

    pub fn with_chip_width(mut self, chip_width: usize) -> Self {
        self.chip_width = Some(chip_width);
        self.geometry = self.geometry.with_chip_width(chip_width);
        self
    }

    pub fn with_chip_width_option(mut self, chip_width: Option<usize>) -> Self {
        if let Some(chip_width) = chip_width {
            self = self.with_chip_width(chip_width);
        }
        self
    }

    pub fn with_pin_label(mut self, pin: usize, label: impl Into<PinLabel>) -> Self {
        if (1..=self.geometry.pin_count).contains(&pin) {
            self.pin_labels[pin - 1] = Some(label.into());
        }
        self
    }

    pub fn with_pin_labels<I, L>(mut self, labels: I) -> Self
    where
        I: IntoIterator<Item = (usize, L)>,
        L: Into<PinLabel>,
    {
        for (pin, label) in labels {
            if (1..=self.geometry.pin_count).contains(&pin) {
                self.pin_labels[pin - 1] = Some(label.into());
            }
        }
        self
    }

    pub fn without_pin_labels(mut self) -> Self {
        self.pin_labels.fill(None);
        self
    }

    pub fn with_options(mut self, options: ChipDiagramOptions) -> Self {
        let pin_count = self.geometry.pin_count;
        self.geometry = options.geometry.with_pin_count(pin_count);
        if let Some(chip_width) = self.chip_width {
            self.geometry = self.geometry.with_chip_width(chip_width);
        }
        self.style = options.style;
        self
    }

    pub fn with_style(mut self, style: ChipDiagramStyle) -> Self {
        self.style = style;
        self
    }

    pub fn geometry(&self) -> &ChipGeometry {
        &self.geometry
    }

    pub fn pin_count(&self) -> usize {
        self.geometry.pin_count
    }

    pub fn has_pin_labels(&self) -> bool {
        self.pin_labels.iter().any(Option::is_some)
    }

    pub fn pin_label(&self, pin: usize) -> Option<&PinLabel> {
        pin.checked_sub(1)
            .and_then(|index| self.pin_labels.get(index))
            .and_then(Option::as_ref)
    }

    pub fn document_size(&self) -> Result<(usize, usize), DipSvgError> {
        validate_pin_count(self.geometry.pin_count)?;
        let metrics = ChipMetrics::new(self);
        Ok((metrics.document_width, metrics.document_height))
    }

    pub fn render(&self) -> Result<String, DipSvgError> {
        validate_pin_count(self.geometry.pin_count)?;
        Ok(ChipRenderer::new(self).render())
    }
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct ChipMetrics {
    pub(crate) pins_per_side: usize,
    pub(crate) diagram_width: usize,
    pub(crate) chip_height: usize,
    pub(crate) document_width: usize,
    pub(crate) document_height: usize,
    pub(crate) view_box_width: usize,
    pub(crate) view_box_height: usize,
    pub(crate) has_pin_labels: bool,
    pub(crate) chip_origin_x: usize,
    pub(crate) left_pin_label_x: usize,
    pub(crate) right_pin_label_x: usize,
    pub(crate) chip_body_height: usize,
    pub(crate) chip_left: usize,
    pub(crate) chip_top: usize,
    pub(crate) chip_right: usize,
    pub(crate) chip_bottom: usize,
    pub(crate) cx: usize,
    pub(crate) notch_radius: Option<f32>,
}

impl ChipMetrics {
    pub(crate) fn new(diagram: &ChipDiagram) -> Self {
        let geometry = &diagram.geometry;
        let pins_per_side = geometry.pins_per_side();
        let has_pin_labels = diagram.has_pin_labels();
        let package_width = geometry.svg_width();
        let diagram_width = if has_pin_labels {
            geometry.labeled_svg_width()
        } else {
            package_width
        };
        let chip_height = geometry.chip_height();
        let (document_width, document_height, view_box_width, view_box_height) = match diagram.style.orientation {
            ChipOrientation::NotchUp | ChipOrientation::NotchDown => {
                (diagram_width, chip_height, diagram_width, chip_height)
            }
            ChipOrientation::NotchLeft | ChipOrientation::NotchRight => {
                (chip_height, diagram_width, chip_height, diagram_width)
            }
        };
        let chip_origin_x = if has_pin_labels {
            PIN_LABEL_COLUMN_WIDTH + PIN_LABEL_GAP
        } else {
            0
        };
        let left_pin_label_x = chip_origin_x.saturating_sub(PIN_LABEL_GAP);
        let right_pin_label_x = chip_origin_x + package_width + PIN_LABEL_GAP;
        let chip_body_height = chip_height - geometry.pin_inset * 2;
        let chip_left = chip_origin_x + geometry.pin_length;
        let chip_top = geometry.pin_inset;
        let chip_right = chip_left + geometry.chip_width;
        let chip_bottom = chip_top + chip_body_height;
        let cx = chip_left + geometry.chip_width / 2;
        let notch_radius = if geometry.notch_radius > 0 {
            Some(geometry.notch_radius as f32)
        } else {
            None
        };

        Self {
            pins_per_side,
            diagram_width,
            chip_height,
            document_width,
            document_height,
            view_box_width,
            view_box_height,
            has_pin_labels,
            chip_origin_x,
            left_pin_label_x,
            right_pin_label_x,
            chip_body_height,
            chip_left,
            chip_top,
            chip_right,
            chip_bottom,
            cx,
            notch_radius,
        }
    }
}

fn validate_pin_count(pin_count: usize) -> Result<(), DipSvgError> {
    if pin_count == 0 || pin_count % 2 != 0 {
        Err(DipSvgError::InvalidPinCount(pin_count))
    } else {
        Ok(())
    }
}
