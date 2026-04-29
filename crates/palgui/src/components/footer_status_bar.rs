use leptos::prelude::*;
use palcore::{ConnectionState, DeviceInfo};
use stylance::import_style;

import_style!(style, "footer_status_bar.module.scss");

#[component]
pub fn FooterStatusBar(
    connection: RwSignal<ConnectionState>,
    device_info: RwSignal<Option<DeviceInfo>>,
    show_device_modal: RwSignal<bool>,
) -> impl IntoView {
    view! {
        <footer class=style::status_bar>
            <div class=style::status_left role="status" aria-live="polite" aria-atomic="true">
                <DeviceStatusIndicator
                    connection=connection
                    device_info=device_info
                    show_device_modal=show_device_modal
                />
            </div>
        </footer>
    }
}

#[component]
fn DeviceStatusIndicator(
    connection: RwSignal<ConnectionState>,
    device_info: RwSignal<Option<DeviceInfo>>,
    show_device_modal: RwSignal<bool>,
) -> impl IntoView {
    view! {
        <span
            aria-hidden="true"
            class=move || {
                let state = if connection.get() == ConnectionState::Connected {
                    style::connected
                } else {
                    style::error
                };
                format!("{} {}", style::status_dot, state)
            }
        ></span>
        <Show
            when=move || connection.get() == ConnectionState::Disconnected
            fallback=move || {
                view! {
                    <>
                        <span class=style::connection_type>
                            {move || {
                                device_info
                                    .get()
                                    .map(|info| connection_type_label(&info))
                                    .unwrap_or_else(|| "Unknown".to_string())
                            }}
                        </span>
                        <span class=style::status_label>"Device Connected:"</span>
                        <span class=style::device_name>
                            {move || {
                                device_info
                                    .get()
                                    .map(|info| info.name)
                                    .unwrap_or_else(|| "Unknown device".to_string())
                            }}
                        </span>
                    </>
                }
            }
        >
            <span class=style::status_label>"Device Disconnected"</span>
        </Show>
        <button
            class=move || {
                if connection.get() == ConnectionState::Connected {
                    style::info_btn.to_string()
                } else {
                    format!("{} {}", style::info_btn, style::info_btn_hidden)
                }
            }
            title="Show Device Info"
            aria-label=move || {
                if let Some(info) = device_info.get() {
                    format!("Show device info for {}", info.name)
                } else {
                    "Show device info".to_string()
                }
            }
            disabled=move || connection.get() != ConnectionState::Connected
            on:click=move |_| show_device_modal.set(true)
        >
            "?"
        </button>
    }
}

fn connection_type_label(info: &DeviceInfo) -> String {
    match &info.connection_type {
        palcore::ConnectionType::Usb(_) => "USB".to_string(),
        palcore::ConnectionType::SerialUart => "Serial UART".to_string(),
        palcore::ConnectionType::UsbCdc(_) => "USB CDC".to_string(),
        palcore::ConnectionType::Ethernet => "Ethernet".to_string(),
        palcore::ConnectionType::Wifi => "Wi-Fi".to_string(),
        palcore::ConnectionType::Other(value) => value.clone(),
    }
}
