use std::{collections::HashMap, sync::Arc, time::Duration};

use palcore::{
    AppError, BackendEvent, ChipDef, ChipListItem, ConnectionFailureEvent, ConnectionState, DeviceInfo,
    InteractResponse, InteractiveStatus, PinCommand, PinMode, PinState, PinStateUpdateEvent, StatusResponse,
};
use palhal::{connect_device as hal_connect, GpioProvider};
use tauri::{async_runtime::JoinHandle, AppHandle, Emitter};
use tokio::sync::{oneshot, Mutex};

#[derive(Clone)]
pub struct AppState {
    chips: Arc<HashMap<String, ChipDef>>,
    session: Arc<Mutex<DeviceSession>>,
}

struct DeviceSession {
    device: Option<Box<dyn GpioProvider>>,
    selected_chip: Option<String>,
    pin_commands: HashMap<u8, PinCommand>,
    interactive_task: Option<InteractiveTask>,
    interactive_setup: Option<InteractiveSetup>,
}

struct InteractiveTask {
    stop_tx: oneshot::Sender<()>,
    join_handle: JoinHandle<()>,
}

struct InteractiveSetup {
    chip_name: String,
}

impl AppState {
    pub fn new(chips: HashMap<String, ChipDef>) -> Self {
        Self {
            chips: Arc::new(chips),
            session: Arc::new(Mutex::new(DeviceSession {
                device: None,
                selected_chip: None,
                pin_commands: HashMap::new(),
                interactive_task: None,
                interactive_setup: None,
            })),
        }
    }

    pub fn chip_list(&self) -> Vec<ChipListItem> {
        let mut list = self
            .chips
            .values()
            .map(|chip| ChipListItem {
                name: chip.name.clone(),
                model: chip.model.clone(),
                alias: chip.alias.clone(),
                source: chip.source.clone(),
            })
            .collect::<Vec<_>>();
        list.sort_by(|a, b| a.name.cmp(&b.name));
        list
    }

    pub fn chip(&self, name: &str) -> Option<ChipDef> {
        self.chips.get(name).cloned()
    }

    pub async fn status(&self) -> StatusResponse {
        self.reconcile_connection_state().await
    }

    pub async fn connect_device(&self, device_type: String) -> Result<String, AppError> {
        tracing::debug!("preparing device session for connection: {device_type}");
        self.stop_interactive().await?;

        let device = hal_connect(&device_type).await.map_err(|error| match error {
            palcore::GpioError::Device(message) if message.contains("Unknown or unsupported device type") => {
                AppError::UnsupportedDevice { device_type }
            }
            other => other.into(),
        })?;

        let mut session = self.session.lock().await;
        session.device = Some(device);
        session.selected_chip = None;
        session.pin_commands.clear();
        session.interactive_setup = None;
        Ok("Connected".to_string())
    }

    pub async fn disconnect_device(&self) -> Result<(), AppError> {
        self.stop_interactive().await?;

        let mut device = {
            let mut session = self.session.lock().await;
            session.selected_chip = None;
            session.pin_commands.clear();
            session.interactive_setup = None;
            session.device.take()
        };

        if let Some(device) = device.as_mut() {
            device.power_off().await?;
        }

        Ok(())
    }

    pub async fn device_info(&self) -> Option<DeviceInfo> {
        let session = self.session.lock().await;
        session.device.as_ref().map(|device| device.info())
    }

    pub async fn select_chip(&self, chip_name: String) -> Result<String, AppError> {
        if !self.chips.contains_key(&chip_name) {
            return Err(AppError::UnknownChip { name: chip_name });
        }

        self.stop_interactive().await?;
        self.power_off().await?;

        let mut session = self.session.lock().await;
        session.selected_chip = Some(chip_name.clone());
        session.pin_commands.clear();
        session.interactive_setup = None;
        Ok(format!("Selected {chip_name}"))
    }

