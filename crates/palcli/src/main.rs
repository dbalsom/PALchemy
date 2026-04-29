use palcore::{ChipDef, PinMode, PinState, PinType};
use palhal::{available_drivers, connect_device};
use tracing::{error, info};

#[derive(Debug, Clone, bpaf::Bpaf)]
#[bpaf(options)]
struct Options {
    /// List available hardware device drivers
    #[bpaf(short('l'), long("driverlist"), switch)]
    driverlist: bool,

    /// Print detailed information about the connected device
    #[bpaf(short('i'), long("deviceinfo"), switch)]
    deviceinfo: bool,

    /// Name of the device driver to connect to (e.g., t48)
    #[bpaf(long("device"))]
    device: Option<String>,

    /// Name of the chip to dump (must match a .toml definition)
    #[bpaf(short, long)]
    chip: Option<String>,

    /// Directory containing chip definition .toml files
    #[bpaf(short('d'), long, fallback("chips".to_string()))]
    chips_dir: String,
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();
    let opts = options().run();

    // Handle --driverlist
    if opts.driverlist {
        println!("Available Device Drivers:");
        for driver in available_drivers() {
            println!("  - {}", driver);
        }
        return;
    }

    // Connect to device if specified
    let dev_name = opts.device.unwrap_or_else(|| "T48".to_string());

    // We only connect if we actually need to do something with the device
    if !opts.deviceinfo && opts.chip.is_none() {
        println!("No operation specified. Try --help");
        return;
    }

    info!("Connecting to device abstractly: {}", dev_name);
    let mut device = match connect_device(&dev_name).await {
        Ok(d) => d,
        Err(e) => {
            error!("Failed to connect to device '{}': {}", dev_name, e);
            std::process::exit(1);
        }
    };

    // Handle --deviceinfo
    if opts.deviceinfo {
        let info = device.info();
        println!("Device: {}", info.name);
        println!("Max Pins: {}", info.num_pins);
        println!("Connection: {:?}", info.connection_type);
        if let Some(fw) = info.firmware_version {
            println!("Firmware: {}", fw);
        }
        if let Some(code) = info.device_code {
            println!("Device Code: {}", code);
        }
        if let Some(sn) = info.serial_number {
            println!("Serial Number: {}", sn);
        }
        if let Some(date) = info.manufacture_date {
            println!("Manufacture Date: {}", date);
        }
        if let Some(volts) = info.supply_voltage {
            println!("Supply Voltage: {:.2}V", volts);
        }
        return;
    }

    // Handle Chip dump
    if let Some(chip_name) = opts.chip {
        // Load chip definitions
        let chips = ChipDef::load_from_dir(&opts.chips_dir).unwrap_or_else(|e| {
            error!("Failed to load chip definitions from '{}': {}", opts.chips_dir, e);
            std::process::exit(1);
        });

        let chip = chips
            .iter()
            .find(|c| c.name == chip_name || c.alias.as_deref() == Some(&chip_name))
            .unwrap_or_else(|| {
                error!("Chip '{}' not found in definitions", chip_name);
                std::process::exit(1);
            });

        info!("Using chip: {} ({})", chip.name, chip.model_description);

        // Classify pins
        let mut vcc_pins = Vec::new();
        let mut gnd_pins = Vec::new();
        let mut inputs = Vec::new();
        let mut outputs = Vec::new();

        for (pin_num_str, pin_def) in &chip.pinout {
            if let Ok(pin) = pin_num_str.parse::<u8>() {
                match pin_def.pin_type {
                    PinType::Power => vcc_pins.push(pin),
                    PinType::Ground => gnd_pins.push(pin),
                    PinType::Input => inputs.push(pin),
                    PinType::Output => outputs.push(pin),
                    PinType::InputOutput => inputs.push(pin),
                    _ => {}
                }
            }
        }

        inputs.sort();
        outputs.sort();

        // Initialize hardware
        device.set_package(chip.package, chip.pins).await.unwrap();
        device.set_power_pins(&vcc_pins, &gnd_pins, chip.voltage).await.unwrap();

        // Run dump
        info!(
            "Starting combinatorial dump: {} inputs, {} outputs",
            inputs.len(),
            outputs.len()
        );

        let mut config = Vec::new();
        for &pin in &inputs {
            config.push((pin as usize, PinMode::Input, PinState::Z));
        }
        for &pin in &outputs {
            config.push((pin as usize, PinMode::Input, PinState::Z));
        }
        device.set_gpios_config(&config).await.unwrap();

        let num_inputs = inputs.len() as u32;
        for i in 0..(1u64 << num_inputs) {
            let mut pins_to_set = Vec::new();
            for (idx, &pin) in inputs.iter().enumerate() {
                let bit = (i >> idx) & 1;
                let state = if bit == 1 { PinState::High } else { PinState::Low };
                pins_to_set.push((pin as usize, PinMode::Output, state));
            }
            device.set_gpios_config(&pins_to_set).await.unwrap();

            let res = device.read_gpios().await.unwrap();

            // Print input vector and output states
            let out_str: String = outputs
                .iter()
                .map(|&p| match res[p as usize - 1] {
                    PinState::High => '1',
                    PinState::Low => '0',
                    PinState::Z => 'Z',
                })
                .collect();

            println!("{:0width$b} -> {}", i, out_str, width = inputs.len());
        }

        info!("Dump complete.");
    }
}
