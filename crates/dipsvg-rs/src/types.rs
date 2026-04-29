use crate::util::gradient_coords_from_strs;
use svg::node::element::Group;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ChipOrientation {
    #[default]
    NotchUp,
    NotchLeft,
    NotchRight,
    NotchDown,
}

impl ChipOrientation {
    pub fn body_gradient_coords(self: ChipOrientation) -> (String, String, String, String) {
        match self {
            ChipOrientation::NotchUp => gradient_coords_from_strs("0%", "0%", "0%", "100%"),
            ChipOrientation::NotchLeft => gradient_coords_from_strs("100%", "0%", "0%", "0%"),
            ChipOrientation::NotchRight => gradient_coords_from_strs("0%", "0%", "100%", "0%"),
            ChipOrientation::NotchDown => gradient_coords_from_strs("0%", "100%", "0%", "0%"),
        }
    }

    pub fn upright_transform(self: ChipOrientation, width: usize, height: usize, content: Group) -> Group {
        let new_content = match self {
            ChipOrientation::NotchUp => return content,
            ChipOrientation::NotchLeft => content.set("transform", format!("translate(0 {width}) rotate(-90)")),
            ChipOrientation::NotchRight => content.set("transform", format!("translate({height} 0) rotate(90)")),
            ChipOrientation::NotchDown => content.set("transform", format!("translate({width} {height}) rotate(180)")),
        };
        new_content
    }

    pub fn upright_transform_string(self: ChipOrientation, width: usize, height: usize) -> String {
        match self {
            ChipOrientation::NotchUp => "".to_string(),
            ChipOrientation::NotchLeft => {
                format!("translate(0 {width}) rotate(-90)")
            }
            ChipOrientation::NotchRight => {
                format!("translate({height} 0) rotate(90)")
            }
            ChipOrientation::NotchDown => {
                format!("translate({width} {height}) rotate(180)")
            }
        }
    }

    pub fn upright_rotate_string(self: ChipOrientation) -> String {
        match self {
            ChipOrientation::NotchUp => "".to_string(),
            ChipOrientation::NotchLeft => {
                format!("rotate(-90)")
            }
            ChipOrientation::NotchRight => {
                format!("rotate(90)")
            }
            ChipOrientation::NotchDown => {
                format!("rotate(180)")
            }
        }
    }

    pub fn label_upright_transform_string(self: ChipOrientation, x: usize, y: usize) -> String {
        match self {
            ChipOrientation::NotchUp => "".to_string(),
            ChipOrientation::NotchLeft => format!("rotate(90 {x} {y})"),
            ChipOrientation::NotchRight => format!("rotate(-90 {x} {y})"),
            ChipOrientation::NotchDown => format!("rotate(180 {x} {y})"),
        }
    }
}
