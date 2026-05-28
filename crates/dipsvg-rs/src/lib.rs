pub mod geometry;
pub mod render;
pub mod types;
pub mod util;

use std::{fs, path::Path};

use palcore::{ChipDef, PackageType, PinDef, PinType};
use thiserror::Error;

pub use geometry::{ChipGeometry, PIN_LABEL_COLUMN_WIDTH, PIN_LABEL_GAP};
use render::ChipRenderer;
use types::*;
pub use util::*;

#[derive(Debug, Clone, PartialEq)]
pub struct ChipDiagramStyle {
    pub high_contrast: bool,
    pub shade_angle: f32,
    pub keep_labels_upright: bool,
    pub pin_label_light_color: String,
    pub pin_label_dark_color: String,

    pub orientation: ChipOrientation,
}

impl Default for ChipDiagramStyle {
    fn default() -> Self {
        Self {
            high_contrast: false,
            shade_angle: 45.0,
            keep_labels_upright: false,
            pin_label_light_color: "#0f172a".to_string(),
            pin_label_dark_color: "#f8fafc".to_string(),
            orientation: ChipOrientation::default(),
        }
    }
}

impl ChipDiagramStyle {
    pub fn with_high_contrast(mut self, high_contrast: bool) -> Self {
        self.high_contrast = high_contrast;
        self
    }

    pub fn with_shade_angle(mut self, shade_angle: f32) -> Self {
        self.shade_angle = shade_angle;
        self
    }

    pub fn with_keep_labels_upright(mut self, keep_labels_upright: bool) -> Self {
        self.keep_labels_upright = keep_labels_upright;
        self
    }

    pub fn with_pin_label_light_color(mut self, color: impl Into<String>) -> Self {
        self.pin_label_light_color = color.into();
        self
    }

    pub fn with_pin_label_dark_color(mut self, color: impl Into<String>) -> Self {
        self.pin_label_dark_color = color.into();
        self
    }

    pub fn with_pin_label_theme_colors(
        mut self,
        light_color: impl Into<String>,
        dark_color: impl Into<String>,
    ) -> Self {
        self.pin_label_light_color = light_color.into();
        self.pin_label_dark_color = dark_color.into();
        self
    }

    pub fn effective_shade_angle(&self) -> f32 {
        match self.orientation {
            ChipOrientation::NotchUp => self.shade_angle,
            ChipOrientation::NotchLeft => self.shade_angle + 90.0,
            ChipOrientation::NotchRight => self.shade_angle - 90.0,
            ChipOrientation::NotchDown => self.shade_angle + 180.0,
        }
    }

    pub fn with_orientation(mut self, orientation: ChipOrientation) -> Self {
        self.orientation = orientation;
        self
    }
}

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

#[deprecated(note = "use ChipDiagramOptions instead")]
pub type DipSvgOptions = ChipDiagramOptions;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PinLabel {
    pub text: String,
    pub active_low: bool,
}

