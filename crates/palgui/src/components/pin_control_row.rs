use std::collections::HashMap;

use leptos::prelude::*;
use palcore::{InteractiveStatus, PinDef, PinDirection, PinState, PinType};
use stylance::import_style;

import_style!(style, "pin_control_row.module.scss");

#[component]
pub fn PinControlRow(
    pin: u8,
    side: &'static str,
    row_index: usize,
    rows_per_side: usize,
    pin_def: Option<PinDef>,
    pin_directions: RwSignal<HashMap<u8, PinDirection>>,
    pin_toggles: RwSignal<HashMap<u8, bool>>,
    output_states: RwSignal<HashMap<u8, PinState>>,
    interactive_status: RwSignal<InteractiveStatus>,
) -> impl IntoView {
    let (name_text, is_active_low) = match &pin_def {
        Some(definition) => {
            let name = definition.name.clone().unwrap_or_else(|| format!("P{pin}"));
            if let Some(stripped) = name.strip_prefix('/') {
                (stripped.to_string(), true)
            } else {
                (name, false)
            }
        }
        None => (format!("P{pin}"), false),
    };
    let accessible_pin_name = format!("Pin {pin} {name_text}");

    let pin_type = pin_def.as_ref().map(|definition| definition.pin_type.clone());
    match pin_type {
        None
        | Some(PinType::Power)
        | Some(PinType::Ground)
        | Some(PinType::NotConnected)
        | Some(PinType::Vpp)
        | Some(PinType::OutputEnable) => {
            let type_label = match pin_type {
                Some(PinType::Power) | Some(PinType::Vpp) => "VCC",
                Some(PinType::Ground) => "GND",
                Some(PinType::OutputEnable) => "OE",
                _ => "NC",
            };
            let type_class = match pin_type {
                Some(PinType::Power) | Some(PinType::Vpp) => {
                    format!("{} {}", style::pin_type, style::pin_type_power)
                }
                Some(PinType::Ground) => {
                    format!("{} {}", style::pin_type, style::pin_type_ground)
                }
                Some(PinType::OutputEnable) => {
                    format!("{} {}", style::pin_type, style::pin_type_input)
                }
                _ => format!("{} {}", style::pin_type, style::pin_type_nc),
            };

            view! {
                <div
                    class=style::pin_row
                    data-pin=pin.to_string()
                    role="group"
                    aria-label=format!("{accessible_pin_name} {type_label}")
                >
                    <div class=style::pin_label>
                        <span style=if is_active_low { "text-decoration: overline" } else { "" }>
                            {name_text}
                        </span>
                    </div>
                    <div class=type_class>{type_label}</div>
                    <div style="width: 34px"></div>
                </div>
            }
            .into_any()
        }
        Some(PinType::Input) => view! {
            <div
                class=style::pin_row
                data-pin=pin.to_string()
                role="group"
                aria-label=format!("{accessible_pin_name} input")
            >
                <div class=style::pin_label>
                    <span style=if is_active_low { "text-decoration: overline" } else { "" }>
                        {name_text.clone()}
                    </span>
                </div>
                <div class=format!("{} {}", style::pin_type, style::pin_type_input)>"Input"</div>
                    <div style="width: 34px; display: flex; justify-content: center">
                    <PinToggle
                        pin=pin
                        side=side
                        row_index=row_index
                        rows_per_side=rows_per_side
                        label=name_text.clone()
                        pin_toggles=pin_toggles
                        interactive_status=interactive_status
                    />
                </div>
            </div>
        }
        .into_any(),
        Some(PinType::Output) => view! {
            <div
                class=style::pin_row
                data-pin=pin.to_string()
                role="group"
                aria-label=format!("{accessible_pin_name} output")
            >
                <div class=style::pin_label>
                    <span style=if is_active_low { "text-decoration: overline" } else { "" }>
                        {name_text}
                    </span>
                </div>
                <div class=format!("{} {}", style::pin_type, style::pin_type_output)>"Output"</div>
                <div style="width: 34px; display: flex; justify-content: center">
                    <PinLed pin=pin output_states=output_states/>
                </div>
                <span class="sr_only">
                    {move || format!("Output state: {}", pin_state_label(output_states.get().get(&pin).copied().unwrap_or(PinState::Z)))}
                </span>
            </div>
        }
        .into_any(),
        Some(PinType::InputOutput) => view! {
            <div
                class=style::pin_row
                data-pin=pin.to_string()
                role="group"
                aria-label=format!("{accessible_pin_name} bidirectional pin")
            >
                <div class=style::pin_label>
                    <span style=if is_active_low { "text-decoration: overline" } else { "" }>
                        {name_text.clone()}
                    </span>
                </div>
                <select
                    data-nav-focus="true"
                    data-nav-side=side
                    data-nav-row=row_index.to_string()
                    data-nav-kind="select"
                    tabindex=0
                    aria-label=format!("Set direction for {accessible_pin_name}")
                    class=move || {
                        let direction = pin_directions.get().get(&pin).copied().unwrap_or(PinDirection::Input);
                        if direction == PinDirection::Input {
                            format!("{} {} {}", style::pin_dropdown, style::pin_type_io, style::pin_type_input)
                        } else {
                            format!("{} {} {}", style::pin_dropdown, style::pin_type_io, style::pin_type_output)
                        }
                    }
                    on:change=move |event| {
                        let direction = if event_target_value(&event) == "output" {
                            PinDirection::Output
                        } else {
                            PinDirection::Input
                        };
                        pin_directions.update(|directions| {
                            directions.insert(pin, direction);
                        });
                    }
                    on:keydown=move |event| {
                        handle_pin_control_keydown(event, side, row_index, rows_per_side, "select");
                    }
                >
                    <option value="input" selected=true>
                        "Input"
                    </option>
                    <option value="output">"Output"</option>
                </select>
                <div style="width: 34px; display: flex; justify-content: center">
                    {move || {
                        let direction = pin_directions.get().get(&pin).copied().unwrap_or(PinDirection::Input);
                        if direction == PinDirection::Input {
                            view! {
                                <PinToggle
                                    pin=pin
                                    side=side
                                    row_index=row_index
                                    rows_per_side=rows_per_side
                                    label=name_text.clone()
                                    pin_toggles=pin_toggles
                                    interactive_status=interactive_status
                                />
                            }
                                .into_any()
                        } else {
                            view! { <PinLed pin=pin output_states=output_states/> }.into_any()
                        }
                    }}
                </div>
                <span class="sr_only">
                    {move || {
                        let direction = pin_directions.get().get(&pin).copied().unwrap_or(PinDirection::Input);
                        if direction == PinDirection::Input {
                            "Direction is input.".to_string()
                        } else {
                            format!(
                                "Direction is output. Output state: {}",
                                pin_state_label(output_states.get().get(&pin).copied().unwrap_or(PinState::Z))
                            )
                        }
                    }}
                </span>
            </div>
        }
        .into_any(),
    }
}

