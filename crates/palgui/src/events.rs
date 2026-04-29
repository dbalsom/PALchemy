#[cfg(feature = "csr")]
mod inner {
    use palcore::{BackendEvent, LogSeverityEvent, MenuSelectionEvent};
    use serde::de::DeserializeOwned;
    use wasm_bindgen::prelude::*;

    #[derive(serde::Deserialize)]
    struct EventEnvelope<T> {
        payload: T,
    }

    #[wasm_bindgen]
    extern "C" {
        #[wasm_bindgen(js_namespace = ["window", "__TAURI__", "event"], catch)]
        async fn listen(event: &str, handler: &Closure<dyn FnMut(JsValue)>) -> Result<JsValue, JsValue>;
    }

    fn register_listener<T>(event_name: &'static str, callback: impl Fn(T) + 'static)
    where
        T: DeserializeOwned + 'static,
    {
        let closure = Closure::wrap(Box::new(move |value: JsValue| {
            match serde_wasm_bindgen::from_value::<EventEnvelope<T>>(value) {
                Ok(event) => callback(event.payload),
                Err(error) => web_sys::console::error_1(&JsValue::from_str(&format!(
                    "failed to deserialize {event_name}: {error}"
                ))),
            }
        }) as Box<dyn FnMut(JsValue)>);

        wasm_bindgen_futures::spawn_local(async move {
            match listen(event_name, &closure).await {
                Ok(_) => closure.forget(),
                Err(error) => {
                    web_sys::console::error_2(&JsValue::from_str(&format!("failed to register {event_name}")), &error)
                }
            }
        });
    }

    pub fn listen_menu_select_device(callback: impl Fn(MenuSelectionEvent) + 'static) {
        register_listener("app_menu_select_device", callback);
    }

    pub fn listen_menu_select_chip(callback: impl Fn(MenuSelectionEvent) + 'static) {
        register_listener("app_menu_select_chip", callback);
    }

    pub fn listen_menu_select_mode(callback: impl Fn(MenuSelectionEvent) + 'static) {
        register_listener("app_menu_select_mode", callback);
    }

    pub fn listen_menu_about(callback: impl Fn(MenuSelectionEvent) + 'static) {
        register_listener("app_menu_about", callback);
    }

    pub fn listen_menu_device_info(callback: impl Fn(MenuSelectionEvent) + 'static) {
        register_listener("app_menu_device_info", callback);
    }

    pub fn listen_backend_event(callback: impl Fn(BackendEvent) + 'static) {
        register_listener("backend_event", callback);
    }

    pub fn listen_log_severity(callback: impl Fn(LogSeverityEvent) + 'static) {
        register_listener("app_set_log_severity", callback);
    }
}

#[cfg(feature = "csr")]
pub use inner::*;

#[cfg(not(feature = "csr"))]
mod stub {
    use palcore::{BackendEvent, LogSeverityEvent, MenuSelectionEvent};

    pub fn listen_menu_select_device(_callback: impl Fn(MenuSelectionEvent) + 'static) {}
    pub fn listen_menu_select_chip(_callback: impl Fn(MenuSelectionEvent) + 'static) {}
    pub fn listen_menu_select_mode(_callback: impl Fn(MenuSelectionEvent) + 'static) {}
    pub fn listen_menu_about(_callback: impl Fn(MenuSelectionEvent) + 'static) {}
    pub fn listen_menu_device_info(_callback: impl Fn(MenuSelectionEvent) + 'static) {}
    pub fn listen_backend_event(_callback: impl Fn(BackendEvent) + 'static) {}
    pub fn listen_log_severity(_callback: impl Fn(LogSeverityEvent) + 'static) {}
}

#[cfg(not(feature = "csr"))]
pub use stub::*;