impl PinLabel {
    pub fn new(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            active_low: false,
        }
    }

    pub fn active_low(mut self, active_low: bool) -> Self {
        self.active_low = active_low;
        self
    }

    fn from_pin_def(pin: usize, definition: &PinDef) -> Self {
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

#[derive(Debug, Clone)]
pub struct ChipDiagram {
    name: String,
    alias: Option<String>,
    pin_count: usize,
    pin_labels: Vec<Option<PinLabel>>,
    chip_width: Option<usize>,
    geometry: ChipGeometry,
    style: ChipDiagramStyle,
}

impl ChipDiagram {
    pub fn new(name: impl Into<String>, pin_count: usize) -> Self {
        Self {
            name: name.into(),
            alias: None,
            pin_count,
            pin_labels: vec![None; pin_count],
            chip_width: None,
            geometry: ChipGeometry::default(),
            style: ChipDiagramStyle::default(),
        }
    }

    pub fn from_chip(chip: &ChipDef) -> Result<Self, DipSvgError> {
        if chip.package != PackageType::DIP {
            return Err(DipSvgError::UnsupportedPackage { package: chip.package });
        }

        let mut diagram = Self::new(chip.display_name(), chip.pins)
            .with_alias_option(chip.alias.clone())
            .with_chip_width_option(chip.width)
            .with_geometry(ChipGeometry::from_chip(chip));
        for pin in 1..=chip.pins {
            if let Some(definition) = chip.pinout.get(&pin.to_string()) {
                diagram = diagram.with_pin_label(pin, PinLabel::from_pin_def(pin, definition));
            }
        }

        Ok(diagram)
    }

    pub fn from_toml(input: &str) -> Result<Self, DipSvgError> {
        let chip = chip_from_toml(input)?;
        Self::from_chip(&chip)
    }

    pub fn from_toml_file<P: AsRef<Path>>(path: P) -> Result<Self, DipSvgError> {
        let input = fs::read_to_string(path)?;
        Self::from_toml(&input)
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
        self.pin_count = pin_count;
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
        if (1..=self.pin_count).contains(&pin) {
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
            if (1..=self.pin_count).contains(&pin) {
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
        self.geometry = options.geometry;
        if let Some(chip_width) = self.chip_width {
            self.geometry = self.geometry.with_chip_width(chip_width);
        }
        self.style = options.style;
        self
    }

    pub fn with_geometry(mut self, geometry: ChipGeometry) -> Self {
        self.geometry = geometry;
        if let Some(chip_width) = self.chip_width {
            self.geometry = self.geometry.with_chip_width(chip_width);
        }
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
        self.pin_count
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
        validate_pin_count(self.pin_count)?;
        let metrics = ChipMetrics::new(self);
        Ok((metrics.document_width, metrics.document_height))
    }

    pub fn render(&self) -> Result<String, DipSvgError> {
        validate_pin_count(self.pin_count)?;
        Ok(ChipRenderer::new(self).render())
    }
}

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

pub fn chip_from_toml(input: &str) -> Result<ChipDef, DipSvgError> {
    let mut chip = toml::from_str::<ChipDef>(input)?;
    chip.normalize_name();
    Ok(chip)
}

pub fn render_toml(input: &str, options: ChipDiagramOptions) -> Result<String, DipSvgError> {
    ChipDiagram::from_toml(input)?.with_options(options).render()
}

pub fn render_toml_file<P: AsRef<Path>>(path: P, options: ChipDiagramOptions) -> Result<String, DipSvgError> {
    ChipDiagram::from_toml_file(path)?.with_options(options).render()
}

pub fn render_chip(chip: &ChipDef, options: ChipDiagramOptions) -> Result<String, DipSvgError> {
    ChipDiagram::from_chip(chip)?.with_options(options).render()
}

pub fn generate_dip_svg(
    name: &str,
    alias: Option<&str>,
    pin_count: usize,
    geometry: &ChipGeometry,
    high_contrast: bool,
) -> String {
    let style = ChipDiagramStyle::default()
        .with_high_contrast(high_contrast)
        .with_orientation(ChipOrientation::NotchUp);

    ChipDiagram::new(name, pin_count)
        .with_alias_option(alias.map(ToOwned::to_owned))
        .with_geometry(*geometry)
        .with_style(style)
        .render()
        .expect("generate_dip_svg requires a positive even DIP pin count")
}

#[derive(Debug, Clone, Copy)]
struct ChipMetrics {
    pins_per_side: usize,
    diagram_width: usize,
    chip_height: usize,
    document_width: usize,
    document_height: usize,
    view_box_width: usize,
    view_box_height: usize,
    has_pin_labels: bool,
    chip_origin_x: usize,
    left_pin_label_x: usize,
    right_pin_label_x: usize,
    chip_body_height: usize,
    chip_left: usize,
    chip_top: usize,
    chip_right: usize,
    chip_bottom: usize,
    cx: usize,
    notch_radius: Option<f32>,
}

impl ChipMetrics {
    fn new(diagram: &ChipDiagram) -> Self {
        let geometry = &diagram.geometry;
        let pins_per_side = diagram.pin_count / 2;
        let has_pin_labels = diagram.has_pin_labels();
        let package_width = geometry.svg_width();
        let diagram_width = if has_pin_labels {
            geometry.labeled_svg_width()
        } else {
            package_width
        };
        let chip_height = geometry.chip_height(pins_per_side);
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
        let chip_body_height = chip_height - geometry.top_inset - geometry.bottom_inset;
        let chip_left = chip_origin_x + geometry.pin_stub_width;
        let chip_top = geometry.top_inset;
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

fn validate_pin_count(pin_count: usize) -> Result<(), DipSvgError> {
    if pin_count == 0 || pin_count % 2 != 0 {
        Err(DipSvgError::InvalidPinCount(pin_count))
    } else {
        Ok(())
    }
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
    fn renders_existing_chip_toml_to_svg() {
        let svg = render_toml(PAL16L8, ChipDiagramOptions::default()).expect("chip should render");

        assert!(svg.starts_with("<svg "));
        assert!(svg.contains(r#"width="540""#));
        assert!(svg.contains(r#"height="500""#));
        assert!(svg.contains(r#"viewBox="0 0 540 500""#));
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

        let svg = render_toml(toml, ChipDiagramOptions::default()).expect("chip should render");
        assert!(svg.contains(r#"width="610""#));
        assert!(svg.contains(r#"viewBox="0 0 610 588""#));
        assert!(svg.contains(r#"width="220" x="195""#));
    }

    #[test]
    fn renders_8253_with_custom_chip_width() {
        let diagram = ChipDiagram::from_toml(INTEL_8253).expect("8253 TOML should parse");
        assert_eq!(diagram.geometry().chip_width, 220);

        let svg = render_toml(INTEL_8253, ChipDiagramOptions::default()).expect("8253 should render");
        assert!(svg.contains(r#"width="610""#));
        assert!(svg.contains(r#"width="220" x="195""#));
        assert!(svg.contains("Intel 8253"));
        assert!(svg.contains("GATE0"));
        assert!(svg.contains("CS"));
        assert!(svg.contains(r#"text-decoration="overline""#));
    }

    #[test]
    fn chip_diagram_builder_can_render_custom_pin_labels() {
        let svg = ChipDiagram::new("CUSTOM", 14)
            .with_pin_label(1, "A0")
            .with_pin_label(14, "VCC")
            .with_options(ChipDiagramOptions {
                geometry: ChipGeometry::default(),
                style: ChipDiagramStyle::default().with_pin_label_theme_colors("#111111", "#eeeeee"),
            })
            .render()
            .expect("builder should render");

        assert!(svg.contains(r#"width="540""#));
        assert!(svg.contains(r#"fill="light-dark(#111111, #eeeeee)""#));
        assert!(svg.contains("A0"));
        assert!(svg.contains("VCC"));
    }

    #[test]
    fn chip_diagram_builder_renders_low_level_diagram() {
        let svg = ChipDiagram::new("PAL16L8", 20)
            .with_alias("PAL")
            .with_options(ChipDiagramOptions {
                geometry: ChipGeometry::default(),
                style: ChipDiagramStyle::default(),
            })
            .render()
            .expect("builder should render");

        assert!(svg.contains("PAL"));
        assert!(svg.contains(r#"width="180""#));
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
        let error = ChipDiagram::new("bad", 19)
            .render()
            .expect_err("odd pin count should fail");

        assert!(matches!(error, DipSvgError::InvalidPinCount(19)));
    }

    #[test]
    fn low_level_renderer_preserves_high_contrast_style() {
        let svg = generate_dip_svg("PAL", None, 20, &ChipGeometry::default(), true);

        assert!(svg.contains(r##"fill="#111111""##));
        assert!(svg.contains(r##"stroke="#ffffff""##));
        assert!(svg.contains(r##"<rect fill="#ffffff""##));
        assert!(svg.contains(r##"rx="2""##));
    }

    #[test]
    fn high_contrast_mode_renders_visible_notch() {
        let svg = generate_dip_svg("PAL", None, 20, &ChipGeometry::default(), true);

        assert!(svg.contains(r##"fill="none" stroke="#ffffff" stroke-linecap="round" stroke-width="2.5""##));
        assert!(svg.contains("A10.75,10.75,0,0,0,100.75,31.25"));
    }

    #[test]
    fn low_level_renderer_uses_gradient_for_pin_legs() {
        let svg = generate_dip_svg("PAL", None, 20, &ChipGeometry::default(), false);

        assert!(svg.contains(r#"id="pinGradient""#));
        assert!(svg.contains(r#"<rect fill="url(#pinGradient)""#));
        assert!(svg.contains(r#"rx="2""#));
    }

    #[test]
    fn low_level_renderer_does_not_emit_translucent_normal_mode_strokes() {
        let svg = generate_dip_svg("PAL", None, 20, &ChipGeometry::default(), false);

        assert!(svg.contains(r#"fill="url(#chipGradient)""#));
        assert!(svg.contains(r#"stroke="none""#));
        assert!(!svg.contains("stroke-opacity"));
        assert!(!svg.contains("border-glass"));
        assert!(!svg.contains("rgba("));
    }

    #[test]
    fn pin_one_indicator_renders_as_shaded_ring() {
        let svg = generate_dip_svg("PAL", None, 20, &ChipGeometry::default(), false);

        assert!(!svg.contains("<circle"));
        assert!(svg.contains(r#"fill="url(#chipNotchInset)" fill-rule="evenodd" stroke="none""#));
    }

    #[test]
    fn pin_one_indicator_aligns_with_first_pin_row() {
        let svg = generate_dip_svg("PAL", None, 20, &ChipGeometry::default(), false);

        assert!(svg.contains(r#"d="M56,52 A5,5,0,1,0,46,52 A5,5,0,1,0,56,52 z"#));
    }

    #[test]
    fn low_level_renderer_uses_filled_bevel_facets() {
        let svg = generate_dip_svg("PAL", None, 20, &ChipGeometry::default(), false);

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
        let svg = generate_dip_svg("PAL", None, 20, &ChipGeometry::default(), false);

        assert!(svg.contains("L27,36 Q21,36,21,42"));
        assert!(svg.contains("L153,464 Q159,464,159,458"));
    }

    #[test]
    fn chip_diagram_builder_uses_shade_angle_for_bevel_gradients() {
        let svg = ChipDiagram::new("PAL", 20)
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
        let svg = ChipDiagram::new("PAL", 20)
            .with_options(ChipDiagramOptions {
                geometry: ChipGeometry::default(),
                style: ChipDiagramStyle::default().with_orientation(ChipOrientation::NotchLeft),
            })
            .render()
            .expect("builder should render");

        assert!(svg.contains(r#"width="500""#));
        assert!(svg.contains(r#"height="180""#));
        assert!(svg.contains(r#"viewBox="0 0 500 180""#));
        assert!(svg.contains(r#"transform="translate(0 180) rotate(-90)""#));
    }

    #[test]
    fn oriented_labels_follow_chip_by_default() {
        let svg = ChipDiagram::new("PAL", 20)
            .with_options(ChipDiagramOptions {
                geometry: ChipGeometry::default(),
                style: ChipDiagramStyle::default().with_orientation(ChipOrientation::NotchLeft),
            })
            .render()
            .expect("builder should render");

        assert!(svg.contains(r#"transform="translate(0 180) rotate(-90)""#));
        assert!(!svg.contains(r#"transform="rotate(90 90 250)""#));
        assert!(!svg.contains(r#"transform="rotate(90 25 52)""#));
        assert!(!svg.contains(r#"transform="rotate(90 155 52)""#));
    }

    #[test]
    fn keep_labels_upright_counter_rotates_text_for_oriented_chips() {
        for (orientation, label_transform, left_pin_transform, right_pin_transform) in [
            (
                ChipOrientation::NotchLeft,
                r#"transform="rotate(90 90 250)""#,
                r#"transform="rotate(90 31 52)""#,
                r#"transform="rotate(90 149 52)""#,
            ),
            (
                ChipOrientation::NotchRight,
                r#"transform="rotate(-90 90 250)""#,
                r#"transform="rotate(-90 31 52)""#,
                r#"transform="rotate(-90 149 52)""#,
            ),
            (
                ChipOrientation::NotchDown,
                r#"transform="rotate(180 90 250)""#,
                r#"transform="rotate(180 31 52)""#,
                r#"transform="rotate(180 149 52)""#,
            ),
        ] {
            let svg = ChipDiagram::new("PAL", 20)
                .with_options(ChipDiagramOptions {
                    geometry: ChipGeometry::default(),
                    style: ChipDiagramStyle::default()
                        .with_orientation(orientation)
                        .with_keep_labels_upright(true),
                })
                .render()
                .expect("builder should render");

            assert!(svg.contains(label_transform));
            assert!(svg.contains(left_pin_transform));
            assert!(svg.contains(right_pin_transform));
        }
    }

    #[test]
    fn keep_labels_upright_centers_pin_number_anchors() {
        let svg = ChipDiagram::new("PAL", 20)
            .with_options(ChipDiagramOptions {
                geometry: ChipGeometry::default(),
                style: ChipDiagramStyle::default()
                    .with_orientation(ChipOrientation::NotchLeft)
                    .with_keep_labels_upright(true),
            })
            .render()
            .expect("builder should render");

        assert!(svg.contains(
            r#"<text dominant-baseline="middle" fill="var(--text-muted, #94a3b8)" font-family="monospace" font-size="12" font-weight="600" text-anchor="middle" transform="rotate(90 31 52)" x="31" y="52">"#
        ));
        assert!(svg.contains(
            r#"<text dominant-baseline="middle" fill="var(--text-muted, #94a3b8)" font-family="monospace" font-size="12" font-weight="600" text-anchor="middle" transform="rotate(90 149 52)" x="149" y="52">"#
        ));
    }

    #[test]
    fn notch_left_orientation_compensates_shade_angle() {
        let svg = ChipDiagram::new("PAL", 20)
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
        let svg = ChipDiagram::new("PAL", 20)
            .with_options(ChipDiagramOptions {
                geometry: ChipGeometry::default(),
                style: ChipDiagramStyle::default().with_orientation(ChipOrientation::NotchLeft),
            })
            .render()
            .expect("builder should render");

        assert!(svg.contains(r#"d="M15,30 L80,30 L80,36 L21,36 z M100,30 L159,30 Q165,30,165,36"#));
        assert!(svg.contains(r#"fill="url(#chipBevelHighlight)""#));
        assert!(svg.contains(r#"d="M165,470 L21,470 Q15,470,15,464"#));
        assert!(svg.contains(r#"fill="url(#chipBevelShadow)""#));
    }

    #[test]
    fn notch_left_orientation_keeps_body_gradient_vertical_on_screen() {
        let svg = ChipDiagram::new("PAL", 20)
            .with_options(ChipDiagramOptions {
                geometry: ChipGeometry::default(),
                style: ChipDiagramStyle::default().with_orientation(ChipOrientation::NotchLeft),
            })
            .render()
            .expect("builder should render");

        assert!(svg.contains(r#"id="chipGradient" x1="100%" x2="0%" y1="0%" y2="0%""#));
        assert!(svg.contains(r#"transform="translate(0 180) rotate(-90)""#));
    }

    #[test]
    fn chip_diagram_builder_renders_notch_right_orientation() {
        let svg = ChipDiagram::new("PAL", 20)
            .with_options(ChipDiagramOptions {
                geometry: ChipGeometry::default(),
                style: ChipDiagramStyle::default().with_orientation(ChipOrientation::NotchRight),
            })
            .render()
            .expect("builder should render");

        assert!(svg.contains(r#"width="500""#));
        assert!(svg.contains(r#"height="180""#));
        assert!(svg.contains(r#"viewBox="0 0 500 180""#));
        assert!(svg.contains(r#"transform="translate(500 0) rotate(90)""#));
        assert!(svg.contains(r#"id="chipGradient" x1="0%" x2="100%" y1="0%" y2="0%""#));
    }

    #[test]
    fn chip_diagram_builder_renders_notch_down_orientation() {
        let svg = ChipDiagram::new("PAL", 20)
            .with_options(ChipDiagramOptions {
                geometry: ChipGeometry::default(),
                style: ChipDiagramStyle::default().with_orientation(ChipOrientation::NotchDown),
            })
            .render()
            .expect("builder should render");

        assert!(svg.contains(r#"width="180""#));
        assert!(svg.contains(r#"height="500""#));
        assert!(svg.contains(r#"viewBox="0 0 180 500""#));
        assert!(svg.contains(r#"transform="translate(180 500) rotate(180)""#));
        assert!(svg.contains(r#"id="chipGradient" x1="0%" x2="0%" y1="100%" y2="0%""#));
    }

    #[test]
    fn notch_radius_can_disable_notch() {
        let svg = ChipDiagram::new("PAL", 20)
            .with_options(ChipDiagramOptions {
                geometry: ChipGeometry::default().with_notch_radius(None),
                style: ChipDiagramStyle::default(),
            })
            .render()
            .expect("builder should render");

        assert!(!svg.contains("A12,12"));
        assert!(!svg.contains(r#"stroke="url(#chipBevelHighlight)""#));
        assert!(svg.contains(r#"d="M15,470 L15,36 Q15,30,21,30 L165,30 L159,36"#));
    }

    #[test]
    fn escapes_chip_label_text() {
        let svg = generate_dip_svg("A&B <C>", None, 14, &ChipGeometry::default(), false);

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
}
