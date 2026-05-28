use std::{
    fs,
    path::{Path, PathBuf},
};

use dipsvg_rs::ChipDiagramStyle;
use dipsvg_rs::ChipGeometry;
use dipsvg_rs::{types::ChipOrientation, ChipDiagram, ChipDiagramOptions};

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
                .with_orientation(orientation)
                .with_keep_labels_upright(true)
                .with_shade_angle(45.0),
        })
        .render()
        .expect("DIP-20 fixture should render")
}

fn assert_common_svg(svg: &str, case: OrientationCase) {
    assert!(svg.starts_with("<svg "));
    assert!(svg.contains(case.width));
    assert!(svg.contains(case.height));
    assert!(svg.contains(case.view_box));
    if let Some(transform) = case.transform {
        assert!(svg.contains(transform));
    }
    if let Some(label_transform) = case.label_transform {
        assert!(svg.contains(label_transform));
    }
    assert!(svg.contains(r#"xmlns="http://www.w3.org/2000/svg""#));
    assert!(svg.contains("PAL16L8"));
    if case.high_contrast {
        assert!(svg.contains(r##"fill="#111111""##));
        assert!(svg.contains(r##"stroke="#ffffff""##));
        assert!(svg.contains(r##"fill="#ffffff" font-family="Inter, sans-serif""##));
        assert!(svg.contains(r##"fill="#ffffff" font-family="monospace""##));
        assert!(svg.contains(r##"class="dip-pin-label" dominant-baseline="middle" fill="#ffffff""##));
    } else {
        assert!(svg.contains("var(--text-main, #f8fafc)"));
        assert!(svg.contains("var(--text-muted, #94a3b8)"));
        assert!(svg.contains("light-dark(#0f172a, #f8fafc)"));
        assert!(svg.contains("var(--chip-body, #3a3e44)"));
    }
    assert!(svg.contains("IN_1"));
    assert!(svg.contains("VCC"));
    match case.orientation {
        ChipOrientation::NotchUp => {
            assert!(svg.contains(r#"text-anchor="end" x="160" y="52""#));
            assert!(svg.contains(r#"text-anchor="start" x="380" y="52""#));
        }
        ChipOrientation::NotchDown => {
            assert!(svg.contains(r#"text-anchor="start" x="380" y="448""#));
            assert!(svg.contains(r#"text-anchor="end" x="160" y="448""#));
        }
        ChipOrientation::NotchLeft => {
            assert!(svg.contains(r#"transform="rotate(90 52 380)" x="52" y="380""#));
            assert!(svg.contains(r#"transform="rotate(-90 52 160)" x="52" y="160""#));
        }
        ChipOrientation::NotchRight => {
            assert!(svg.contains(r#"transform="rotate(-90 448 160)" x="448" y="160""#));
            assert!(svg.contains(r#"transform="rotate(90 448 380)" x="448" y="380""#));
        }
    }
}

#[derive(Clone, Copy)]
struct OrientationCase {
    orientation: ChipOrientation,
    high_contrast: bool,
    file_name: &'static str,
    width: &'static str,
    height: &'static str,
    view_box: &'static str,
    transform: Option<&'static str>,
    label_transform: Option<&'static str>,
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
            width: r#"width="540""#,
            height: r#"height="500""#,
            view_box: r#"viewBox="0 0 540 500""#,
            transform: None,
            label_transform: None,
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
            width: r#"width="500""#,
            height: r#"height="540""#,
            view_box: r#"viewBox="0 0 500 540""#,
            transform: Some(r#"transform="translate(0 540) rotate(-90)""#),
            label_transform: Some(r#"transform="rotate(90 270 250)""#),
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
            width: r#"width="500""#,
            height: r#"height="540""#,
            view_box: r#"viewBox="0 0 500 540""#,
            transform: Some(r#"transform="translate(500 0) rotate(90)""#),
            label_transform: Some(r#"transform="rotate(-90 270 250)""#),
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
            width: r#"width="540""#,
            height: r#"height="500""#,
            view_box: r#"viewBox="0 0 540 500""#,
            transform: Some(r#"transform="translate(540 500) rotate(180)""#),
            label_transform: Some(r#"transform="rotate(180 270 250)""#),
        },
    ]
}

fn output_dir() -> PathBuf {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests").join("output");
    fs::create_dir_all(&dir).expect("test output directory should be created");
    dir
}
