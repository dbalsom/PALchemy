use std::collections::{HashMap, VecDeque};

use leptos::prelude::*;
use palcore::{
    AppSettings, ChipDef, ConnectionFailureEvent, ConnectionState, DeviceInfo, InteractiveStatus, LogEvent, LogLevel,
    LogSource, PinDirection, PinState, SelectedMode,
};

#[derive(Clone)]
pub struct LogEntry {
    pub level: LogLevel,
    pub source: LogSource,
    pub text: String,
}

#[derive(Clone, Copy)]
pub struct LogWriter {
    entries: RwSignal<VecDeque<LogEntry>>,
    max_entries: usize,
}

impl LogWriter {
    pub fn new(entries: RwSignal<VecDeque<LogEntry>>, max_entries: usize) -> Self {
        Self { entries, max_entries }
    }

    pub fn info(self, message: impl Into<String>) {
        self.push(LogLevel::Info, LogSource::Frontend, message.into());
    }

    pub fn error(self, message: impl Into<String>) {
        self.push(LogLevel::Error, LogSource::Frontend, message.into());
    }

    pub fn backend(self, event: LogEvent) {
        self.push(event.level, event.source, event.message);
    }

    pub fn history(self, events: impl IntoIterator<Item = LogEvent>) {
        self.entries.update(|entries| {
            for event in events {
                entries.push_back(LogEntry {
                    level: event.level,
                    source: event.source,
                    text: event.message,
                });
            }
            while entries.len() > self.max_entries {
                entries.pop_front();
            }
        });
    }

    fn push(self, level: LogLevel, source: LogSource, text: String) {
        self.entries.update(|entries| {
            entries.push_back(LogEntry { level, source, text });
            while entries.len() > self.max_entries {
                entries.pop_front();
            }
        });
    }
}

#[derive(Clone)]
pub struct AppModel {
    pub current_chip: RwSignal<Option<ChipDef>>,
    pub selected_mode: RwSignal<SelectedMode>,
    pub selected_device: RwSignal<Option<String>>,
    pub connection: RwSignal<ConnectionState>,
    pub interactive_status: RwSignal<InteractiveStatus>,
    pub pin_directions: RwSignal<HashMap<u8, PinDirection>>,
    pub pin_toggles: RwSignal<HashMap<u8, bool>>,
    pub output_states: RwSignal<HashMap<u8, PinState>>,
    pub show_device_modal: RwSignal<bool>,
    pub show_about_modal: RwSignal<bool>,
    pub show_error_modal: RwSignal<bool>,
    pub error_modal: RwSignal<Option<ConnectionFailureEvent>>,
    pub settings: RwSignal<AppSettings>,
    pub device_info: RwSignal<Option<DeviceInfo>>,
    pub log_entries: RwSignal<VecDeque<LogEntry>>,
    pub log_writer: LogWriter,
}

impl AppModel {
    pub fn new() -> Self {
        let log_entries = RwSignal::new(
            vec![LogEntry {
                level: LogLevel::Info,
                source: LogSource::Frontend,
                text: "Welcome to PALchemy. Connect to a device to begin.".to_string(),
            }]
            .into(),
        );

        Self {
            current_chip: RwSignal::new(None),
            selected_mode: RwSignal::new(SelectedMode::Interact),
            selected_device: RwSignal::new(None),
            connection: RwSignal::new(ConnectionState::Disconnected),
            interactive_status: RwSignal::new(InteractiveStatus::Stopped),
            pin_directions: RwSignal::new(HashMap::new()),
            pin_toggles: RwSignal::new(HashMap::new()),
            output_states: RwSignal::new(HashMap::new()),
            show_device_modal: RwSignal::new(false),
            show_about_modal: RwSignal::new(false),
            show_error_modal: RwSignal::new(false),
            error_modal: RwSignal::new(None),
            settings: RwSignal::new(AppSettings::default()),
            device_info: RwSignal::new(None),
            log_entries,
            log_writer: LogWriter::new(log_entries, 1000),
        }
    }

    pub fn reset_interaction_state(&self) {
        self.interactive_status.set(InteractiveStatus::Stopped);
        self.pin_toggles.set(HashMap::new());
        self.output_states.set(HashMap::new());
    }

    pub fn show_connection_failure(&self, event: ConnectionFailureEvent) {
        self.error_modal.set(Some(event));
        self.show_error_modal.set(true);
    }

    pub fn apply_chip(&self, chip: ChipDef) {
        let directions = chip
            .pinout
            .iter()
            .filter_map(|(pin, definition)| {
                let pin = pin.parse::<u8>().ok()?;
                let direction = match definition.pin_type {
                    palcore::PinType::Output => Some(PinDirection::Output),
                    palcore::PinType::Input | palcore::PinType::InputOutput => Some(PinDirection::Input),
                    _ => None,
                }?;
                Some((pin, direction))
            })
            .collect();

        self.current_chip.set(Some(chip));
        self.pin_directions.set(directions);
        self.pin_toggles.set(HashMap::new());
        self.output_states.set(HashMap::new());
        self.interactive_status.set(InteractiveStatus::Stopped);
    }
}
