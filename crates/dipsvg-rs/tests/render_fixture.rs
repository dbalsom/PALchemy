// dipsvg-rs: A Rust library for rendering chip definitions to SVG diagrams
// Copyright (C) 2026 Daniel Balsom
// SPDX-License-Identifier: MIT OR GPL-3.0-or-later

use std::{
    fs,
    path::{Path, PathBuf},
};

use dipsvg::ChipDiagramStyle;
use dipsvg::ChipGeometry;
use dipsvg::{types::ChipOrientation, ChipDiagram, ChipDiagramOptions};

#[test]
fn renders_dip20_fixture_to_standalone_svg_files() {
    let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    let chip_path = manifest_dir.join("tests").join("fixtures").join("PAL16L8.toml");

    for high_contrast in [false, true] {
        for case in orientation_cases(false, high_contrast) {
            let svg = render_fixture(&chip_path, case.orientation, Some(12.0), case.high_contrast);
            let output_path = output_dir().join(case.file_name);
            fs::write(&output_path, &svg).expect("standalone SVG should be written");

            let written_svg = fs::read_to_string(&output_path).expect("standalone SVG should be readable");
            assert_eq!(written_svg, svg);
            assert_common_svg(&written_svg, case);
            if !case.high_contrast {
                assert!(written_svg.contains("chipNotchInset"));
            }
        }

        for case in orientation_cases(true, high_contrast) {
            let svg = render_fixture(&chip_path, case.orientation, None, case.high_contrast);
            let output_path = output_dir().join(case.file_name);
            fs::write(&output_path, &svg).expect("no-notch standalone SVG should be written");

            let written_svg = fs::read_to_string(&output_path).expect("no-notch standalone SVG should be readable");
            assert_eq!(written_svg, svg);
            assert_common_svg(&written_svg, case);
            assert!(!written_svg.contains(r#"stroke="url(#chipNotchInset)""#));
            assert!(!written_svg.contains("A12,12"));
        }
    }
}

fn render_fixture(
    chip_path: &Path,
    orientation: ChipOrientation,
    notch_radius: Option<f32>,
    high_contrast: bool,
) -> String {
    ChipDiagram::from_toml_file(chip_path)
        .expect("DIP-20 fixture should load")
        .with_options(ChipDiagramOptions {
            geometry: ChipGeometry::default().with_notch_radius(notch_radius),
            style: ChipDiagramStyle::default()
                .with_high_contrast(high_contrast)
                .with_chip_body_drop_shadow(true)
                .with_orientation(orientation)
                .with_keep_labels_upright(true)
                .with_shade_angle(45.0),
        })
        .render()
        .expect("DIP-20 fixture should render")
}

fn assert_common_svg(svg: &str, case: OrientationCase) {
    let geometry = ChipGeometry::default();
    let package_width = geometry.labeled_svg_width();
    let chip_height = geometry.chip_height();
    let (width, height) = match case.orientation {
        ChipOrientation::NotchUp | ChipOrientation::NotchDown => (package_width, chip_height),
        ChipOrientation::NotchLeft | ChipOrientation::NotchRight => (chip_height, package_width),
    };

    assert!(svg.starts_with("<svg "));
    assert_svg_size(svg, width, height);
    let transform = case.orientation.upright_transform_string(package_width, chip_height);
    if !transform.is_empty() {
        assert!(svg.contains(&format!(r#"transform="{transform}""#)));
    }
    assert!(svg.contains(r#"xmlns="http://www.w3.org/2000/svg""#));
    assert!(svg.contains("PAL16L8"));
    let chip_label = text_element_start_tag(svg, "PAL16L8");
    match case.orientation {
        ChipOrientation::NotchUp => assert!(!chip_label.contains(r#"transform="rotate("#)),
        ChipOrientation::NotchLeft => assert!(chip_label.contains(r#"transform="rotate(90 "#)),
        ChipOrientation::NotchRight => assert!(chip_label.contains(r#"transform="rotate(-90 "#)),
        ChipOrientation::NotchDown => assert!(chip_label.contains(r#"transform="rotate(180 "#)),
    }
    if case.high_contrast {
        assert!(svg.contains(r##"fill="#111111""##));
        assert!(svg.contains(r##"stroke="#ffffff""##));
        assert!(svg.contains(r##"fill="#ffffff" font-family="Inter, sans-serif""##));
        assert!(svg.contains(r##"fill="#ffffff" font-family="monospace""##));
        assert!(svg.contains(r##"class="dip-pin-label""##));
        assert!(svg.contains(r##"dominant-baseline="central""##));
        assert!(svg.contains(r##"fill="#ffffff""##));
        assert!(!svg.contains("chipBodyDropShadowBlur"));
        assert!(!svg.contains(r#"class="dip-chip-body-shadow""#));
    } else {
        assert!(svg.contains("var(--text-main, #f8fafc)"));
        assert!(svg.contains("var(--text-muted, #94a3b8)"));
        assert!(svg.contains("light-dark(#0f172a, #f8fafc)"));
        assert!(svg.contains("var(--chip-body, #3a3e44)"));
        assert!(svg.contains("chipBodyDropShadowBlur"));
        assert!(svg.contains(r#"class="dip-chip-body-shadow""#));
    }
    assert!(svg.contains("IN_1"));
    assert!(svg.contains("VCC"));
    let input_label = text_element_start_tag(svg, "IN_1");
    let power_label = text_element_start_tag(svg, "VCC");
    match case.orientation {
        ChipOrientation::NotchUp => {
            assert!(input_label.contains(r#"text-anchor="end""#));
            assert!(!input_label.contains(r#"transform="rotate("#));
            assert!(power_label.contains(r#"text-anchor="start""#));
            assert!(!power_label.contains(r#"transform="rotate("#));
        }
        ChipOrientation::NotchDown => {
            assert!(input_label.contains(r#"text-anchor="start""#));
            assert!(!input_label.contains(r#"transform="rotate("#));
            assert!(power_label.contains(r#"text-anchor="end""#));
            assert!(!power_label.contains(r#"transform="rotate("#));
        }
        ChipOrientation::NotchLeft => {
            assert!(input_label.contains(r#"text-anchor="start""#));
            assert!(input_label.contains(r#"transform="rotate(90 "#));
            assert!(power_label.contains(r#"text-anchor="start""#));
            assert!(power_label.contains(r#"transform="rotate(-90 "#));
        }
        ChipOrientation::NotchRight => {
            assert!(input_label.contains(r#"text-anchor="start""#));
            assert!(input_label.contains(r#"transform="rotate(-90 "#));
            assert!(power_label.contains(r#"text-anchor="start""#));
            assert!(power_label.contains(r#"transform="rotate(90 "#));
        }
    }
}

#[derive(Clone, Copy)]
struct OrientationCase {
    orientation: ChipOrientation,
    high_contrast: bool,
    file_name: &'static str,
}

fn orientation_cases(no_notch: bool, high_contrast: bool) -> [OrientationCase; 4] {
    [
        OrientationCase {
            orientation: ChipOrientation::NotchUp,
            high_contrast,
            file_name: if no_notch {
                if high_contrast {
                    "PAL16L8_pin1_up_high_contrast_no_notch.svg"
                } else {
                    "PAL16L8_pin1_up_no_notch.svg"
                }
            } else {
                if high_contrast {
                    "PAL16L8_pin1_up_high_contrast.svg"
                } else {
                    "PAL16L8_pin1_up.svg"
                }
            },
        },
        OrientationCase {
            orientation: ChipOrientation::NotchLeft,
            high_contrast,
            file_name: if no_notch {
                if high_contrast {
                    "PAL16L8_pin1_left_high_contrast_no_notch.svg"
                } else {
                    "PAL16L8_pin1_left_no_notch.svg"
                }
            } else {
                if high_contrast {
                    "PAL16L8_pin1_left_high_contrast.svg"
                } else {
                    "PAL16L8_pin1_left.svg"
                }
            },
        },
        OrientationCase {
            orientation: ChipOrientation::NotchRight,
            high_contrast,
            file_name: if no_notch {
                if high_contrast {
                    "PAL16L8_pin1_right_high_contrast_no_notch.svg"
                } else {
                    "PAL16L8_pin1_right_no_notch.svg"
                }
            } else {
                if high_contrast {
                    "PAL16L8_pin1_right_high_contrast.svg"
                } else {
                    "PAL16L8_pin1_right.svg"
                }
            },
        },
        OrientationCase {
            orientation: ChipOrientation::NotchDown,
            high_contrast,
            file_name: if no_notch {
                if high_contrast {
                    "PAL16L8_pin1_down_high_contrast_no_notch.svg"
                } else {
                    "PAL16L8_pin1_down_no_notch.svg"
                }
            } else {
                if high_contrast {
                    "PAL16L8_pin1_down_high_contrast.svg"
                } else {
                    "PAL16L8_pin1_down.svg"
                }
            },
        },
    ]
}

fn assert_svg_size(svg: &str, width: usize, height: usize) {
    assert!(svg.contains(&format!(r#"width="{width}""#)));
    assert!(svg.contains(&format!(r#"height="{height}""#)));
    assert!(svg.contains(&format!(r#"viewBox="0 0 {width} {height}""#)));
}

fn text_element_start_tag<'a>(svg: &'a str, text: &str) -> &'a str {
    let element_text = format!(">\n{text}\n</text>");
    let text_end = svg.find(&element_text).expect("expected SVG text element");
    let text_start = svg[..text_end].rfind("<text ").expect("expected text start tag");
    &svg[text_start..text_end + 1]
}

fn output_dir() -> PathBuf {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests").join("output");
    fs::create_dir_all(&dir).expect("test output directory should be created");
    dir
}
