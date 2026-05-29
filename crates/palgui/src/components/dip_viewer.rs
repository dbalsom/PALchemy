use std::collections::HashMap;

use dipsvg::{ChipDiagram, ChipDiagramOptions, ChipDiagramStyle, ChipGeometry};
use leptos::prelude::*;
use palcore::{AppSettings, ChipDef, InteractiveStatus, PinDirection, PinState};
use stylance::import_style;

use crate::components::pin_control_row::PinControlRow;

import_style!(style, "dip_viewer.module.scss");

#[component]
pub fn DipViewer(
    chip: RwSignal<Option<ChipDef>>,
    settings: RwSignal<AppSettings>,
    pin_directions: RwSignal<HashMap<u8, PinDirection>>,
    pin_toggles: RwSignal<HashMap<u8, bool>>,
    output_states: RwSignal<HashMap<u8, PinState>>,
    interactive_status: RwSignal<InteractiveStatus>,
) -> impl IntoView {
    view! {
        {move || {
            let Some(chip) = chip.get() else {
                return view! {
                    <div class=style::viewer_container id="dip-container">
                        <div class=style::placeholder_text>
                            "Select a chip to view pinout"
                        </div>
                    </div>
                }
                .into_any();
            };

            let pin_count = chip.pins as usize;
            let pins_per_side = pin_count / 2;
            let layout = ChipGeometry::from_chip(&chip);
            let chip_height = layout.chip_height();
            let diagram_width = layout.labeled_svg_width();

            let left_pins: Vec<u8> = (1..=pins_per_side as u8).collect();
            let right_pins: Vec<u8> = (pins_per_side as u8 + 1..=pin_count as u8).rev().collect();

            let layout_style = format!(
                "--dip-pin-pitch:{}px;--dip-pin-inset:{}px;--dip-chip-height:{}px;--dip-svg-width:{}px;",
                layout.pin_pitch,
                layout.pin_inset,
                chip_height,
                diagram_width
            );

            let chip_for_left = chip.clone();
            let chip_for_right = chip.clone();

            view! {
                <div class=style::viewer_container id="dip-container">
                    <div class=style::dip_wrapper style=layout_style>
                        <div class=format!("{} left-controls", style::pin_controls)>
                            <For each=move || left_pins.clone().into_iter().enumerate() key=|(index, pin)| (*index, *pin) let:item>
                                {
                                    let (row_index, pin) = item;
                                    let pin_def = chip_for_left.pinout.get(&pin.to_string()).cloned();
                                    view! {
                                        <PinControlRow
                                            pin=pin
                                            side="left"
                                            row_index=row_index
                                            rows_per_side=pins_per_side
                                            pin_def=pin_def
                                            pin_directions=pin_directions
                                            pin_toggles=pin_toggles
                                            output_states=output_states
                                            interactive_status=interactive_status
                                        />
                                    }
                                }
                            </For>
                        </div>

                        <div
                            class=style::chip_svg_container
                            inner_html=move || {
                                let diagram_style =
                                    ChipDiagramStyle::default().with_high_contrast(settings.get().high_contrast);
                                let diagram_options = ChipDiagramOptions {
                                    geometry: layout,
                                    style: diagram_style,
                                };

                                ChipDiagram::from_chip(&chip)
                                    .and_then(|diagram| diagram.with_options(diagram_options).render())
                                    .unwrap_or_else(|_| {
                                        format!(
                                            r#"<div class="{}">Unable to render chip diagram</div>"#,
                                            style::placeholder_text,
                                        )
                                    })
                            }
                        ></div>

                        <div class=format!("{} right-controls", style::pin_controls)>
                            <For each=move || right_pins.clone().into_iter().enumerate() key=|(index, pin)| (*index, *pin) let:item>
                                {
                                    let (row_index, pin) = item;
                                    let pin_def = chip_for_right.pinout.get(&pin.to_string()).cloned();
                                    view! {
                                        <PinControlRow
                                            pin=pin
                                            side="right"
                                            row_index=row_index
                                            rows_per_side=pins_per_side
                                            pin_def=pin_def
                                            pin_directions=pin_directions
                                            pin_toggles=pin_toggles
                                            output_states=output_states
                                            interactive_status=interactive_status
                                        />
                                    }
                                }
                            </For>
                        </div>
                    </div>
                </div>
            }
            .into_any()
        }}
    }
    .into_any()
}
