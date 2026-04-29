use svg::node::element::{LinearGradient, Stop};

pub fn midpoint(left: usize, right: usize) -> f32 {
    (left + right) as f32 / 2.0
}

pub fn gradient_coords_from_strs(x1: &str, y1: &str, x2: &str, y2: &str) -> (String, String, String, String) {
    (x1.to_string(), y1.to_string(), x2.to_string(), y2.to_string())
}

pub fn gradient_coords(angle_degrees: f32) -> (String, String, String, String) {
    let radians = angle_degrees.to_radians();
    let dx = radians.cos();
    let dy = radians.sin();
    let x1 = 50.0 - dx * 50.0;
    let y1 = 50.0 - dy * 50.0;
    let x2 = 50.0 + dx * 50.0;
    let y2 = 50.0 + dy * 50.0;

    (
        format!("{x1:.2}%"),
        format!("{y1:.2}%"),
        format!("{x2:.2}%"),
        format!("{y2:.2}%"),
    )
}

pub fn gradient(id: &str, coords: (String, String, String, String), stops: &[(&str, &str, &str)]) -> LinearGradient {
    let (x1, y1, x2, y2) = coords;
    stops.iter().fold(
        LinearGradient::new()
            .set("id", id)
            .set("x1", x1.as_str())
            .set("y1", y1.as_str())
            .set("x2", x2.as_str())
            .set("y2", y2.as_str()),
        |gradient, (offset, color, opacity)| {
            gradient.add(
                Stop::new()
                    .set("offset", *offset)
                    .set("stop-color", *color)
                    .set("stop-opacity", *opacity),
            )
        },
    )
}

pub fn gradient_with_style_stops(
    id: &str,
    coords: (String, String, String, String),
    stops: &[(&str, &str)],
) -> LinearGradient {
    let (x1, y1, x2, y2) = coords;
    stops.iter().fold(
        LinearGradient::new()
            .set("id", id)
            .set("x1", x1.as_str())
            .set("y1", y1.as_str())
            .set("x2", x2.as_str())
            .set("y2", y2.as_str()),
        |gradient, (offset, style)| gradient.add(Stop::new().set("offset", *offset).set("style", *style)),
    )
}
