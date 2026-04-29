#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ChipGeometry {
    pub pin_pitch: usize,
    pub top_inset: usize,
    pub bottom_inset: usize,
    pub chip_width: usize,
    pub pin_stub_width: usize,
    pub pin_stub_height: usize,
    pub chip_corner_radius: usize,
    pub notch_radius: usize,
    pub bevel_inset: usize,
}

impl Default for ChipGeometry {
    fn default() -> Self {
        Self {
            pin_pitch: 44,
            top_inset: 30,
            bottom_inset: 30,
            chip_width: 150,
            pin_stub_width: 15,
            pin_stub_height: 10,
            chip_corner_radius: 6,
            notch_radius: 12,
            bevel_inset: 3,
        }
    }
}

impl ChipGeometry {
    pub fn chip_height(&self, pins_per_side: usize) -> usize {
        pins_per_side * self.pin_pitch + self.top_inset + self.bottom_inset
    }

    pub fn svg_width(&self) -> usize {
        self.chip_width + self.pin_stub_width * 2
    }

    pub fn pin_center_y(&self, index: usize) -> usize {
        self.top_inset + (index * self.pin_pitch) + self.pin_pitch / 2
    }

    pub fn with_notch_radius(mut self, notch_radius: Option<f32>) -> Self {
        if let Some(radius) = notch_radius {
            self.notch_radius = radius as usize;
        } else {
            self.notch_radius = 0;
        }
        self
    }
}
