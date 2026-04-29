use super::ChipRenderer;
use crate::types::*;
use crate::util::*;

use svg::node::element::Definitions;

impl<'a> ChipRenderer<'a> {
    pub fn definitions(shade_angle: f32, orientation: ChipOrientation) -> Definitions {
        let highlight_coords = gradient_coords(shade_angle);
        let shadow_coords = gradient_coords(shade_angle + 180.0);
        let body_coords = orientation.body_gradient_coords();

        Definitions::new()
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
                gradient_coords_from_strs("0%", "0%", "0%", "100%"),
                &[
                    ("0%", "#f8fafc", "0.95"),
                    ("48%", "var(--chip-pin, #cbd5e1)", "1"),
                    ("100%", "#64748b", "1"),
                ],
            ))
    }
}
