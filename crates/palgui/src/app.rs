use crate::{
    components::{
        about_modal::AboutModal, device_info_modal::DeviceInfoModal, dip_viewer::DipViewer, error_modal::ErrorModal,
        footer_status_bar::FooterStatusBar, log_viewer::LogViewer, mode_action_bar::ModeActionBar,
    },
    controller,
    state::AppModel,
};
use leptos::prelude::*;
use leptos_meta::{provide_meta_context, Title};
use leptos_router::{
    components::{Route, Router, Routes},
    StaticSegment,
};
use palcore::ConnectionState;
use stylance::import_style;

import_style!(style, "app.module.scss");

#[component]
pub fn App() -> impl IntoView {
    provide_meta_context();

    let model = AppModel::new();
    provide_context(model.clone());
    controller::bootstrap(model);

    view! {
        <Title text="PALchemy"/>

        <Router>
            <Routes fallback=|| "Page not found.".into_view()>
                <Route path=StaticSegment("") view=HomePage/>
                <Route path=StaticSegment("log") view=LogViewerPage/>
            </Routes>
        </Router>
    }
}

#[component]
fn HomePage() -> impl IntoView {
    let model = expect_context::<AppModel>();
    let connect_action = controller::create_connect_action(model.clone());
    let action_handler = controller::create_mode_action(model.clone());

    controller::install_home_bindings(model.clone(), connect_action);

    let action_disabled = Signal::derive({
        let model = model.clone();
        move || model.connection.get() != ConnectionState::Connected || model.current_chip.get().is_none()
    });

    view! {
        <>
            <div class=style::app_container>
                <main class=style::main_content>
                    <div class=style::center_panel>
                        <ModeActionBar
                            current_chip=model.current_chip
                            selected_mode=model.selected_mode
                            interactive_status=model.interactive_status
                            action_disabled=action_disabled
                            on_action=move |_| {
                                let _ = action_handler.dispatch(());
                            }
                        />

                        <DipViewer
                            chip=model.current_chip
                            settings=model.settings
                            pin_directions=model.pin_directions
                            pin_toggles=model.pin_toggles
                            output_states=model.output_states
                            interactive_status=model.interactive_status
                        />
                    </div>
                </main>

                <FooterStatusBar
                    connection=model.connection
                    device_info=model.device_info
                    show_device_modal=model.show_device_modal
                />
            </div>

            <DeviceInfoModal
                show_device_modal=model.show_device_modal
                device_info=model.device_info
            />
            <AboutModal show_about_modal=model.show_about_modal/>
            <ErrorModal
                show_error_modal=model.show_error_modal
                error_modal=model.error_modal
            />
        </>
    }
}

#[component]
fn LogViewerPage() -> impl IntoView {
    let model = expect_context::<AppModel>();

    view! {
        <div style="height: 100vh; display: flex; flex-direction: column;">
            <LogViewer log_entries=model.log_entries />
        </div>
    }
}