    pub async fn interact_chip(
        &self,
        chip_name: String,
        pins: HashMap<u8, PinCommand>,
    ) -> Result<InteractResponse, AppError> {
        let mut session = self.session.lock().await;
        ensure_interactive_ready(&self.chips, &mut session, &chip_name).await?;
        poll_interactive_session(&self.chips, &mut session, &chip_name, pins).await
    }

    pub async fn set_interactive_commands(&self, pins: HashMap<u8, PinCommand>) -> Result<(), AppError> {
        let mut session = self.session.lock().await;
        session.pin_commands = pins;
        Ok(())
    }

    pub async fn start_interactive(&self, app: AppHandle, poll_hz: u16) -> Result<(), AppError> {
        {
            let mut session = self.session.lock().await;
            if session.interactive_task.is_some() {
                return Err(AppError::InteractiveAlreadyRunning);
            }
            if session.device.is_none() {
                return Err(AppError::DeviceNotConnected);
            }
            let Some(chip_name) = session.selected_chip.clone() else {
                return Err(AppError::InteractiveChipNotSelected);
            };
            ensure_interactive_ready(&self.chips, &mut session, &chip_name).await?;
        }

        let (stop_tx, mut stop_rx) = oneshot::channel();
        let chips = Arc::clone(&self.chips);
        let session = Arc::clone(&self.session);
        let join_handle = tauri::async_runtime::spawn(async move {
            let poll_period = Duration::from_secs_f64(1.0 / f64::from(poll_hz.max(1)));
            let mut interval = tokio::time::interval(poll_period);
            loop {
                tokio::select! {
                    _ = &mut stop_rx => {
                        break;
                    }
                    _ = interval.tick() => {
                        let result = {
                            let mut guard = session.lock().await;
                            if let Some(chip_name) = guard.selected_chip.clone() {
                                let commands = guard.pin_commands.clone();
                                poll_interactive_session(&chips, &mut guard, &chip_name, commands).await
                            } else {
                                Err(AppError::InteractiveChipNotSelected)
                            }
                        };

                        match result {
                            Ok(response) => {
                                let event = PinStateUpdateEvent {
                                    outputs: response.outputs,
                                };
                                let _ = app.emit("backend_event", BackendEvent::PinStateUpdate(event));
                            }
                            Err(error) => {
                                tracing::error!("interactive runner failed: {error}");
                                if is_disconnect_error(&error) {
                                    tracing::warn!("device disconnected unexpectedly during interactive mode");
                                    clear_disconnected_session(&session, false).await;
                                    crate::menu::sync_status(&app, &StatusResponse {
                                        connection: ConnectionState::Disconnected,
                                        interactive: InteractiveStatus::Stopped,
                                        selected_chip: None,
                                    });
                                } else {
                                    let _ = app.emit(
                                        "backend_event",
                                        BackendEvent::OperationFailure(ConnectionFailureEvent {
                                            title: "Interactive mode stopped".to_string(),
                                            message: error.to_string(),
                                            device_type: None,
                                        }),
                                    );
                                }
                                break;
                            }
                        }
                    }
                }
            }

            let mut device = {
                let mut guard = session.lock().await;
                guard.interactive_task = None;
                guard.device.take()
            };

            if let Some(device) = device.as_mut() {
                if let Err(error) = device.power_off().await {
                    tracing::error!("failed to power off device: {error}");
                }
            }

            let mut guard = session.lock().await;
            if guard.device.is_none() {
                guard.device = device;
            }
            let status = StatusResponse {
                connection: if guard.device.is_some() {
                    ConnectionState::Connected
                } else {
                    ConnectionState::Disconnected
                },
                interactive: InteractiveStatus::Stopped,
                selected_chip: guard.selected_chip.clone(),
            };
            drop(guard);
            crate::menu::sync_status(&app, &status);
        });

        let mut guard = self.session.lock().await;
        guard.interactive_task = Some(InteractiveTask { stop_tx, join_handle });
        Ok(())
    }

