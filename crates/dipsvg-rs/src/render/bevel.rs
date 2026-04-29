use super::ChipRenderer;
use crate::util;

use svg::node::element::{path::Data, Path as SvgPath};

const NOTCH_BEVEL_OVERLAP: f32 = 2.0;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum BevelCorner {
    TopLeft,
    TopRight,
    BottomRight,
    BottomLeft,
}

impl BevelCorner {
    // Return the corner opposite from this one.
    pub fn opposite(self: BevelCorner) -> BevelCorner {
        match self {
            BevelCorner::TopLeft => BevelCorner::BottomRight,
            BevelCorner::TopRight => BevelCorner::BottomLeft,
            BevelCorner::BottomRight => BevelCorner::TopLeft,
            BevelCorner::BottomLeft => BevelCorner::TopRight,
        }
    }
}

impl<'a> ChipRenderer<'a> {
    pub fn bevel_curves_for_shade(
        shade_angle: f32,
        chip_left: usize,
        chip_top: usize,
        chip_right: usize,
        chip_bottom: usize,
        inner_left: usize,
        inner_top: usize,
        inner_right: usize,
        inner_bottom: usize,
        corner_radius: usize,
        notch_radius: Option<f32>,
    ) -> (SvgPath, SvgPath) {
        let highlight_corner = ChipRenderer::highlight_corner_for_angle(shade_angle);
        let shadow_corner = highlight_corner.opposite();

        (
            ChipRenderer::bevel_curve_for_corner(
                highlight_corner,
                chip_left,
                chip_top,
                chip_right,
                chip_bottom,
                inner_left,
                inner_top,
                inner_right,
                inner_bottom,
                corner_radius,
                notch_radius,
                "url(#chipBevelHighlight)",
            ),
            ChipRenderer::bevel_curve_for_corner(
                shadow_corner,
                chip_left,
                chip_top,
                chip_right,
                chip_bottom,
                inner_left,
                inner_top,
                inner_right,
                inner_bottom,
                corner_radius,
                notch_radius,
                "url(#chipBevelShadow)",
            ),
        )
    }

    fn highlight_corner_for_angle(shade_angle: f32) -> BevelCorner {
        let radians = shade_angle.to_radians();
        let source_x = -radians.cos();
        let source_y = -radians.sin();

        match (source_x.is_sign_negative(), source_y.is_sign_negative()) {
            (true, true) => BevelCorner::TopLeft,
            (false, true) => BevelCorner::TopRight,
            (false, false) => BevelCorner::BottomRight,
            (true, false) => BevelCorner::BottomLeft,
        }
    }

    fn bevel_curve_for_corner(
        corner: BevelCorner,
        chip_left: usize,
        chip_top: usize,
        chip_right: usize,
        chip_bottom: usize,
        inner_left: usize,
        inner_top: usize,
        inner_right: usize,
        inner_bottom: usize,
        corner_radius: usize,
        notch_radius: Option<f32>,
        fill: &str,
    ) -> SvgPath {
        match corner {
            BevelCorner::TopLeft => ChipRenderer::top_left_bevel_curve(
                chip_left,
                chip_top,
                chip_right,
                chip_bottom,
                inner_left,
                inner_top,
                inner_bottom,
                corner_radius,
                notch_radius,
                fill,
            ),
            BevelCorner::TopRight => ChipRenderer::top_right_bevel_curve(
                chip_left,
                chip_top,
                chip_right,
                chip_bottom,
                inner_top,
                inner_right,
                inner_bottom,
                corner_radius,
                notch_radius,
                fill,
            ),
            BevelCorner::BottomRight => ChipRenderer::bottom_right_bevel_curve(
                chip_left,
                chip_top,
                chip_right,
                chip_bottom,
                inner_right,
                inner_top,
                inner_bottom,
                corner_radius,
                fill,
            ),
            BevelCorner::BottomLeft => ChipRenderer::bottom_left_bevel_curve(
                chip_left,
                chip_top,
                chip_right,
                chip_bottom,
                inner_left,
                inner_top,
                inner_bottom,
                corner_radius,
                fill,
            ),
        }
    }

    fn top_left_bevel_curve(
        chip_left: usize,
        chip_top: usize,
        chip_right: usize,
        chip_bottom: usize,
        inner_left: usize,
        inner_top: usize,
        inner_bottom: usize,
        corner_radius: usize,
        notch_radius: Option<f32>,
        fill: &str,
    ) -> SvgPath {
        let inner_radius = corner_radius;
        let mut data = Data::new()
            .move_to((chip_left, chip_bottom))
            .line_to((chip_left, chip_top + corner_radius))
            .quadratic_curve_to((chip_left, chip_top, chip_left + corner_radius, chip_top));

        if let Some(notch_radius) = notch_radius {
            let cx = util::midpoint(chip_left, chip_right);
            let notch_left = cx - notch_radius;
            let notch_right = cx + notch_radius;
            let notch_left_overlap = notch_left + NOTCH_BEVEL_OVERLAP;
            let notch_right_overlap = notch_right - NOTCH_BEVEL_OVERLAP;
            data = data
                .line_to((notch_left_overlap, chip_top as f32))
                .line_to((notch_left_overlap, inner_top as f32))
                .line_to((inner_left + inner_radius, inner_top))
                .quadratic_curve_to((inner_left, inner_top, inner_left, inner_top + corner_radius))
                .line_to((inner_left, inner_bottom))
                .close()
                .move_to((notch_right_overlap, chip_top as f32))
                .line_to((chip_right, chip_top))
                .line_to((chip_right - corner_radius, inner_top))
                .line_to((notch_right_overlap, inner_top as f32))
                .close();
        } else {
            data = data
                .line_to((chip_right, chip_top))
                .line_to((chip_right - corner_radius, inner_top))
                .line_to((inner_left + inner_radius, inner_top))
                .quadratic_curve_to((inner_left, inner_top, inner_left, inner_top + corner_radius))
                .line_to((inner_left, inner_bottom))
                .close();
        }

        SvgPath::new().set("d", data).set("fill", fill)
    }

