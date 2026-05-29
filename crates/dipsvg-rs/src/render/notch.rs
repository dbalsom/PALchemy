// dipsvg-rs: A Rust library for rendering chip definitions to SVG diagrams
// Copyright (C) 2026 Daniel Balsom
// SPDX-License-Identifier: MIT OR GPL-3.0-or-later

use super::ChipRenderer;

use svg::node::element::{path::Data, Mask, Path as SvgPath, Rectangle};

impl<'a> ChipRenderer<'a> {
    pub const NOTCH_VECTOR_THICKNESS: f32 = 5.0;
    pub const HIGH_CONTRAST_NOTCH_STROKE_WIDTH: f32 = 2.5;

    pub fn notch_cutout_mask(svg_width: usize, chip_height: usize, cx: f32, chip_top: f32, notch_radius: f32) -> Mask {
        Mask::new()
            .set("id", "chipNotchCutoutMask")
            .set("maskUnits", "userSpaceOnUse")
            .set("x", 0)
            .set("y", 0)
            .set("width", svg_width)
            .set("height", chip_height)
            .add(
                Rectangle::new()
                    .set("x", 0)
                    .set("y", 0)
                    .set("width", svg_width)
                    .set("height", chip_height)
                    .set("fill", "#ffffff"),
            )
            .add(
                SvgPath::new()
                    .set("d", ChipRenderer::notch_vector_data(cx, chip_top, notch_radius))
                    .set("fill", "#000000"),
            )
    }

    pub fn notch_vector_data(cx: f32, y: f32, radius: f32) -> Data {
        let inner_radius = (radius - ChipRenderer::NOTCH_VECTOR_THICKNESS).max(1.0);

        Data::new()
            .move_to((cx - radius, y))
            .elliptical_arc_to((radius, radius, 0, 0, 0, cx + radius, y))
            .line_to((cx + inner_radius, y))
            .elliptical_arc_to((inner_radius, inner_radius, 0, 0, 1, cx - inner_radius, y))
            .close()
    }

    pub fn high_contrast_notch_data(cx: f32, y: f32, radius: f32) -> Data {
        let stroke_inset = ChipRenderer::HIGH_CONTRAST_NOTCH_STROKE_WIDTH / 2.0;
        let inset_radius = (radius - stroke_inset).max(1.0);

        Data::new()
            .move_to((cx - inset_radius, y + stroke_inset))
            .elliptical_arc_to((inset_radius, inset_radius, 0, 0, 0, cx + inset_radius, y + stroke_inset))
    }
}
