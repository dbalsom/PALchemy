use std::collections::HashMap;

use leptos::prelude::*;
use palcore::{
    AppError, BackendEvent, BackendStatusEvent, ChipDef, ConnectionFailureEvent, ConnectionState, InteractiveStatus,
    PinCommand, PinDirection, SelectedMode, StatusResponse,
};

use crate::{events, ipc, menu, platform, state::AppModel};

const STATUS_POLL_MS: i32 = 2_000;

pub fn bootstrap(model: AppModel) {
    Effect::new({
        let model = model.clone();
        move |_| {
            platform::apply_settings(model.settings.get());
        }
    });

    let model_for_events = model.clone();
    events::listen_backend_event(move |event| {
        apply_backend_event(&model_for_events, event);
    });

    let log_writer = model.log_writer;
    leptos::task::spawn_local(async move {
        match ipc::get_log_history().await {
            Ok(events) => log_writer.history(events),
            Err(error) => log_writer.error(format!("Failed to load log history: {error}")),
        }
    });

    let model_for_settings = model.clone();
    leptos::task::spawn_local(async move {
        match ipc::get_settings().await {
            Ok(settings) => {
                model_for_settings.settings.set(settings);
            }
            Err(error) => {
                model_for_settings
                    .log_writer
                    .error(format!("Failed to load settings: {error}"));
            }
        }
    });

    start_status_poll(model);
}

pub fn create_connect_action(model: AppModel) -> Action<String, ()> {
    Action::new_local(move |device_type: &String| {
        let device_type = device_type.clone();
        let model = model.clone();
        async move {
            model.selected_device.set(Some(device_type.clone()));
            match ipc::connect_device(device_type.clone()).await {
                Ok(_) => {
                    model.log_writer.info(format!(
                        "Connect request accepted for {device_type}; awaiting backend confirmation."
                    ));
                }
                Err(error) => {
                    model.log_writer.error(format!("Connection failed: {error}"));
                    model.show_connection_failure(ConnectionFailureEvent {
                        title: "Device connection failed".to_string(),
                        message: error.to_string(),
                        device_type: Some(device_type),
                    });
                }
            }
        }
    })
}

pub fn create_mode_action(model: AppModel) -> Action<(), ()> {
    Action::new_local(move |_: &()| {
        let model = model.clone();
        async move {
            let Some(chip) = model.current_chip.get_untracked() else {
                return;
            };

            match model.selected_mode.get_untracked() {
                SelectedMode::Dump => run_dump(&model, chip).await,
                SelectedMode::Interact => run_interactive_toggle(&model, chip).await,
            }
        }
    })
}

pub fn install_home_bindings(model: AppModel, connect_action: Action<String, ()>) {
    menu::setup_menu_listeners(model.clone(), connect_action);

    Effect::new({
        let model = model.clone();
        move |_| {
            if model.interactive_status.get() != InteractiveStatus::Running {
                return;
            }

            let pins = build_pin_commands(&model.pin_directions.get(), &model.pin_toggles.get());
            leptos::task::spawn_local(async move {
                let _ = ipc::update_interactive_commands(pins).await;
            });
        }
    });

    Effect::new({
        let model = model.clone();
        move |_| {
            let mode = model.selected_mode.get();
            if mode != SelectedMode::Dump || model.interactive_status.get() != InteractiveStatus::Running {
                return;
            }

            let model_for_stop = model.clone();
            leptos::task::spawn_local(async move {
                if ipc::set_interactive_mode(false).await.is_err() {
                    model_for_stop
                        .log_writer
                        .error("Failed to request interactive mode stop after mode change.");
                }
            });

            model
                .log_writer
                .info("Requested interactive mode stop after switching to dump mode.");
        }
    });
}

pub async fn load_chip_definition(model: &AppModel, chip_name: String) -> Result<(), AppError> {
    match ipc::get_chip(chip_name.clone()).await? {
        Some(chip) => {
            model.apply_chip(chip);
            Ok(())
        }
        None => Err(AppError::UnknownChip { name: chip_name }),
    }
}

pub fn apply_backend_event(model: &AppModel, event: BackendEvent) {
    match event {
        BackendEvent::Status(event) => apply_backend_status(model, event),
        BackendEvent::ConnectionFailure(event) => apply_connection_failure(model, event),
        BackendEvent::OperationFailure(event) => apply_operation_failure(model, event),
        BackendEvent::SettingsUpdated(settings) => model.settings.set(settings),
        BackendEvent::Log(event) => model.log_writer.backend(event),
        BackendEvent::PinStateUpdate(event) => model.output_states.set(event.outputs),
    }
}

fn start_status_poll(model: AppModel) {
    leptos::task::spawn_local(async move {
        loop {
            match ipc::get_status().await {
                Ok(status) => apply_status_snapshot(&model, status, false),
                Err(error) => handle_status_poll_error(&model, error),
            }

            platform::sleep_ms(STATUS_POLL_MS).await;
        }
    });
}

fn apply_backend_status(model: &AppModel, event: BackendStatusEvent) {
    apply_status_snapshot(model, event.status, true);
}

