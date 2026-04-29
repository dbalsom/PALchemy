#[cfg(feature = "csr")]
pub fn apply_settings(settings: palcore::AppSettings) {
    use wasm_bindgen::JsCast;

    let Some(window) = web_sys::window() else {
        return;
    };
    let Some(document) = window.document() else {
        return;
    };
    let Some(root) = document.document_element() else {
        return;
    };

    let _ = root.set_attribute("data-theme", settings.theme.as_str());
    let contrast = if settings.high_contrast { "high" } else { "standard" };
    let _ = root.set_attribute("data-contrast", contrast);
    let text_size = if settings.large_text { "large" } else { "standard" };
    let _ = root.set_attribute("data-text-size", text_size);

    if let Some(html_root) = root.dyn_ref::<web_sys::HtmlElement>() {
        let _ = html_root
            .style()
            .set_property("color-scheme", if settings.high_contrast { "dark" } else { "dark" });
    }
}

#[cfg(not(feature = "csr"))]
pub fn apply_settings(_settings: palcore::AppSettings) {}

#[cfg(feature = "csr")]
pub async fn sleep_ms(ms: i32) {
    let promise = js_sys::Promise::new(&mut |resolve, _| {
        if let Some(window) = web_sys::window() {
            let _ = window.set_timeout_with_callback_and_timeout_and_arguments_0(&resolve, ms);
        }
    });
    let _ = wasm_bindgen_futures::JsFuture::from(promise).await;
}

#[cfg(not(feature = "csr"))]
pub async fn sleep_ms(_ms: i32) {}