    fn top_right_bevel_curve(
        chip_left: usize,
        chip_top: usize,
        chip_right: usize,
        chip_bottom: usize,
        inner_top: usize,
        inner_right: usize,
        inner_bottom: usize,
        corner_radius: usize,
        notch_radius: Option<f32>,
        fill: &str,
    ) -> SvgPath {
        let inner_radius = corner_radius;
        let mut data = Data::new().move_to((chip_left, chip_top));

        if let Some(notch_radius) = notch_radius {
            let cx = util::midpoint(chip_left, chip_right);
            let notch_left = cx - notch_radius;
            let notch_right = cx + notch_radius;
            let notch_left_overlap = notch_left + NOTCH_BEVEL_OVERLAP;
            let notch_right_overlap = notch_right - NOTCH_BEVEL_OVERLAP;
            data = data
                .line_to((notch_left_overlap, chip_top as f32))
                .line_to((notch_left_overlap, inner_top as f32))
                .line_to((chip_left + corner_radius, inner_top))
                .close()
                .move_to((notch_right_overlap, chip_top as f32))
                .line_to((chip_right - corner_radius, chip_top))
                .quadratic_curve_to((chip_right, chip_top, chip_right, chip_top + corner_radius))
                .line_to((chip_right, chip_bottom))
                .line_to((inner_right, inner_bottom))
                .line_to((inner_right, inner_top + inner_radius))
                .quadratic_curve_to((inner_right, inner_top, inner_right - inner_radius, inner_top))
                .line_to((notch_right_overlap, inner_top as f32))
                .close();
        } else {
            data = data
                .line_to((chip_right - corner_radius, chip_top))
                .quadratic_curve_to((chip_right, chip_top, chip_right, chip_top + corner_radius))
                .line_to((chip_right, chip_bottom))
                .line_to((inner_right, inner_bottom))
                .line_to((inner_right, inner_top + inner_radius))
                .quadratic_curve_to((inner_right, inner_top, inner_right - inner_radius, inner_top))
                .line_to((chip_left + corner_radius, inner_top))
                .close();
        }

        SvgPath::new().set("d", data).set("fill", fill)
    }

    fn bottom_right_bevel_curve(
        chip_left: usize,
        chip_top: usize,
        chip_right: usize,
        chip_bottom: usize,
        inner_right: usize,
        inner_top: usize,
        inner_bottom: usize,
        corner_radius: usize,
        fill: &str,
    ) -> SvgPath {
        let inner_radius = corner_radius;
        let data = Data::new()
            .move_to((chip_right, chip_top))
            .line_to((chip_right, chip_bottom - corner_radius))
            .quadratic_curve_to((chip_right, chip_bottom, chip_right - corner_radius, chip_bottom))
            .line_to((chip_left, chip_bottom))
            .line_to((chip_left + corner_radius, inner_bottom))
            .line_to((inner_right - inner_radius, inner_bottom))
            .quadratic_curve_to((inner_right, inner_bottom, inner_right, inner_bottom - corner_radius))
            .line_to((inner_right, inner_top))
            .close();

        SvgPath::new().set("d", data).set("fill", fill)
    }

    fn bottom_left_bevel_curve(
        chip_left: usize,
        chip_top: usize,
        chip_right: usize,
        chip_bottom: usize,
        inner_left: usize,
        inner_top: usize,
        inner_bottom: usize,
        corner_radius: usize,
        fill: &str,
    ) -> SvgPath {
        let inner_radius = corner_radius;
        let data = Data::new()
            .move_to((chip_right, chip_bottom))
            .line_to((chip_left + corner_radius, chip_bottom))
            .quadratic_curve_to((chip_left, chip_bottom, chip_left, chip_bottom - corner_radius))
            .line_to((chip_left, chip_top))
            .line_to((inner_left, inner_top))
            .line_to((inner_left, inner_bottom - inner_radius))
            .quadratic_curve_to((inner_left, inner_bottom, inner_left + inner_radius, inner_bottom))
            .line_to((chip_right - corner_radius, inner_bottom))
            .close();

        SvgPath::new().set("d", data).set("fill", fill)
    }
}