    pub async fn stop_interactive(&self) -> Result<(), AppError> {
        let task = {
            let mut session = self.session.lock().await;
            session.interactive_task.take()
        };

        if let Some(task) = task {
            let _ = task.stop_tx.send(());
            let _ = task.join_handle.await;
        }

        self.power_off().await
    }

    async fn power_off(&self) -> Result<(), AppError> {
        let mut session = self.session.lock().await;
        session.interactive_setup = None;
        let Some(device) = session.device.as_mut() else {
            return Ok(());
        };
        device.power_off().await?;
        Ok(())
    }

    async fn reconcile_connection_state(&self) -> StatusResponse {
        let disconnect_error = {
            let mut session = self.session.lock().await;
            if let Some(device) = session.device.as_mut() {
                device.check_connection().await.err()
            } else {
                None
            }
        };

        if let Some(error) = disconnect_error {
            tracing::warn!("device connection check failed: {error}");
            clear_disconnected_session(&self.session, true).await;
        }

        let session = self.session.lock().await;
        StatusResponse {
            connection: if session.device.is_some() {
                ConnectionState::Connected
            } else {
                ConnectionState::Disconnected
            },
            interactive: if session.interactive_task.is_some() {
                InteractiveStatus::Running
            } else {
                InteractiveStatus::Stopped
            },
            selected_chip: session.selected_chip.clone(),
        }
    }
}

async fn clear_disconnected_session(session: &Arc<Mutex<DeviceSession>>, abort_task: bool) {
    let task = {
        let mut guard = session.lock().await;
        guard.selected_chip = None;
        guard.pin_commands.clear();
        guard.interactive_setup = None;
        guard.device = None;
        guard.interactive_task.take()
    };

    if abort_task {
        if let Some(task) = task {
            task.join_handle.abort();
        }
    }
}

fn is_disconnect_error(error: &AppError) -> bool {
    match error {
        AppError::Hardware { message } | AppError::Internal { message } => {
            let message = message.to_ascii_lowercase();
            message.contains("device not found")
                || message.contains("usb timeout")
                || message.contains("usb error")
                || message.contains("i/o error")
                || message.contains("io error")
                || message.contains("entity not found")
                || message.contains("no such device")
        }
        AppError::DeviceNotConnected => true,
        _ => false,
    }
}

async fn ensure_interactive_ready(
    chips: &HashMap<String, ChipDef>,
    session: &mut DeviceSession,
    chip_name: &str,
) -> Result<(), AppError> {
    if session
        .interactive_setup
        .as_ref()
        .is_some_and(|setup| setup.chip_name == chip_name)
    {
        return Ok(());
    }

    let chip = chips.get(chip_name).ok_or_else(|| AppError::UnknownChip {
        name: chip_name.to_string(),
    })?;
    let device = session.device.as_mut().ok_or(AppError::DeviceNotConnected)?;

    let mut power_pins = Vec::new();
    let mut ground_pins = Vec::new();
    for (pin_str, definition) in &chip.pinout {
        if let Ok(pin_num) = pin_str.parse::<u8>() {
            match definition.pin_type {
                palcore::PinType::Power => power_pins.push(pin_num),
                palcore::PinType::Ground => ground_pins.push(pin_num),
                _ => {}
            }
        }
    }

    power_pins.sort_unstable();
    ground_pins.sort_unstable();
    let io_voltage = chip.interactive_io_voltage();

    tracing::debug!(
        "interactive setup for {}: package={:?} pins={} vcc_pins={:?} gnd_pins={:?} vcc={}V io={}V",
        chip.name,
        chip.package,
        chip.pins,
        power_pins,
        ground_pins,
        chip.voltage,
        io_voltage
    );

    device.set_package(chip.package, chip.pins).await?;
    tracing::debug!("set package for {} complete", chip.name);
    device.set_power_pins(&power_pins, &ground_pins, chip.voltage).await?;
    tracing::debug!(
        "applied power for {}: VCC {:?}, GND {:?}, {}V",
        chip.name,
        power_pins,
        ground_pins,
        chip.voltage
    );
    device.set_io_voltage(io_voltage).await?;
    tracing::debug!("applied IO reference for {}: {}V", chip.name, io_voltage);
    tokio::time::sleep(Duration::from_millis(10)).await;
    session.interactive_setup = Some(InteractiveSetup {
        chip_name: chip_name.to_string(),
    });
    Ok(())
}