#[component]
fn PinToggle(
    pin: u8,
    side: &'static str,
    row_index: usize,
    rows_per_side: usize,
    label: String,
    pin_toggles: RwSignal<HashMap<u8, bool>>,
    interactive_status: RwSignal<InteractiveStatus>,
) -> impl IntoView {
    view! {
        <button
            type="button"
            data-nav-focus="true"
            data-nav-side=side
            data-nav-row=row_index.to_string()
            data-nav-kind="toggle"
            tabindex=0
            aria-label=move || {
                let state = if pin_toggles.get().get(&pin).copied().unwrap_or(false) {
                    "high"
                } else {
                    "low"
                };
                format!("Toggle pin {pin} {label} {state}")
            }
            aria-pressed=move || pin_toggles.get().get(&pin).copied().unwrap_or(false)
            class=move || {
                let active = pin_toggles.get().get(&pin).copied().unwrap_or(false);
                format!("{} {}", style::pin_toggle, if active { style::active } else { "" })
            }
            disabled=move || interactive_status.get() != InteractiveStatus::Running
            on:click=move |_| {
                if interactive_status.get() != InteractiveStatus::Running {
                    return;
                }
                pin_toggles.update(|toggles| {
                    let current = toggles.get(&pin).copied().unwrap_or(false);
                    toggles.insert(pin, !current);
                });
            }
            on:keydown=move |event| {
                handle_pin_control_keydown(event, side, row_index, rows_per_side, "toggle");
            }
        ></button>
    }
}

