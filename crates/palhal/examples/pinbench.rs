use std::{
    env, process,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    time::Instant,
};

use palcore::{PackageType, PinMode, PinState};
use palhal::connect_device;

const IO_VOLTAGE_V: f32 = 3.3;
const BENCH_PIN: usize = 1;
const SOCKET_PINS: usize = 40;

#[tokio::main]
async fn main() {
    if let Err(error) = run().await {
        eprintln!("pinbench failed: {error}");
        process::exit(1);
    }
}

async fn run() -> Result<(), Box<dyn std::error::Error>> {
    let (driver, duration_secs) = parse_args()?;
    let mut device = connect_device(&driver).await?;

    println!("Connected to {}", device.info().name);
    println!("Benchmarking chip pin {BENCH_PIN} using DIP-{SOCKET_PINS} mapping at {IO_VOLTAGE_V:.1}V IO reference");
    if duration_secs == 0.0 {
        println!("Running until Ctrl-C");
    } else {
        println!("Running for {duration_secs:.3} seconds");
    }

    device.set_package(PackageType::DIP, SOCKET_PINS).await?;
    device.set_power_pins(&[], &[], 0.0).await?;
    device.set_io_voltage(IO_VOLTAGE_V).await?;

    let stop = Arc::new(AtomicBool::new(false));
    let stop_signal = Arc::clone(&stop);
    let ctrl_c_task = tokio::spawn(async move {
        let _ = tokio::signal::ctrl_c().await;
        stop_signal.store(true, Ordering::Relaxed);
    });

    let start = Instant::now();
    let mut transitions: u64 = 0;
    let mut state = PinState::Low;

    let bench_result = async {
        loop {
            if stop.load(Ordering::Relaxed) {
                break;
            }
            if duration_secs > 0.0 && start.elapsed().as_secs_f64() >= duration_secs {
                break;
            }

            state = match state {
                PinState::Low | PinState::Z => PinState::High,
                PinState::High => PinState::Low,
            };

            device.set_gpios_config(&[(BENCH_PIN, PinMode::Output, state)]).await?;
            transitions += 1;
        }

        Ok::<(), Box<dyn std::error::Error>>(())
    }
    .await;

    stop.store(true, Ordering::Relaxed);
    ctrl_c_task.abort();

    let power_off_result = device.power_off().await;
    bench_result?;
    power_off_result?;

    let elapsed_secs = start.elapsed().as_secs_f64();
    let toggle_rate_hz = if elapsed_secs > 0.0 {
        transitions as f64 / elapsed_secs
    } else {
        0.0
    };
    let square_wave_hz = toggle_rate_hz / 2.0;

    println!("Elapsed: {elapsed_secs:.6} s");
    println!("Transitions: {transitions}");
    println!("Toggle rate: {toggle_rate_hz:.2} edges/s");
    println!("Approx square-wave frequency: {square_wave_hz:.2} Hz");

    Ok(())
}

fn parse_args() -> Result<(String, f64), Box<dyn std::error::Error>> {
    let mut args = env::args().skip(1);
    let Some(driver) = args.next() else {
        print_usage();
        return Err("missing driver argument".into());
    };
    let Some(duration) = args.next() else {
        print_usage();
        return Err("missing duration_seconds argument".into());
    };
    if args.next().is_some() {
        print_usage();
        return Err("too many arguments".into());
    }

    let duration_secs: f64 = duration.parse()?;
    if duration_secs < 0.0 {
        return Err("duration_seconds must be >= 0".into());
    }

    Ok((driver, duration_secs))
}

fn print_usage() {
    eprintln!("Usage: cargo run -p palhal --example pinbench -- <driver> <duration_seconds>");
    eprintln!("Example: cargo run -p palhal --example pinbench -- t48 5");
    eprintln!("Use duration_seconds=0 to run until Ctrl-C");
}
