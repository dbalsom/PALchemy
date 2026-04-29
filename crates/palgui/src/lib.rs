pub mod app;
pub mod components;
pub mod controller;
pub mod events;
pub mod ipc;
pub mod menu;
pub mod platform;
pub mod state;

#[cfg(feature = "csr")]
#[wasm_bindgen::prelude::wasm_bindgen(start)]
pub fn main() {
    console_error_panic_hook::set_once();
    let _ = console_log::init_with_level(log::Level::Debug);
    leptos::mount::mount_to_body(app::App);
}
