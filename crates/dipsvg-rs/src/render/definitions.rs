// dipsvg-rs: A Rust library for rendering chip definitions to SVG diagrams
// Copyright (C) 2026 Daniel Balsom
// SPDX-License-Identifier: MIT OR GPL-3.0-or-later

use super::ChipRenderer;
use crate::util::*;
use crate::ChipDiagramOptions;

use svg::node::element::{Definitions, Filter, FilterEffectGaussianBlur};

impl<'a> ChipRenderer<'a> {
    pub(super) fn definitions(options: &ChipDiagramOptions) -> Definitions {
        let shade_angle = options.style.effective_shade_angle();
        let orientation = options.style.orientation;
        let highlight_coords = gradient_coords(shade_angle);
        let shadow_coords = gradient_coords(shade_angle + 180.0);
        let body_coords = orientation.body_gradient_coords();

        let definitions = Definitions::new()
            .add(gradient_with_style_stops(
                "chipGradient",
                body_coords,
                &[
                    ("0%", "stop-color:var(--chip-body, #3a3e44)"),
                    ("100%", "stop-color:var(--chip-body2, #272a30)"),
                ],
            ))
            .add(gradient(
                "chipBevelHighlight",
                highlight_coords,
                &[
                    ("0%", "#ffffff", "0.36"),
                    ("35%", "#ffffff", "0.08"),
                    ("100%", "#ffffff", "0.00"),
                ],
            ))
            .add(gradient(
                "chipBevelShadow",
                shadow_coords.clone(),
                &[
                    ("0%", "#000000", "0.24"),
                    ("45%", "#000000", "0.12"),
                    ("100%", "var(--chip-body2, #272a30)", "0.34"),
                ],
            ))
            .add(gradient_with_style_stops(
                "chipNotchInset",
                shadow_coords.clone(),
                &[
                    ("0%", "stop-color:var(--chip-highlight, #9b9fa4)"),
                    ("66%", "stop-color:var(--chip-body, #3a3e44)"),
                    ("100%", "stop-color:var(--chip-body2, #272a30)"),
                ],
            ))
            .add(gradient_with_style_stops(
                "pinIndicatorInset",
                shadow_coords,
                &[
                    ("0%", "stop-color:var(--chip-highlight, #9b9fa4)"),
                    ("50%", "stop-color:var(--chip-body, #3a3e44)"),
                    ("100%", "stop-color:var(--chip-body2, #272a30)"),
                ],
            ))
            .add(gradient(
                "pinGradient",
                gradient_coords_from_strs("0%", "0%", "100%", "0%"),
                &[
                    ("0%", "#8793a3", "1"),
                    ("33%", "var(--chip-pin, #cbd5e1)", "1"),
                    ("45%", "#f8fafc", "0.96"),
                    ("48%", "#f8fafc", "0.96"),
                    ("50%", "var(--chip-pin, #cbd5e1)", "1"),
                    ("66%", "#475569", "1"),
                ],
            ))
            .add(gradient(
                "pinGradientReverse",
                gradient_coords_from_strs("100%", "0%", "0%", "0%"),
                &[
                    ("0%", "#8793a3", "1"),
                    ("33%", "var(--chip-pin, #cbd5e1)", "1"),
                    ("45%", "#f8fafc", "0.96"),
                    ("48%", "#f8fafc", "0.96"),
                    ("50%", "var(--chip-pin, #cbd5e1)", "1"),
                    ("66%", "#475569", "1"),
                ],
            ));

        if ChipRenderer::include_body_drop_shadow(options) {
            definitions.add(
                Filter::new()
                    .set("id", super::CHIP_BODY_DROP_SHADOW_FILTER_ID)
                    .set("x", "-20%")
                    .set("y", "-20%")
                    .set("width", "140%")
                    .set("height", "140%")
                    .set("color-interpolation-filters", "sRGB")
                    .add(FilterEffectGaussianBlur::new().set("stdDeviation", super::CHIP_BODY_DROP_SHADOW_BLUR)),
            )
        } else {
            definitions
        }
    }
}
