// dipsvg-rs: A Rust library for rendering chip definitions to SVG diagrams
// Copyright (C) 2026 Daniel Balsom
// SPDX-License-Identifier: MIT OR GPL-3.0-or-later

//! SVG rendering for DIP chip definitions.
//!
//! `dipsvg-rs` renders TOML chip definitions into pretty SVG diagrams.
//!
//! It can render directly from TOML text or files, from a parsed [`ChipDef`], or through the
//! [`ChipDiagram`] builder when callers need to customize geometry, style, orientation, or pin
//! labels.
//!
//! The usual high-level entry points are [`render_toml`], [`render_toml_file`], and [`render_chip`].
//! For lower-level control, construct a [`ChipDiagram`] with a [`ChipGeometry`] and optional
//! [`ChipDiagramStyle`].
//!
//! ```no_run
//! use dipsvg::{render_toml_file, ChipDiagramOptions};
//!
//! let svg = render_toml_file("chips/PAL16L8.toml", ChipDiagramOptions::default())?;
//! # Ok::<(), dipsvg::DipSvgError>(())
//! ```

pub mod diagram;
pub mod geometry;
pub mod label;
pub mod render;
pub mod style;
pub mod types;
pub mod util;

use std::path::Path;

use palcore::{ChipDef, PackageType};
use thiserror::Error;

pub use diagram::{ChipDiagram, ChipDiagramOptions};
pub use geometry::{ChipGeometry, PIN_LABEL_COLUMN_WIDTH, PIN_LABEL_GAP};
pub use label::PinLabel;
pub use style::ChipDiagramStyle;
use types::*;
pub use util::*;

#[deprecated(note = "use ChipDiagramOptions instead")]
pub type DipSvgOptions = ChipDiagramOptions;

