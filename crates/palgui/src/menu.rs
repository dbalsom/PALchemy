use leptos::prelude::*;
use palcore::SelectedMode;

use crate::{controller, events, state::AppModel};

pub fn setup_menu_listeners(model: AppModel, connect_action: Action<String, ()>) {
    let log_writer = model.log_writer;
    events::listen_menu_about(move |_| {
        log_writer.info("Received About menu event.");
        model.show_about_modal.set(true);
    });

    let model_for_device_info = model.clone();
    let log_writer = model.log_writer;
    events::listen_menu_device_info(move |_| {
        log_writer.info("Received Device Info menu event.");
        model_for_device_info.show_device_modal.set(true);
    });

    let model_for_connect = model.clone();
    events::listen_menu_select_device(move |event| {
        model_for_connect.selected_device.set(Some(event.value.clone()));
        let _ = connect_action.dispatch(event.value);
    });

    let model_for_mode = model.clone();
    events::listen_menu_select_mode(move |event| {
        let mode = if event.value == "dump" {
            SelectedMode::Dump
        } else {
            SelectedMode::Interact
        };
        model_for_mode.selected_mode.set(mode);
        model_for_mode.reset_interaction_state();
    });

    let model_for_chip = model.clone();
    let log_writer = model.log_writer;
    events::listen_menu_select_chip(move |event| {
        let key = event.value.clone();
        let model = model_for_chip.clone();
        leptos::task::spawn_local(async move {
            match controller::load_chip_definition(&model, key.clone()).await {
                Ok(()) => log_writer.info(format!("Loaded {key}")),
                Err(error) => log_writer.error(format!("Failed to load chip: {error}")),
            }
        });
    });
}
