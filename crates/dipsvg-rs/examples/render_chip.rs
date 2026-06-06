// dipsvg-rs: A Rust library for rendering chip definitions to SVG diagrams
// Copyright (C) 2026 Daniel Balsom
// SPDX-License-Identifier: MIT OR GPL-3.0-or-later

// Example of how to render a chip diagram from a TOML file to SVG.

use std::{env, fs, path::PathBuf};

use dipsvg::{ChipDiagram, ChipDiagramOptions, ChipDiagramStyle, ChipGeometry};

const USAGE: &str = "usage: render_chip <chip.toml> <output.svg>";

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = env::args_os().skip(1);
    let input = args
        .next()
        .map(PathBuf::from)
        .ok_or(USAGE)?;
    let output = args
        .next()
        .map(PathBuf::from)
        .ok_or(USAGE)?;

    // We create a ChipDiagram, and call build() to produce the SVG, which is just a string.
    // ChipGeometry defines the physical geometry of the chip, in 'transparent' attributes such
    // as radii, lengths, widths.
    // ChipDiagramStyle defines the visual style of the chip, things like colors, shadows,
    // label orientation. Think of it as like "CSS for chips".
    let svg = ChipDiagram::from_toml_file(&input)?
        .with_options(ChipDiagramOptions {
            geometry: ChipGeometry::default().with_notch_radius(Some(12.0)),
            style: ChipDiagramStyle::default()
                .with_chip_body_drop_shadow(true)
                .with_keep_labels_upright(true)
                .with_shade_angle(45.0),
        })
        .render()?;

    fs::write(output, svg)?;
    Ok(())
}
