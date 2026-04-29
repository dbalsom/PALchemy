use leptos::prelude::*;
use palcore::ConnectionFailureEvent;
use stylance::import_style;

import_style!(style, "error_modal.module.scss");

#[component]
pub fn ErrorModal(
    show_error_modal: RwSignal<bool>,
    error_modal: RwSignal<Option<ConnectionFailureEvent>>,
) -> impl IntoView {
    view! {
        <Show when=move || show_error_modal.get()>
            <div
                class=style::modal_overlay
                on:click=move |_| show_error_modal.set(false)
                on:keydown=move |event| {
                    if event.key() == "Escape" {
                        show_error_modal.set(false);
                    }
                }
            >
                <div
                    class=style::modal_content
                    role="alertdialog"
                    aria-modal="true"
                    aria-labelledby="connection-error-title"
                    aria-describedby="connection-error-message"
                    tabindex="-1"
                    on:click=move |event| event.stop_propagation()
                >
                    <div class=style::modal_header>
                        <h2 id="connection-error-title">{move || {
                            error_modal
                                .get()
                                .map(|event| event.title)
                                .unwrap_or_else(|| "Error".to_string())
                        }}</h2>
                        <button
                            class=style::close_icon
                            aria-label="Close error dialog"
                            autofocus
                            on:click=move |_| show_error_modal.set(false)
                        >
                            "x"
                        </button>
                    </div>

                    <div class=style::modal_body>
                        <p id="connection-error-message" class=style::message>
                            {move || {
                                error_modal
                                    .get()
                                    .map(|event| event.message)
                                    .unwrap_or_else(|| "An unexpected error occurred.".to_string())
                            }}
                        </p>
                        <Show when=move || {
                            error_modal
                                .get()
                                .and_then(|event| event.device_type)
                                .is_some()
                        }>
                            <p class=style::device_label>
                                {move || {
                                    error_modal
                                        .get()
                                        .and_then(|event| event.device_type)
                                        .map(|device_type| format!("Device: {device_type}"))
                                        .unwrap_or_default()
                                }}
                            </p>
                        </Show>
                    </div>

                    <div class=style::modal_footer>
                        <button class="btn secondary" on:click=move |_| show_error_modal.set(false)>
                            "Dismiss"
                        </button>
                    </div>
                </div>
            </div>
        </Show>
    }
}
