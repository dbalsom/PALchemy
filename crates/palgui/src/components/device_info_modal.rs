use leptos::prelude::*;
use palcore::{ConnectionType, DeviceInfo, UsbSpeed};
use stylance::import_style;

import_style!(style, "device_info_modal.module.scss");

#[component]
pub fn DeviceInfoModal(show_device_modal: RwSignal<bool>, device_info: RwSignal<Option<DeviceInfo>>) -> impl IntoView {
    view! {
        <Show when=move || show_device_modal.get()>
            <div
                class=style::modal_overlay
                on:click=move |_| show_device_modal.set(false)
                on:keydown=move |event| {
                    if event.key() == "Escape" {
                        show_device_modal.set(false);
                    }
                }
            >
                <div
                    class=style::modal_content
                    role="dialog"
                    aria-modal="true"
                    aria-labelledby="device-info-title"
                    aria-describedby="device-info-summary"
                    tabindex="-1"
                    on:click=move |event| event.stop_propagation()
                >
                    <div class=style::modal_header>
                        <h2 id="device-info-title">"Device Information"</h2>
                        <button
                            class=style::close_icon
                            aria-label="Close device information dialog"
                            autofocus
                            on:click=move |_| show_device_modal.set(false)
                        >
                            "x"
                        </button>
                    </div>
                    <div class=style::modal_body>
                        <p id="device-info-summary" class="sr_only">
                            "Details about the currently connected device and its capabilities."
                        </p>
                        {move || {
                            device_info.get().map(|info| {
                                let capabilities: Vec<String> = [
                                    ("pullup", info.capabilities.pullup),
                                    ("pulldown", info.capabilities.pulldown),
                                    ("variable voltage", info.capabilities.variable_voltage),
                                    ("high speed clock", info.capabilities.high_speed_clock),
                                    ("custom logic", info.capabilities.custom_logic),
                                ]
                                .into_iter()
                                .filter_map(|(label, enabled)| enabled.then(|| label.to_string()))
                                .collect();

                                view! {
                                    <div class=style::info_grid role="group" aria-label="Connected device details">
                                        <span class=style::info_label>"Name"</span>
                                        <span class=style::info_value>{info.name.clone()}</span>

                                        <span class=style::info_label>"Connection"</span>
                                        <span class=style::info_value>{connection_label(&info.connection_type)}</span>

                                        {optional_info_row("Firmware Version", info.firmware_version.clone())}
                                        {optional_info_row("Device Code", info.device_code.clone())}
                                        {optional_info_row("Serial Number", info.serial_number.clone())}
                                        {optional_info_row("Manufacture Date", info.manufacture_date.clone())}

                                        {if let Some(volts) = info.supply_voltage {
                                            view! {
                                                <span class=style::info_label>"USB Voltage"</span>
                                                <span class=style::info_value>{format!("{volts:.2}V")}</span>
                                            }
                                                .into_any()
                                        } else {
                                            view! {}.into_any()
                                        }}

                                        <span class=style::info_label>"Pins"</span>
                                        <span class=style::info_value>{info.num_pins.to_string()}</span>

                                        <span class=style::info_label>"Capabilities"</span>
                                        <div>
                                            {if capabilities.is_empty() {
                                                view! {
                                                    <span class=style::capability_tag>"Standard GPIO"</span>
                                                }
                                                    .into_any()
                                            } else {
                                                capabilities
                                                    .into_iter()
                                                    .map(|capability| {
                                                        view! {
                                                            <span class=style::capability_tag>{capability}</span>
                                                        }
                                                    })
                                                    .collect_view()
                                                    .into_any()
                                            }}
                                        </div>

                                        {info
                                            .additional_info
                                            .iter()
                                            .map(|(key, value)| {
                                                view! {
                                                    <span class=style::info_label>{key.clone()}</span>
                                                    <span class=style::info_value>{value.clone()}</span>
                                                }
                                            })
                                            .collect_view()}
                                    </div>
                                }
                                .into_any()
                            })
                            .unwrap_or_else(|| {
                                view! { <p>"No device information is available."</p> }.into_any()
                            })
                        }}
                    </div>
                    <div class=style::modal_footer>
                        <button class="btn secondary" on:click=move |_| show_device_modal.set(false)>
                            "Close"
                        </button>
                    </div>
                </div>
            </div>
        </Show>
    }
}

fn optional_info_row(label: &'static str, value: Option<String>) -> AnyView {
    value
        .map(|value| {
            view! {
                <span class=style::info_label>{label}</span>
                <span class=style::info_value>{value}</span>
            }
            .into_any()
        })
        .unwrap_or_else(|| view! {}.into_any())
}

fn connection_label(connection: &ConnectionType) -> String {
    match connection {
        ConnectionType::Usb(speed) => match speed {
            UsbSpeed::Full12Mbps => "USB (12 Mbps)".to_string(),
            UsbSpeed::High480Mbps => "USB (480 Mbps)".to_string(),
            UsbSpeed::Super5Gbps => "USB (5 Gbps)".to_string(),
            UsbSpeed::Low => "USB (Low Speed)".to_string(),
            UsbSpeed::Unknown => "USB".to_string(),
        },
        ConnectionType::SerialUart => "Serial UART".to_string(),
        ConnectionType::UsbCdc(_) => "USB CDC Serial".to_string(),
        ConnectionType::Ethernet => "Ethernet".to_string(),
        ConnectionType::Wifi => "Wi-Fi".to_string(),
        ConnectionType::Other(value) => value.clone(),
    }
}
