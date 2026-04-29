mod bevel;
mod definitions;
mod notch;

use svg::{
    node::element::{path::Data, ClipPath, Group, Path as SvgPath, Rectangle, Text},
    Document,
};

use super::{ChipDiagram, ChipMetrics};

pub struct ChipRenderer<'a> {
    diagram: &'a ChipDiagram,
    metrics: ChipMetrics,
}

impl<'a> ChipRenderer<'a> {
    pub fn new(diagram: &'a ChipDiagram) -> Self {
        Self {
            diagram,
            metrics: ChipMetrics::new(diagram),
        }
    }

    pub fn render(&self) -> String {
        let diagram = self.diagram;
        let metrics = self.metrics;
        let geometry = &diagram.geometry;
        let pin_count = diagram.pin_count;
        let style = &diagram.style;
        let pins_per_side = metrics.pins_per_side;
        let svg_width = metrics.svg_width;
        let chip_height = metrics.chip_height;
        let document_width = metrics.document_width;
        let document_height = metrics.document_height;
        let view_box_width = metrics.view_box_width;
        let view_box_height = metrics.view_box_height;
        let chip_body_height = metrics.chip_body_height;
        let chip_left = metrics.chip_left;
        let chip_top = metrics.chip_top;
        let chip_right = metrics.chip_right;
        let chip_bottom = metrics.chip_bottom;
        let cx = metrics.cx;
        let notch_radius = metrics.notch_radius;
        let mut document = Document::new()
            .set("width", document_width)
            .set("height", document_height)
            .set("viewBox", (0, 0, view_box_width, view_box_height));
        let mut content = Group::new();

        let pin_one_indicator_fill = if style.high_contrast {
            "#ffffff"
        } else {
            "url(#chipNotchInset)"
        };
        let label_fill = if style.high_contrast {
            "#ffffff"
        } else {
            "var(--text-main, #f8fafc)"
        };
        let pin_label_fill = if style.high_contrast {
            "#ffffff"
        } else {
            "var(--text-muted, #94a3b8)"
        };
        let pin_color = if style.high_contrast {
            "#ffffff"
        } else {
            "url(#pinGradient)"
        };

        document = document.add(ChipRenderer::definitions(
            style.effective_shade_angle(),
            style.orientation,
        ));

        for i in 0..pins_per_side {
            let pin_y = geometry.pin_center_y(i) - geometry.pin_stub_height / 2;
            content = content.add(
                Rectangle::new()
                    .set("x", 0)
                    .set("y", pin_y)
                    .set("width", geometry.pin_stub_width + 5)
                    .set("height", geometry.pin_stub_height)
                    .set("fill", pin_color)
                    .set("rx", 2),
            );
            content = content.add(
                Rectangle::new()
                    .set("x", geometry.pin_stub_width + geometry.chip_width - 5)
                    .set("y", pin_y)
                    .set("width", geometry.pin_stub_width + 5)
                    .set("height", geometry.pin_stub_height)
                    .set("fill", pin_color)
                    .set("rx", 2),
            );
        }

        let body_fill = if style.high_contrast {
            "#111111"
        } else {
            "url(#chipGradient)"
        };
        let body = Rectangle::new()
            .set("x", chip_left)
            .set("y", chip_top)
            .set("width", geometry.chip_width)
            .set("height", chip_body_height)
            .set("fill", body_fill)
            .set("rx", geometry.chip_corner_radius);
        let body = if style.high_contrast {
            body.set("stroke", "#ffffff").set("stroke-width", 2.5)
        } else {
            body.set("stroke", "none")
        };
        content = content.add(body);

        if style.high_contrast {
            // High contrast intentionally keeps a flat body for maximum edge clarity.
        }
        let bevel_left = chip_left + geometry.bevel_inset;
        let bevel_top = chip_top + geometry.bevel_inset;
        let bevel_right = chip_right - geometry.bevel_inset;
        let bevel_bottom = chip_bottom - geometry.bevel_inset;
        if !style.high_contrast {
            let bevel = geometry.bevel_inset;
            let clip_path = ClipPath::new().set("id", "chipBodyClip").add(
                Rectangle::new()
                    .set("x", chip_left)
                    .set("y", chip_top)
                    .set("width", geometry.chip_width)
                    .set("height", chip_body_height)
                    .set("rx", geometry.chip_corner_radius),
            );

            let (highlight_curve, shadow_curve) = ChipRenderer::bevel_curves_for_shade(
                style.effective_shade_angle(),
                chip_left,
                chip_top,
                chip_right,
                chip_bottom,
                bevel_left + bevel,
                bevel_top + bevel,
                bevel_right - bevel,
                bevel_bottom - bevel,
                geometry.chip_corner_radius,
                notch_radius,
            );

            let mut bevel_group = Group::new()
                .set("clip-path", "url(#chipBodyClip)")
                .add(highlight_curve)
                .add(shadow_curve);

            if let Some(notch_radius) = notch_radius {
                let notch_cutout_mask =
                    ChipRenderer::notch_cutout_mask(svg_width, chip_height, cx as f32, chip_top as f32, notch_radius);
                bevel_group = bevel_group.set("mask", "url(#chipNotchCutoutMask)");
                content = content.add(notch_cutout_mask);
            }

            content = content.add(clip_path).add(bevel_group);
        }

        if let Some(notch_radius) = notch_radius {
            if style.high_contrast {
                content = content.add(
                    SvgPath::new()
                        .set(
                            "d",
                            ChipRenderer::high_contrast_notch_data(cx as f32, chip_top as f32, notch_radius),
                        )
                        .set("fill", "none")
                        .set("stroke", "#ffffff")
                        .set("stroke-width", ChipRenderer::HIGH_CONTRAST_NOTCH_STROKE_WIDTH)
                        .set("stroke-linecap", "round"),
                );
            } else {
                let notch_highlight_data = ChipRenderer::notch_vector_data(cx as f32, chip_top as f32, notch_radius);
                let clipped_notch = Group::new().set("clip-path", "url(#chipBodyClip)").add(
                    SvgPath::new()
                        .set("d", notch_highlight_data)
                        .set("fill", "url(#chipNotchInset)")
                        .set("stroke", "none"),
                );
                content = content.add(clipped_notch);
            }
        }

        let pin_one_indicator_x = (chip_left + 36) as f32;
        let pin_one_indicator_y = geometry.pin_center_y(0) as f32;
        content = content.add(
            SvgPath::new()
                .set(
                    "d",
                    ChipRenderer::pin_one_indicator_ring_data(pin_one_indicator_x, pin_one_indicator_y),
                )
                .set("fill", pin_one_indicator_fill)
                .set("fill-rule", "evenodd")
                .set("stroke", "none"),
        );

        let display_label = diagram.alias.as_deref().unwrap_or(&diagram.name);
        let max_text_width = (geometry.chip_width as f64) - 50.0;
        let approx_char_width = 9.5;
        let expected_width = (display_label.len() as f64) * approx_char_width;

        let font_size = if expected_width > max_text_width {
            (16.0 * (max_text_width / expected_width)).max(8.0)
        } else {
            16.0
        };

        // Add the chip label text, centered on the chip body.
        let chip_label_y = chip_height / 2;
        let chip_label = ChipRenderer::label_text(
            Text::new(display_label)
                .set("x", cx)
                .set("y", chip_label_y)
                .set("fill", label_fill)
                .set("font-size", format!("{font_size:.1}"))
                .set("font-weight", 700)
                .set("font-family", "Inter, sans-serif")
                .set("text-anchor", "middle")
                .set("dominant-baseline", "middle")
                .set("opacity", 0.6),
            style,
            cx,
            chip_label_y,
        );
        content = content.add(chip_label);

        for i in 0..pins_per_side {
            let y = geometry.pin_center_y(i);
            let pin_label_inset = if style.keep_labels_upright { 16 } else { 10 };
            let left_pin_number_x = geometry.pin_stub_width + pin_label_inset;
            let right_pin_number_x = geometry.pin_stub_width + geometry.chip_width - pin_label_inset;
            let upright_pin_anchor = if style.keep_labels_upright {
                Some("middle")
            } else {
                None
            };
            let mut left_pin_label = Text::new((i + 1).to_string())
                .set("x", left_pin_number_x)
                .set("y", y)
                .set("fill", pin_label_fill)
                .set("font-size", 12)
                .set("font-weight", 600)
                .set("font-family", "monospace")
                .set("dominant-baseline", "middle");
            if let Some(anchor) = upright_pin_anchor {
                left_pin_label = left_pin_label.set("text-anchor", anchor);
            }
            content = content.add(ChipRenderer::label_text(
                left_pin_label,
                style,
                left_pin_number_x,
                y,
            ));
            let right_pin_anchor = if style.keep_labels_upright {
                "middle"
            } else {
                "end"
            };
            content = content.add(ChipRenderer::label_text(
                Text::new((pin_count - i).to_string())
                    .set("x", right_pin_number_x)
                    .set("y", y)
                    .set("fill", pin_label_fill)
                    .set("font-size", 12)
                    .set("font-weight", 600)
                    .set("font-family", "monospace")
                    .set("text-anchor", right_pin_anchor)
                    .set("dominant-baseline", "middle"),
                style,
                right_pin_number_x,
                y,
            ));
        }

        let content = style.orientation.upright_transform(svg_width, chip_height, content);

        document = document.add(content);
        document.to_string()
    }

    fn pin_one_indicator_ring_data(cx: f32, cy: f32) -> Data {
        let outer_radius = 5.0;
        let inner_radius = 3.5;

        Data::new()
            .move_to((cx + outer_radius, cy))
            .elliptical_arc_to((outer_radius, outer_radius, 0, 1, 0, cx - outer_radius, cy))
            .elliptical_arc_to((outer_radius, outer_radius, 0, 1, 0, cx + outer_radius, cy))
            .close()
            .move_to((cx + inner_radius, cy))
            .elliptical_arc_to((inner_radius, inner_radius, 0, 1, 1, cx - inner_radius, cy))
            .elliptical_arc_to((inner_radius, inner_radius, 0, 1, 1, cx + inner_radius, cy))
            .close()
    }

    fn label_text(text: Text, style: &super::ChipDiagramStyle, x: usize, y: usize) -> Text {
        if !style.keep_labels_upright {
            return text;
        }

        let transform = style.orientation.label_upright_transform_string(x, y);
        if transform.is_empty() {
            text
        } else {
            text.set("transform", transform)
        }
    }
}