async fn poll_interactive_session(
    chips: &HashMap<String, ChipDef>,
    session: &mut DeviceSession,
    chip_name: &str,
    pins: HashMap<u8, PinCommand>,
) -> Result<InteractResponse, AppError> {
    let chip = chips.get(chip_name).ok_or_else(|| AppError::UnknownChip {
        name: chip_name.to_string(),
    })?;
    let device = session.device.as_mut().ok_or(AppError::DeviceNotConnected)?;

    let mut pin_config = Vec::new();
    for (pin_num, command) in pins {
        let (mode, state) = match command {
            PinCommand::DriveHigh => (PinMode::Output, PinState::High),
            PinCommand::DriveLow => (PinMode::Output, PinState::Low),
            PinCommand::Read => (PinMode::Input, PinState::Z),
        };
        pin_config.push((pin_num as usize, mode, state));
    }

    pin_config.sort_unstable_by_key(|(pin, _, _)| *pin);
    device.set_gpios_config(&pin_config).await?;
    tracing::debug!("configured {} GPIO pins for {}", pin_config.len(), chip.name);

    let states = device.read_gpios().await?;
    let outputs: HashMap<u8, PinState> = states
        .into_iter()
        .enumerate()
        .map(|(index, state)| ((index + 1) as u8, state))
        .collect();

    tracing::debug!("read GPIO state for {} complete", chip.name);
    let output_snapshot = chip
        .pinout
        .iter()
        .filter_map(|(pin_str, definition)| {
            (definition.pin_type == palcore::PinType::Output)
                .then(|| {
                    let pin = pin_str.parse::<u8>().ok()?;
                    let state = outputs.get(&pin)?;
                    Some(format!("{pin}={state:?}"))
                })
                .flatten()
        })
        .collect::<Vec<_>>();
    tracing::debug!(
        "output state snapshot for {}: {}",
        chip.name,
        output_snapshot.join(", ")
    );

    Ok(InteractResponse { outputs })
}

#[cfg(test)]
mod tests {
    use super::*;