#[derive(Debug, Error)]
pub enum DipSvgError {
    #[error("failed to read chip definition: {0}")]
    Io(#[from] std::io::Error),
    #[error("failed to parse chip definition TOML: {0}")]
    Toml(#[from] toml::de::Error),
    #[error("unsupported package {package:?}; only DIP packages can be rendered")]
    UnsupportedPackage { package: PackageType },
    #[error("DIP pin count must be a positive even number, got {0}")]
    InvalidPinCount(usize),
}

pub fn chip_from_toml(input: impl AsRef<str>) -> Result<ChipDef, DipSvgError> {
    let mut chip = toml::from_str::<ChipDef>(input.as_ref())?;
    chip.normalize_name();
    Ok(chip)
}

pub fn render_toml(input: impl AsRef<str>, options: ChipDiagramOptions) -> Result<String, DipSvgError> {
    ChipDiagram::from_toml(input)?.with_options(options).render()
}

pub fn render_toml_file<P: AsRef<Path>>(path: P, options: ChipDiagramOptions) -> Result<String, DipSvgError> {
    ChipDiagram::from_toml_file(path)?.with_options(options).render()
}

pub fn render_chip(chip: &ChipDef, options: ChipDiagramOptions) -> Result<String, DipSvgError> {
    ChipDiagram::from_chip(chip)?.with_options(options).render()
}

pub fn generate_dip_svg(name: &str, alias: Option<&str>, geometry: &ChipGeometry, high_contrast: bool) -> String {
    let style = ChipDiagramStyle::default()
        .with_high_contrast(high_contrast)
        .with_orientation(ChipOrientation::NotchUp);

    ChipDiagram::new(name, *geometry)
        .with_alias_option(alias.map(ToOwned::to_owned))
        .with_style(style)
        .render()
        .expect("generate_dip_svg requires geometry with a positive even DIP pin count")
}

#[cfg(test)]
mod tests {
    use super::{
        chip_from_toml, generate_dip_svg, render_toml, ChipDiagram, ChipDiagramOptions, ChipDiagramStyle, ChipGeometry,
        ChipOrientation, DipSvgError,
    };

    const PAL16L8: &str = include_str!("../../../chips/PAL16L8.toml");
    const INTEL_8253: &str = include_str!("../../../chips/8253.toml");

    #[test]
    fn parses_existing_chip_toml_and_uses_model_as_default_name() {
        let chip = chip_from_toml(PAL16L8).expect("chip should parse");

        assert_eq!(chip.name, "PAL16L8");
        assert_eq!(chip.pins, 20);
    }

    #[test]
    fn toml_helpers_accept_owned_strings() {
        let toml = PAL16L8.to_string();
        let chip = chip_from_toml(toml.clone()).expect("owned TOML should parse");
        let svg = render_toml(toml, ChipDiagramOptions::default()).expect("owned TOML should render");

        assert_eq!(chip.name, "PAL16L8");
        assert!(svg.contains("PAL16L8"));
    }

    #[test]
    fn renders_existing_chip_toml_to_svg() {
        let diagram = ChipDiagram::from_toml(PAL16L8).expect("chip TOML should parse");
        let (width, height) = diagram.document_size().expect("chip dimensions should be valid");
        let svg = diagram.render().expect("chip should render");

        assert_eq!(diagram.geometry().pin_count, 20);
        assert!(svg.starts_with("<svg "));
        assert_svg_size(&svg, width, height);
        assert!(svg.contains("PAL16L8"));
        assert!(svg.contains("\n1\n</text>"));
        assert!(svg.contains("\n20\n</text>"));
        assert!(svg.contains("IN_1"));
        assert!(svg.contains("OUT_1"));
        assert!(svg.contains("GND"));
        assert!(svg.contains("VCC"));
        assert!(svg.contains(r#"class="dip-pin-label""#));
        assert!(svg.contains(r#"fill="light-dark(#0f172a, #f8fafc)""#));
        assert!(svg.contains(r#"text-decoration="overline""#));
    }

    #[test]
    fn toml_width_sets_chip_geometry_width() {
        let toml = r#"
          model = "WIDE24"
          model_description = "Wide DIP test"
          class = "logic"
          pins = 24
          width = 220
          voltage = 5.0
          package = "DIP"

          [pinout]
          1 = { type = "IO", name = "D7" }
          12 = { type = "GND" }
          24 = { type = "VCC" }
          "#;

        let diagram = ChipDiagram::from_toml(toml).expect("chip TOML should parse");
        assert_eq!(diagram.geometry().chip_width, 220);

        let (width, height) = diagram.document_size().expect("chip dimensions should be valid");
        let svg = diagram.render().expect("chip should render");
        assert_svg_size(&svg, width, height);
        assert!(svg.contains(r#"width="220""#));
    }

    #[test]
    fn renders_8253_with_custom_chip_width() {
        let diagram = ChipDiagram::from_toml(INTEL_8253).expect("8253 TOML should parse");
        assert_eq!(diagram.geometry().chip_width, 220);
        assert_eq!(diagram.geometry().pin_count, 24);

        let (width, height) = diagram.document_size().expect("8253 dimensions should be valid");
        let svg = diagram.render().expect("8253 should render");
        assert_svg_size(&svg, width, height);
        assert!(svg.contains(r#"width="220""#));
        assert!(svg.contains("Intel 8253"));
        assert!(svg.contains("GATE0"));
        assert!(svg.contains("CS"));
        assert!(svg.contains(r#"text-decoration="overline""#));
    }

    #[test]
    fn chip_diagram_builder_can_render_custom_pin_labels() {
        let diagram = ChipDiagram::new("CUSTOM", ChipGeometry::default().with_pin_count(14))
            .with_pin_label(1, "A0")
            .with_pin_label(14, "VCC")
            .with_options(ChipDiagramOptions {
                geometry: ChipGeometry::default(),
                style: ChipDiagramStyle::default().with_pin_label_theme_colors("#111111", "#eeeeee"),
            });
        let (width, height) = diagram.document_size().expect("diagram dimensions should be valid");
        let svg = diagram.render().expect("builder should render");

        assert_eq!(diagram.pin_count(), 14);
        assert_eq!(diagram.geometry().pin_count, 14);
        assert_svg_size(&svg, width, height);
        assert!(svg.contains(r#"fill="light-dark(#111111, #eeeeee)""#));
        assert!(svg.contains("A0"));
        assert!(svg.contains("VCC"));
    }

    #[test]
    fn chip_diagram_builder_renders_low_level_diagram() {
        let diagram = ChipDiagram::new("PAL16L8", ChipGeometry::default())
            .with_alias("PAL")
            .with_options(ChipDiagramOptions {
                geometry: ChipGeometry::default(),
                style: ChipDiagramStyle::default(),
            });
        let (width, height) = diagram.document_size().expect("diagram dimensions should be valid");
        let svg = diagram.render().expect("builder should render");

        assert!(svg.contains("PAL"));
        assert_svg_size(&svg, width, height);
        assert!(svg.contains(r#"fill="url(#pinGradient)""#));
    }

    #[test]
    fn chip_diagram_builder_accepts_toml_and_options() {
        let svg = ChipDiagram::from_toml(PAL16L8)
            .expect("chip TOML should parse")
            .with_options(ChipDiagramOptions {
                geometry: ChipGeometry::default(),
                style: ChipDiagramStyle::default().with_high_contrast(true),
            })
            .render()
            .expect("builder should render");

        assert!(svg.contains("PAL16L8"));
        assert!(svg.contains(r##"fill="#111111""##));
    }

    #[test]
    fn chip_diagram_builder_rejects_invalid_pin_count_on_render() {
        let error = ChipDiagram::new("bad", ChipGeometry::default().with_pin_count(19))
            .render()
            .expect_err("odd pin count should fail");

        assert!(matches!(error, DipSvgError::InvalidPinCount(19)));
    }

    #[test]
    fn low_level_renderer_preserves_high_contrast_style() {
        let svg = generate_dip_svg("PAL", None, &ChipGeometry::default(), true);

        assert!(svg.contains(r##"fill="#111111""##));
        assert!(svg.contains(r##"stroke="#ffffff""##));
        assert!(svg.contains(r##"class="dip-pin-leg""##));
        assert!(svg.contains(r##"fill="#ffffff" stroke="none""##));
    }

    #[test]
    fn high_contrast_mode_renders_visible_notch() {
        let svg = generate_dip_svg("PAL", None, &ChipGeometry::default(), true);
        let notch_path = line_with(&svg, r#"stroke-linecap="round""#);

        assert!(svg.contains(r##"fill="none" stroke="#ffffff" stroke-linecap="round" stroke-width="2.5""##));
        assert!(notch_path.contains(" A"));
    }

    #[test]
    fn low_level_renderer_uses_gradient_for_pin_legs() {
        let svg = generate_dip_svg("PAL", None, &ChipGeometry::default(), false);
        let pin_legs = lines_with(&svg, r#"class="dip-pin-leg""#);

        assert!(svg.contains(r#"id="pinGradient""#));
        assert!(svg.contains(r#"id="pinGradientReverse""#));
        assert!(svg.contains(r#"id="pinGradient" x1="0%" x2="100%" y1="0%" y2="0%""#));
        assert!(svg.contains(r#"id="pinGradientReverse" x1="100%" x2="0%" y1="0%" y2="0%""#));
        assert!(has_gradient_stop(&svg, "0%", "#8793a3", "1"));
        assert!(has_gradient_stop(&svg, "33%", "var(--chip-pin, #cbd5e1)", "1"));
        assert!(has_gradient_stop(&svg, "45%", "#f8fafc", "0.96"));
        assert!(has_gradient_stop(&svg, "48%", "#f8fafc", "0.96"));
        assert!(has_gradient_stop(&svg, "50%", "var(--chip-pin, #cbd5e1)", "1"));
        assert!(has_gradient_stop(&svg, "66%", "#475569", "1"));
        assert_eq!(pin_legs.len(), 20);
        assert!(pin_legs.iter().all(|line| line.starts_with("<path ")));
        assert_eq!(
            pin_legs
                .iter()
                .filter(|line| line.contains(r#"fill="url(#pinGradient)" stroke="none""#))
                .count(),
            10
        );
        assert_eq!(
            pin_legs
                .iter()
                .filter(|line| line.contains(r#"fill="url(#pinGradientReverse)" stroke="none""#))
                .count(),
            10
        );
        assert!(pin_legs.iter().all(|line| line.matches(" L").count() >= 7));
        assert!(!svg.contains(r#"<rect fill="url(#pinGradient)""#));
    }

    #[test]
    fn chip_body_drop_shadow_is_opt_in() {
        let default_svg = ChipDiagram::new("PAL", ChipGeometry::default())
            .render()
            .expect("builder should render");
        let shadow_svg = ChipDiagram::new("PAL", ChipGeometry::default())
            .with_options(ChipDiagramOptions {
                geometry: ChipGeometry::default(),
                style: ChipDiagramStyle::default().with_chip_body_drop_shadow(true),
            })
            .render()
            .expect("builder should render");

        assert!(!default_svg.contains("chipBodyDropShadowBlur"));
        assert!(!default_svg.contains(r#"class="dip-chip-body-shadow""#));
        assert!(shadow_svg.contains(r#"id="chipBodyDropShadowBlur""#));
        assert!(shadow_svg.contains(r#"<feGaussianBlur stdDeviation="3"/>"#));
        assert!(shadow_svg.contains(r#"class="dip-chip-body-shadow""#));
        assert!(shadow_svg.contains(r#"filter="url(#chipBodyDropShadowBlur)""#));
        assert!(shadow_svg.contains(r##"fill="#000000""##));
    }

    #[test]
    fn chip_body_drop_shadow_is_suppressed_in_high_contrast_mode() {
        let svg = ChipDiagram::new("PAL", ChipGeometry::default())
            .with_options(ChipDiagramOptions {
                geometry: ChipGeometry::default(),
                style: ChipDiagramStyle::default()
                    .with_chip_body_drop_shadow(true)
                    .with_high_contrast(true),
            })
            .render()
            .expect("builder should render");

        assert!(!svg.contains("chipBodyDropShadowBlur"));
        assert!(!svg.contains(r#"class="dip-chip-body-shadow""#));
        assert!(svg.contains(r##"fill="#111111""##));
        assert!(svg.contains(r##"stroke="#ffffff""##));
    }

    #[test]
    fn chip_body_drop_shadow_renders_above_pin_legs_and_below_body() {
        let svg = ChipDiagram::new("PAL", ChipGeometry::default())
            .with_options(ChipDiagramOptions {
                geometry: ChipGeometry::default(),
                style: ChipDiagramStyle::default().with_chip_body_drop_shadow(true),
            })
            .render()
            .expect("builder should render");

        let pin_leg = svg.find(r#"class="dip-pin-leg""#).expect("expected pin legs to render");
        let shadow = svg
            .find(r#"class="dip-chip-body-shadow""#)
            .expect("expected chip body shadow to render");
        let body = svg
            .find(r#"<rect fill="url(#chipGradient)""#)
            .expect("expected chip body to render");

        assert!(pin_leg < shadow);
        assert!(shadow < body);
    }

    #[test]
    fn chip_body_drop_shadow_offsets_follow_style_angle_and_orientation() {
        for (orientation, expected_offset) in [
            (ChipOrientation::NotchUp, "translate(18.00 0.00)"),
            (ChipOrientation::NotchLeft, "translate(0.00 18.00)"),
            (ChipOrientation::NotchRight, "translate(0.00 -18.00)"),
            (ChipOrientation::NotchDown, "translate(-18.00 0.00)"),
        ] {
            let svg = ChipDiagram::new("PAL", ChipGeometry::default())
                .with_options(ChipDiagramOptions {
                    geometry: ChipGeometry::default(),
                    style: ChipDiagramStyle::default()
                        .with_chip_body_drop_shadow(true)
                        .with_shadow_distance(18.0)
                        .with_shade_angle(0.0)
                        .with_orientation(orientation),
                })
                .render()
                .expect("builder should render");
            let shadow = line_with(&svg, r#"class="dip-chip-body-shadow""#);

            assert!(shadow.contains(&format!(r#"transform="{expected_offset}""#)));
        }
    }

    #[test]
    fn chip_body_drop_shadow_distance_controls_offset_magnitude() {
        for (distance, expected_offset) in [(8.0, "translate(8.00 0.00)"), (24.0, "translate(24.00 0.00)")] {
            let svg = ChipDiagram::new("PAL", ChipGeometry::default())
                .with_options(ChipDiagramOptions {
                    geometry: ChipGeometry::default(),
                    style: ChipDiagramStyle::default()
                        .with_chip_body_drop_shadow(true)
                        .with_shadow_distance(distance)
                        .with_shade_angle(0.0),
                })
                .render()
                .expect("builder should render");
            let shadow = line_with(&svg, r#"class="dip-chip-body-shadow""#);

            assert!(shadow.contains(&format!(r#"transform="{expected_offset}""#)));
        }
    }

    #[test]
    fn pin_shoulder_length_controls_taper_position_without_changing_package_width() {
        let short_shoulder = ChipGeometry {
            pin_length: 60,
            pin_shoulder_length: 12,
            ..ChipGeometry::default()
        };
        let long_shoulder = ChipGeometry {
            pin_length: 60,
            pin_shoulder_length: 48,
            ..ChipGeometry::default()
        };
        let short_svg = generate_dip_svg("PAL", None, &short_shoulder, false);
        let long_svg = generate_dip_svg("PAL", None, &long_shoulder, false);

        assert_svg_size(&short_svg, short_shoulder.svg_width(), short_shoulder.chip_height());
        assert_svg_size(&long_svg, long_shoulder.svg_width(), long_shoulder.chip_height());
        assert_ne!(
            line_with(&short_svg, r#"class="dip-pin-leg""#),
            line_with(&long_svg, r#"class="dip-pin-leg""#)
        );
    }

    #[test]
    fn leg_end_width_controls_leg_end_without_changing_package_width() {
        let narrow_end = ChipGeometry {
            pin_length: 60,
            leg_start_width: 18,
            leg_end_width: 8,
            pin_shoulder_width: 28,
            ..ChipGeometry::default()
        };
        let wide_end = ChipGeometry {
            pin_length: 60,
            leg_start_width: 18,
            leg_end_width: 14,
            pin_shoulder_width: 28,
            ..ChipGeometry::default()
        };
        let narrow_svg = generate_dip_svg("PAL", None, &narrow_end, false);
        let wide_svg = generate_dip_svg("PAL", None, &wide_end, false);

        assert_svg_size(&narrow_svg, narrow_end.svg_width(), narrow_end.chip_height());
        assert_svg_size(&wide_svg, wide_end.svg_width(), wide_end.chip_height());
        assert_ne!(
            line_with(&narrow_svg, r#"class="dip-pin-leg""#),
            line_with(&wide_svg, r#"class="dip-pin-leg""#)
        );
    }

    #[test]
    fn leg_start_width_controls_leg_width_after_shoulder_without_changing_package_width() {
        let narrow_start = ChipGeometry {
            pin_length: 60,
            leg_start_width: 8,
            leg_end_width: 4,
            pin_shoulder_width: 28,
            ..ChipGeometry::default()
        };
        let wide_start = ChipGeometry {
            pin_length: 60,
            leg_start_width: 18,
            leg_end_width: 4,
            pin_shoulder_width: 28,
            ..ChipGeometry::default()
        };
        let narrow_svg = generate_dip_svg("PAL", None, &narrow_start, false);
        let wide_svg = generate_dip_svg("PAL", None, &wide_start, false);

        assert_svg_size(&narrow_svg, narrow_start.svg_width(), narrow_start.chip_height());
        assert_svg_size(&wide_svg, wide_start.svg_width(), wide_start.chip_height());
        assert_ne!(
            line_with(&narrow_svg, r#"class="dip-pin-leg""#),
            line_with(&wide_svg, r#"class="dip-pin-leg""#)
        );
    }

    #[test]
    fn low_level_renderer_does_not_emit_translucent_normal_mode_strokes() {
        let svg = generate_dip_svg("PAL", None, &ChipGeometry::default(), false);

        assert!(svg.contains(r#"fill="url(#chipGradient)""#));
        assert!(svg.contains(r#"stroke="none""#));
        assert!(!svg.contains("stroke-opacity"));
        assert!(!svg.contains("border-glass"));
        assert!(!svg.contains("rgba("));
    }

    #[test]
    fn pin_one_indicator_renders_as_shaded_ring() {
        let svg = generate_dip_svg("PAL", None, &ChipGeometry::default(), false);

        assert!(!svg.contains("<circle"));
        assert!(svg.contains(r#"fill="url(#chipNotchInset)" fill-rule="evenodd" stroke="none""#));
    }

    #[test]
    fn pin_one_indicator_aligns_with_first_pin_row() {
        let geometry = ChipGeometry {
            pin_pitch: 58,
            pin_inset: 17,
            ..ChipGeometry::default()
        };
        let svg = generate_dip_svg("PAL", None, &geometry, false);
        let indicator_path = line_with(&svg, r#"fill-rule="evenodd""#);

        assert!(indicator_path.contains(&format!(",{} A5", geometry.pin_center_y(0))));
    }

    #[test]
    fn low_level_renderer_uses_filled_bevel_facets() {
        let svg = generate_dip_svg("PAL", None, &ChipGeometry::default(), false);

        assert!(svg.contains(r#"id="chipBodyClip""#));
        assert!(svg.contains(r#"fill="url(#chipBevelHighlight)""#));
        assert!(svg.contains(r#"fill="url(#chipBevelShadow)""#));
        assert_eq!(svg.matches(r#"fill="url(#chipBevelHighlight)""#).count(), 1);
        assert_eq!(svg.matches(r#"fill="url(#chipBevelShadow)""#).count(), 1);
        assert!(!svg.contains("chipTopBevel"));
        assert!(!svg.contains("chipBottomBevel"));
    }

    #[test]
    fn bevel_curves_round_the_inner_turns() {
        let svg = generate_dip_svg("PAL", None, &ChipGeometry::default(), false);
        let highlight = line_with(&svg, r#"fill="url(#chipBevelHighlight)""#);
        let shadow = line_with(&svg, r#"fill="url(#chipBevelShadow)""#);

        assert!(highlight.contains(" Q"));
        assert!(shadow.contains(" Q"));
    }

    #[test]
    fn chip_diagram_builder_uses_shade_angle_for_bevel_gradients() {
        let svg = ChipDiagram::new("PAL", ChipGeometry::default())
            .with_options(ChipDiagramOptions {
                geometry: ChipGeometry::default(),
                style: ChipDiagramStyle::default().with_shade_angle(0.0),
            })
            .render()
            .expect("builder should render");

        assert!(svg.contains(r#"id="chipBevelHighlight" x1="0.00%" x2="100.00%" y1="50.00%" y2="50.00%""#));
        assert!(svg.contains(r#"id="chipBevelShadow" x1="100.00%" x2="0.00%" y1="50.00%" y2="50.00%""#));
    }

    #[test]
    fn chip_diagram_builder_renders_notch_left_orientation() {
        let diagram = ChipDiagram::new("PAL", ChipGeometry::default()).with_options(ChipDiagramOptions {
            geometry: ChipGeometry::default(),
            style: ChipDiagramStyle::default().with_orientation(ChipOrientation::NotchLeft),
        });
        let (width, height) = diagram.document_size().expect("diagram dimensions should be valid");
        let transform = ChipOrientation::NotchLeft.upright_transform_string(height, width);
        let svg = diagram.render().expect("builder should render");

        assert_svg_size(&svg, width, height);
        assert!(svg.contains(&format!(r#"transform="{transform}""#)));
    }

    #[test]
    fn oriented_labels_follow_chip_by_default() {
        let svg = ChipDiagram::new("PAL", ChipGeometry::default())
            .with_options(ChipDiagramOptions {
                geometry: ChipGeometry::default(),
                style: ChipDiagramStyle::default().with_orientation(ChipOrientation::NotchLeft),
            })
            .render()
            .expect("builder should render");

        let geometry = ChipGeometry::default();
        let transform =
            ChipOrientation::NotchLeft.upright_transform_string(geometry.svg_width(), geometry.chip_height());

        assert!(svg.contains(&format!(r#"transform="{transform}""#)));
        assert!(!text_element_start_tag(&svg, "PAL").contains(r#"transform="rotate("#));
        assert!(!text_element_start_tag(&svg, "1").contains(r#"transform="rotate("#));
        assert!(!text_element_start_tag(&svg, "20").contains(r#"transform="rotate("#));
    }

    #[test]
    fn keep_labels_upright_counter_rotates_text_for_oriented_chips() {
        for (orientation, expected_transform) in [
            (ChipOrientation::NotchLeft, r#"transform="rotate(90 "#),
            (ChipOrientation::NotchRight, r#"transform="rotate(-90 "#),
            (ChipOrientation::NotchDown, r#"transform="rotate(180 "#),
        ] {
            let svg = ChipDiagram::new("PAL", ChipGeometry::default())
                .with_options(ChipDiagramOptions {
                    geometry: ChipGeometry::default(),
                    style: ChipDiagramStyle::default()
                        .with_orientation(orientation)
                        .with_keep_labels_upright(true),
                })
                .render()
                .expect("builder should render");

            assert!(text_element_start_tag(&svg, "PAL").contains(expected_transform));
            assert!(text_element_start_tag(&svg, "1").contains(expected_transform));
            assert!(text_element_start_tag(&svg, "20").contains(expected_transform));
        }
    }

    #[test]
    fn keep_labels_upright_centers_pin_number_anchors() {
        let svg = ChipDiagram::new("PAL", ChipGeometry::default())
            .with_options(ChipDiagramOptions {
                geometry: ChipGeometry::default(),
                style: ChipDiagramStyle::default()
                    .with_orientation(ChipOrientation::NotchLeft)
                    .with_keep_labels_upright(true),
            })
            .render()
            .expect("builder should render");

        let pin_one = text_element_start_tag(&svg, "1");
        let pin_twenty = text_element_start_tag(&svg, "20");

        assert!(pin_one.contains(r#"text-anchor="middle""#));
        assert!(pin_one.contains(r#"transform="rotate(90 "#));
        assert!(pin_twenty.contains(r#"text-anchor="middle""#));
        assert!(pin_twenty.contains(r#"transform="rotate(90 "#));
    }

    #[test]
    fn notch_left_orientation_compensates_shade_angle() {
        let svg = ChipDiagram::new("PAL", ChipGeometry::default())
            .with_options(ChipDiagramOptions {
                geometry: ChipGeometry::default(),
                style: ChipDiagramStyle::default()
                    .with_shade_angle(0.0)
                    .with_orientation(ChipOrientation::NotchLeft),
            })
            .render()
            .expect("builder should render");

        assert!(svg.contains(r#"id="chipBevelHighlight" x1="50.00%" x2="50.00%" y1="0.00%" y2="100.00%""#));
        assert!(svg.contains(r#"id="chipBevelShadow" x1="50.00%" x2="50.00%" y1="100.00%" y2="0.00%""#));
    }

    #[test]
    fn notch_left_orientation_moves_highlight_geometry() {
        let svg = ChipDiagram::new("PAL", ChipGeometry::default())
            .with_options(ChipDiagramOptions {
                geometry: ChipGeometry::default(),
                style: ChipDiagramStyle::default().with_orientation(ChipOrientation::NotchLeft),
            })
            .render()
            .expect("builder should render");

        let highlight = line_with(&svg, r#"fill="url(#chipBevelHighlight)""#);
        let shadow = line_with(&svg, r#"fill="url(#chipBevelShadow)""#);

        assert!(highlight.contains(" Q"));
        assert!(highlight.contains(" z M"));
        assert!(svg.contains(r#"fill="url(#chipBevelHighlight)""#));
        assert!(shadow.contains(" Q"));
        assert!(svg.contains(r#"fill="url(#chipBevelShadow)""#));
    }

    #[test]
    fn notch_left_orientation_keeps_body_gradient_vertical_on_screen() {
        let svg = ChipDiagram::new("PAL", ChipGeometry::default())
            .with_options(ChipDiagramOptions {
                geometry: ChipGeometry::default(),
                style: ChipDiagramStyle::default().with_orientation(ChipOrientation::NotchLeft),
            })
            .render()
            .expect("builder should render");

        assert!(svg.contains(r#"id="chipGradient" x1="100%" x2="0%" y1="0%" y2="0%""#));
        let geometry = ChipGeometry::default();
        let transform =
            ChipOrientation::NotchLeft.upright_transform_string(geometry.svg_width(), geometry.chip_height());
        assert!(svg.contains(&format!(r#"transform="{transform}""#)));
    }

    #[test]
    fn chip_diagram_builder_renders_notch_right_orientation() {
        let diagram = ChipDiagram::new("PAL", ChipGeometry::default()).with_options(ChipDiagramOptions {
            geometry: ChipGeometry::default(),
            style: ChipDiagramStyle::default().with_orientation(ChipOrientation::NotchRight),
        });
        let (width, height) = diagram.document_size().expect("diagram dimensions should be valid");
        let transform = ChipOrientation::NotchRight.upright_transform_string(height, width);
        let svg = diagram.render().expect("builder should render");

        assert_svg_size(&svg, width, height);
        assert!(svg.contains(&format!(r#"transform="{transform}""#)));
        assert!(svg.contains(r#"id="chipGradient" x1="0%" x2="100%" y1="0%" y2="0%""#));
    }

    #[test]
    fn chip_diagram_builder_renders_notch_down_orientation() {
        let diagram = ChipDiagram::new("PAL", ChipGeometry::default()).with_options(ChipDiagramOptions {
            geometry: ChipGeometry::default(),
            style: ChipDiagramStyle::default().with_orientation(ChipOrientation::NotchDown),
        });
        let (width, height) = diagram.document_size().expect("diagram dimensions should be valid");
        let transform = ChipOrientation::NotchDown.upright_transform_string(width, height);
        let svg = diagram.render().expect("builder should render");

        assert_svg_size(&svg, width, height);
        assert!(svg.contains(&format!(r#"transform="{transform}""#)));
        assert!(svg.contains(r#"id="chipGradient" x1="0%" x2="0%" y1="100%" y2="0%""#));
    }

    #[test]
    fn notch_radius_can_disable_notch() {
        let svg = ChipDiagram::new("PAL", ChipGeometry::default())
            .with_options(ChipDiagramOptions {
                geometry: ChipGeometry::default().with_notch_radius(None),
                style: ChipDiagramStyle::default(),
            })
            .render()
            .expect("builder should render");

        assert!(!svg.contains("A12,12"));
        assert!(!svg.contains(r#"stroke="url(#chipBevelHighlight)""#));
        assert!(svg.contains(r#"fill="url(#chipBevelHighlight)""#));
    }

    #[test]
    fn escapes_chip_label_text() {
        let geometry = ChipGeometry::default().with_pin_count(14);
        let svg = generate_dip_svg("A&B <C>", None, &geometry, false);

        assert!(svg.contains("A&amp;B &lt;C&gt;"));
    }

    #[test]
    fn rejects_non_dip_package_from_toml() {
        let toml = r#"
          model = "SOP14"
          model_description = "Small outline test"
          class = "logic"
          pins = 14
          voltage = 5.0
          package = "SOP"

          [pinout]
          1 = { type = "Input" }
          "#;

        let error = render_toml(toml, ChipDiagramOptions::default()).expect_err("SOP should be rejected");

        assert!(matches!(error, DipSvgError::UnsupportedPackage { .. }));
    }

    fn line_with<'a>(svg: &'a str, needle: &str) -> &'a str {
        svg.lines()
            .find(|line| line.contains(needle))
            .expect("expected SVG line to be present")
    }

    fn assert_svg_size(svg: &str, width: usize, height: usize) {
        assert!(svg.contains(&format!(r#"width="{width}""#)));
        assert!(svg.contains(&format!(r#"height="{height}""#)));
        assert!(svg.contains(&format!(r#"viewBox="0 0 {width} {height}""#)));
    }

    fn lines_with<'a>(svg: &'a str, needle: &str) -> Vec<&'a str> {
        svg.lines().filter(|line| line.contains(needle)).collect()
    }

    fn has_gradient_stop(svg: &str, offset: &str, color: &str, opacity: &str) -> bool {
        svg.lines().any(|line| {
            line.contains("<stop")
                && line.contains(&format!(r#"offset="{offset}""#))
                && line.contains(&format!(r#"stop-color="{color}""#))
                && line.contains(&format!(r#"stop-opacity="{opacity}""#))
        })
    }

    fn text_element_start_tag<'a>(svg: &'a str, text: &str) -> &'a str {
        let element_text = format!(">\n{text}\n</text>");
        let text_end = svg.find(&element_text).expect("expected SVG text element");
        let text_start = svg[..text_end].rfind("<text ").expect("expected text start tag");
        &svg[text_start..text_end + 1]
    }
}