#[component]
fn PinLed(pin: u8, output_states: RwSignal<HashMap<u8, PinState>>) -> impl IntoView {
    view! {
        <div aria-hidden="true" class=move || {
            let state = output_states.get().get(&pin).copied().unwrap_or(PinState::Z);
            let is_on = matches!(state, PinState::High);
            format!("{} {}", style::pin_led, if is_on { style::on } else { "" })
        }></div>
    }
}

fn pin_state_label(state: PinState) -> &'static str {
    match state {
        PinState::High => "high",
        PinState::Low => "low",
        PinState::Z => "high impedance",
    }
}

#[cfg(feature = "csr")]
fn handle_pin_control_keydown(
    event: leptos::ev::KeyboardEvent,
    side: &'static str,
    row_index: usize,
    rows_per_side: usize,
    kind: &'static str,
) {
    use wasm_bindgen::JsCast;

    let (target_side, target_row, direction, prefer_same_row) = match event.key().as_str() {
        "ArrowUp" => (side, row_index.saturating_sub(1), -1, false),
        "ArrowDown" => (side, row_index.saturating_add(1), 1, false),
        "ArrowLeft" => (opposite_side(side), row_index, 0, true),
        "ArrowRight" => (opposite_side(side), row_index, 0, true),
        _ => return,
    };

    event.prevent_default();

    let Some(document) = web_sys::window().and_then(|window| window.document()) else {
        return;
    };

    let target = if prefer_same_row {
        find_row_control(&document, target_side, target_row, kind)
    } else {
        find_directional_control(&document, target_side, target_row, rows_per_side, direction, kind)
    };

    if let Some(element) = target.and_then(|element| element.dyn_into::<web_sys::HtmlElement>().ok()) {
        let _ = element.focus();
    }
}

#[cfg(not(feature = "csr"))]
fn handle_pin_control_keydown(
    _event: leptos::ev::KeyboardEvent,
    _side: &'static str,
    _row_index: usize,
    _rows_per_side: usize,
    _kind: &'static str,
) {
}

#[cfg(feature = "csr")]
fn find_directional_control(
    document: &web_sys::Document,
    side: &'static str,
    start_row: usize,
    rows_per_side: usize,
    direction: i32,
    kind: &'static str,
) -> Option<web_sys::Element> {
    if direction == 0 {
        return None;
    }

    let mut row = start_row as i32;
    while row >= 0 && row < rows_per_side as i32 {
        if let Some(element) = find_row_control(document, side, row as usize, kind) {
            return Some(element);
        }
        row += direction;
    }

    None
}

#[cfg(feature = "csr")]
fn find_row_control(
    document: &web_sys::Document,
    side: &'static str,
    row_index: usize,
    kind: &'static str,
) -> Option<web_sys::Element> {
    find_selector(
        document,
        &format!(r#"[data-nav-side="{side}"][data-nav-row="{row_index}"][data-nav-kind="{kind}"]"#),
    )
    .or_else(|| {
        find_selector(
            document,
            &format!(r#"[data-nav-side="{side}"][data-nav-row="{row_index}"][data-nav-focus="true"]"#),
        )
    })
}

#[cfg(feature = "csr")]
fn find_selector(document: &web_sys::Document, selector: &str) -> Option<web_sys::Element> {
    document.query_selector(selector).ok().flatten()
}

#[cfg(feature = "csr")]
fn opposite_side(side: &'static str) -> &'static str {
    match side {
        "left" => "right",
        _ => "left",
    }
}