    use async_trait::async_trait;
    use palcore::{ConnectionType, DeviceCapabilities, GpioError, PackageType, PinDef, PinType, UsbSpeed};
    use std::sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    };

    #[derive(Clone)]
    struct FakeMetrics {
        power_off_count: Arc<AtomicUsize>,
        set_package_count: Arc<AtomicUsize>,
        set_power_count: Arc<AtomicUsize>,
        set_io_voltage_count: Arc<AtomicUsize>,
        set_gpio_config_count: Arc<AtomicUsize>,
        read_gpio_count: Arc<AtomicUsize>,
    }

    impl FakeMetrics {
        fn new() -> Self {
            Self {
                power_off_count: Arc::new(AtomicUsize::new(0)),
                set_package_count: Arc::new(AtomicUsize::new(0)),
                set_power_count: Arc::new(AtomicUsize::new(0)),
                set_io_voltage_count: Arc::new(AtomicUsize::new(0)),
                set_gpio_config_count: Arc::new(AtomicUsize::new(0)),
                read_gpio_count: Arc::new(AtomicUsize::new(0)),
            }
        }
    }

    struct FakeProvider {
        metrics: FakeMetrics,
        connection_checks_until_failure: Option<Arc<AtomicUsize>>,
    }

    #[async_trait]
    impl GpioProvider for FakeProvider {
        fn info(&self) -> DeviceInfo {
            DeviceInfo {
                name: "Fake".to_string(),
                num_pins: 24,
                capabilities: DeviceCapabilities::default(),
                connection_type: ConnectionType::Usb(UsbSpeed::Unknown),
                firmware_version: None,
                device_code: None,
                serial_number: None,
                manufacture_date: None,
                supply_voltage: None,
                additional_info: HashMap::new(),
            }
        }

        async fn set_package(&mut self, _package: PackageType, _num_pins: usize) -> Result<(), GpioError> {
            self.metrics.set_package_count.fetch_add(1, Ordering::SeqCst);
            Ok(())
        }

        async fn check_connection(&mut self) -> Result<(), GpioError> {
            if let Some(counter) = &self.connection_checks_until_failure {
                if counter.fetch_sub(1, Ordering::SeqCst) == 0 {
                    return Err(GpioError::Device("Device not found".into()));
                }
            }
            Ok(())
        }

        async fn set_power_pins(
            &mut self,
            _vcc_pins: &[u8],
            _gnd_pins: &[u8],
            _voltage_v: f32,
        ) -> Result<(), GpioError> {
            self.metrics.set_power_count.fetch_add(1, Ordering::SeqCst);
            Ok(())
        }

        async fn power_off(&mut self) -> Result<(), GpioError> {
            self.metrics.power_off_count.fetch_add(1, Ordering::SeqCst);
            Ok(())
        }

        async fn set_vpp_pins(&mut self, _pins: &[u8], _voltage_v: f32) -> Result<(), GpioError> {
            Ok(())
        }

        async fn set_io_voltage(&mut self, _voltage_v: f32) -> Result<(), GpioError> {
            self.metrics.set_io_voltage_count.fetch_add(1, Ordering::SeqCst);
            Ok(())
        }

        async fn set_gpios_config(&mut self, _pins: &[(usize, PinMode, PinState)]) -> Result<(), GpioError> {
            self.metrics.set_gpio_config_count.fetch_add(1, Ordering::SeqCst);
            Ok(())
        }

        async fn read_gpios(&mut self) -> Result<Vec<PinState>, GpioError> {
            self.metrics.read_gpio_count.fetch_add(1, Ordering::SeqCst);
            Ok(vec![PinState::Low; 24])
        }

        async fn set_clock_pins(&mut self, _pins: &[usize], _freq_hz: u32) -> Result<(), GpioError> {
            Ok(())
        }
    }

    fn test_chip() -> ChipDef {
        ChipDef {
            name: "PAL16L8".to_string(),
            model: "PAL16L8".to_string(),
            alias: None,
            source: None,
            model_description: "Test chip".to_string(),
            app_description: None,
            class: "PAL".to_string(),
            pins: 20,
            width: None,
            package: PackageType::DIP,
            voltage: 5.0,
            io_voltage: None,
            vpp_voltage: 0.0,
            pinout: HashMap::from([
                (
                    "1".to_string(),
                    PinDef {
                        pin_type: PinType::Input,
                        name: Some("I1".to_string()),
                        active_low: false,
                    },
                ),
                (
                    "10".to_string(),
                    PinDef {
                        pin_type: PinType::Ground,
                        name: Some("GND".to_string()),
                        active_low: false,
                    },
                ),
                (
                    "20".to_string(),
                    PinDef {
                        pin_type: PinType::Power,
                        name: Some("VCC".to_string()),
                        active_low: false,
                    },
                ),
            ]),
        }
    }

    async fn app_state_with_device() -> (AppState, FakeMetrics) {
        let metrics = FakeMetrics::new();
        let mut chips = HashMap::new();
        chips.insert("PAL16L8".to_string(), test_chip());
        let state = AppState::new(chips);

        let provider = FakeProvider {
            metrics: metrics.clone(),
            connection_checks_until_failure: None,
        };

        {
            let mut session = state.session.lock().await;
            session.device = Some(Box::new(provider));
            session.selected_chip = Some("PAL16L8".to_string());
            session.pin_commands.insert(1, PinCommand::DriveHigh);

            let (stop_tx, stop_rx) = oneshot::channel();
            let join_handle = tauri::async_runtime::spawn(async move {
                let _ = stop_rx.await;
            });
            session.interactive_task = Some(InteractiveTask { stop_tx, join_handle });
        }

        (state, metrics)
    }

    #[tokio::test]
    async fn disconnect_device_stops_interactive_and_clears_session() {
        let (state, power_off_count) = app_state_with_device().await;

        state.disconnect_device().await.unwrap();

        let status = state.status().await;
        assert_eq!(status.connection, ConnectionState::Disconnected);
        assert_eq!(status.interactive, InteractiveStatus::Stopped);
        assert_eq!(status.selected_chip, None);
        assert!(power_off_count.power_off_count.load(Ordering::SeqCst) >= 1);
    }

    #[tokio::test]
    async fn select_chip_stops_interactive_and_updates_selection() {
        let (state, power_off_count) = app_state_with_device().await;

        state.select_chip("PAL16L8".to_string()).await.unwrap();

        let status = state.status().await;
        assert_eq!(status.connection, ConnectionState::Connected);
        assert_eq!(status.interactive, InteractiveStatus::Stopped);
        assert_eq!(status.selected_chip.as_deref(), Some("PAL16L8"));
        assert!(power_off_count.power_off_count.load(Ordering::SeqCst) >= 1);
    }

    #[tokio::test]
    async fn status_clears_session_after_unexpected_disconnect() {
        let mut chips = HashMap::new();
        chips.insert("PAL16L8".to_string(), test_chip());
        let state = AppState::new(chips);

        {
            let mut session = state.session.lock().await;
            session.device = Some(Box::new(FakeProvider {
                metrics: FakeMetrics::new(),
                connection_checks_until_failure: Some(Arc::new(AtomicUsize::new(0))),
            }));
            session.selected_chip = Some("PAL16L8".to_string());
        }

        let status = state.status().await;
        assert_eq!(status.connection, ConnectionState::Disconnected);
        assert_eq!(status.interactive, InteractiveStatus::Stopped);
        assert_eq!(status.selected_chip, None);
    }

    #[tokio::test]
    async fn interactive_setup_runs_once_per_selected_chip() {
        let (state, metrics) = app_state_with_device().await;

        state.stop_interactive().await.unwrap();

        let pins = HashMap::from([(1, PinCommand::DriveHigh)]);
        state.interact_chip("PAL16L8".to_string(), pins.clone()).await.unwrap();
        state.interact_chip("PAL16L8".to_string(), pins).await.unwrap();

        assert_eq!(metrics.set_package_count.load(Ordering::SeqCst), 1);
        assert_eq!(metrics.set_power_count.load(Ordering::SeqCst), 1);
        assert_eq!(metrics.set_io_voltage_count.load(Ordering::SeqCst), 1);
        assert_eq!(metrics.set_gpio_config_count.load(Ordering::SeqCst), 2);
        assert_eq!(metrics.read_gpio_count.load(Ordering::SeqCst), 2);
    }

    #[tokio::test]
    async fn power_off_invalidates_cached_interactive_setup() {
        let (state, metrics) = app_state_with_device().await;

        state.stop_interactive().await.unwrap();

        let pins = HashMap::from([(1, PinCommand::DriveHigh)]);
        state.interact_chip("PAL16L8".to_string(), pins.clone()).await.unwrap();
        state.stop_interactive().await.unwrap();
        state.interact_chip("PAL16L8".to_string(), pins).await.unwrap();

        assert_eq!(metrics.set_package_count.load(Ordering::SeqCst), 2);
        assert_eq!(metrics.set_power_count.load(Ordering::SeqCst), 2);
        assert_eq!(metrics.set_io_voltage_count.load(Ordering::SeqCst), 2);
    }
}
