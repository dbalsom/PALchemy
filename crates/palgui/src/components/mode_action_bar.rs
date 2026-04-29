use leptos::prelude::*;
use palcore::{ChipDef, InteractiveStatus, SelectedMode};
use stylance::import_style;

import_style!(style, "mode_action_bar.module.scss");

#[component]
pub fn ModeActionBar(
    current_chip: RwSignal<Option<ChipDef>>,
    selected_mode: RwSignal<SelectedMode>,
    interactive_status: RwSignal<InteractiveStatus>,
    action_disabled: Signal<bool>,
    on_action: impl Fn(leptos::ev::MouseEvent) + Copy + 'static,
) -> impl IntoView {
    view! {
        <section class=style::mode_action_bar aria-labelledby="chip-mode-heading" aria-describedby="chip-mode-description">
            <ChipModeInfo current_chip=current_chip selected_mode=selected_mode/>
            <button
                id="btn-action"
                aria-describedby="chip-mode-description chip-action-help"
                class=move || {
                    if interactive_status.get() == InteractiveStatus::Running {
                        "btn primary"
                    } else {
                        "btn secondary"
                    }
                }
                disabled=move || action_disabled.get()
                on:click=on_action
            >
                {move || action_label(selected_mode.get(), interactive_status.get())}
            </button>
            <span id="chip-action-help" class="sr_only">
                {move || action_help_text(current_chip.get(), selected_mode.get(), action_disabled.get())}
            </span>
        </section>
    }
}

#[component]
fn ChipModeInfo(current_chip: RwSignal<Option<ChipDef>>, selected_mode: RwSignal<SelectedMode>) -> impl IntoView {
    view! {
        <div class=style::mode_info>
            <h2 id="chip-mode-heading" class=style::chip_name>
                {move || current_chip.get().map(chip_heading).unwrap_or_default()}
            </h2>
            <p class=style::chip_description>
                {move || current_chip.get().and_then(chip_description).unwrap_or_default()}
            </p>
            <p id="chip-mode-description" class=style::mode_desc>
                {move || match selected_mode.get() {
                    SelectedMode::Interact => {
                        "Interactive Mode: Toggle input pins to observe output states."
                    }
                    SelectedMode::Dump => {
                        "Combinatorial Dump: Exhaustively scan inputs to find logical mappings."
                    }
                }}
            </p>
        </div>
    }
}

fn action_label(mode: SelectedMode, status: InteractiveStatus) -> &'static str {
    match (mode, status) {
        (SelectedMode::Interact, InteractiveStatus::Running) => "Stop Interactive",
        (SelectedMode::Interact, InteractiveStatus::Stopped) => "Start Interactive",
        (SelectedMode::Dump, _) => "Start Dump",
    }
}

fn chip_heading(chip: ChipDef) -> String {
    let label = if chip.name == chip.model {
        chip.name
    } else {
        format!("{} ({})", chip.name, chip.model)
    };

    if let Some(source) = chip.source {
        format!("{label} - {source}")
    } else {
        label
    }
}

fn chip_description(chip: ChipDef) -> Option<String> {
    let mut text = chip.model_description;
    if let Some(app_description) = chip.app_description {
        text = format!("{text} - {app_description}");
    }

    if text.is_empty() {
        None
    } else {
        Some(text)
    }
}

fn action_help_text(current_chip: Option<ChipDef>, mode: SelectedMode, action_disabled: bool) -> &'static str {
    if action_disabled {
        if current_chip.is_none() {
            "Select a chip before starting a mode."
        } else {
            "Connect a device before starting a mode."
        }
    } else {
        match mode {
            SelectedMode::Interact => "Starts or stops interactive mode for the selected chip.",
            SelectedMode::Dump => "Starts a combinatorial dump for the selected chip.",
        }
    }
}