fn apply_connection_failure(model: &AppModel, event: ConnectionFailureEvent) {
    model.connection.set(ConnectionState::Disconnected);
    model.interactive_status.set(InteractiveStatus::Stopped);
    model.device_info.set(None);
    model.reset_interaction_state();
    model
        .log_writer
        .error(format!("Backend reported connection failure: {}", event.message));
    model.show_connection_failure(event);
}

fn apply_operation_failure(model: &AppModel, event: ConnectionFailureEvent) {
    model
        .log_writer
        .error(format!("Backend reported operation failure: {}", event.message));
    model.show_connection_failure(event);
}

fn apply_status_snapshot(model: &AppModel, status: StatusResponse, log_transition: bool) {
    let previous_connection = model.connection.get_untracked();
    let previous_interactive = model.interactive_status.get_untracked();

    model.connection.set(status.connection);
    model.interactive_status.set(status.interactive);

    if log_transition && previous_connection != status.connection {
        match status.connection {
            ConnectionState::Connected => model.log_writer.info("Backend confirmed device connection."),
            ConnectionState::Disconnected => model.log_writer.info("Backend confirmed device disconnection."),
        }
    }

    if log_transition && previous_interactive != status.interactive {
        match status.interactive {
            InteractiveStatus::Running => model.log_writer.info("Backend confirmed interactive mode start."),
            InteractiveStatus::Stopped => model.log_writer.info("Backend confirmed interactive mode stop."),
        }
    }

    match status.connection {
        ConnectionState::Connected => {
            if previous_connection != ConnectionState::Connected || model.device_info.get_untracked().is_none() {
                refresh_device_info(model.clone());
            }
        }
        ConnectionState::Disconnected => {
            model.device_info.set(None);
            model.reset_interaction_state();
        }
    }
}

fn handle_status_poll_error(model: &AppModel, error: AppError) {
    if model.connection.get_untracked() == ConnectionState::Connected {
        model.log_writer.error(format!("Lost connection to backend: {error}"));
    }
    model.connection.set(ConnectionState::Disconnected);
    model.interactive_status.set(InteractiveStatus::Stopped);
    model.device_info.set(None);
    model.reset_interaction_state();
}

fn refresh_device_info(model: AppModel) {
    leptos::task::spawn_local(async move {
        match ipc::get_device_info().await {
            Ok(info) => {
                model.device_info.set(info);
            }
            Err(error) => {
                model
                    .log_writer
                    .error(format!("Failed to refresh device info: {error}"));
            }
        }
    });
}

async fn run_dump(model: &AppModel, chip: ChipDef) {
    if let Err(error) = ipc::select_chip(chip.name.clone()).await {
        model
            .log_writer
            .error(format!("Hardware initialization failed: {error}"));
        return;
    }

    model.log_writer.info(format!("Starting dump of {}...", chip.name));
    match ipc::dump_chip(chip.name.clone()).await {
        Ok(data) => {
            let vectors = data.vectors.len();
            model
                .log_writer
                .info(format!("Dump complete. Evaluated {vectors} combinations."));
        }
        Err(error) => {
            model.log_writer.error(format!("Dump failed: {error}"));
        }
    }
}

async fn run_interactive_toggle(model: &AppModel, chip: ChipDef) {
    if model.interactive_status.get_untracked() == InteractiveStatus::Running {
        match ipc::set_interactive_mode(false).await {
            Ok(()) => {
                model
                    .log_writer
                    .info("Interactive mode stop requested; awaiting backend confirmation.");
            }
            Err(error) => {
                model
                    .log_writer
                    .error(format!("Failed to stop interactive mode: {error}"));
            }
        }
        return;
    }

    if let Err(error) = ipc::select_chip(chip.name.clone()).await {
        model
            .log_writer
            .error(format!("Hardware initialization failed: {error}"));
        return;
    }

    let commands = build_pin_commands(
        &model.pin_directions.get_untracked(),
        &model.pin_toggles.get_untracked(),
    );
    if let Err(error) = ipc::update_interactive_commands(commands).await {
        model
            .log_writer
            .error(format!("Failed to stage interactive pin commands: {error}"));
        return;
    }

    match ipc::set_interactive_mode(true).await {
        Ok(()) => {
            model
                .log_writer
                .info("Interactive mode start requested; awaiting backend confirmation.");
        }
        Err(error) => {
            model
                .log_writer
                .error(format!("Failed to start interactive mode: {error}"));
        }
    }
}

fn build_pin_commands(
    pin_directions: &HashMap<u8, PinDirection>,
    pin_toggles: &HashMap<u8, bool>,
) -> HashMap<u8, PinCommand> {
    pin_directions
        .iter()
        .map(|(&pin, direction)| {
            let command = match direction {
                PinDirection::Input => {
                    if pin_toggles.get(&pin).copied().unwrap_or(false) {
                        PinCommand::DriveHigh
                    } else {
                        PinCommand::DriveLow
                    }
                }
                PinDirection::Output => PinCommand::Read,
            };
            (pin, command)
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn build_pin_commands_maps_inputs_and_outputs() {
        let commands = build_pin_commands(
            &HashMap::from([(1, PinDirection::Input), (2, PinDirection::Output)]),
            &HashMap::from([(1, true)]),
        );

        assert_eq!(commands.get(&1), Some(&PinCommand::DriveHigh));
        assert_eq!(commands.get(&2), Some(&PinCommand::Read));
    }
}
