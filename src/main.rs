use std::fs::File;
use std::time::Duration;

use btleplug::api::{Central, Manager as _, ScanFilter};
use btleplug::platform::{Manager, Peripheral};
use chrono::Local;
use clap::Command;

use color_eyre::eyre::{eyre, Error, Result};

mod csv_io;
mod device;
mod types;
use crate::csv_io::save_history_csv;
use crate::device::{
    find_peripheral, get_current_sensor_data, get_history, get_local_name,
    print_current_sensor_data, ARANET4_SERVICE_UUID,
};

fn cli() -> Command {
    Command::new("arachiver")
        .about("Aranet4 archiver")
        .subcommand_required(true)
        .arg_required_else_help(true)
        .subcommand(Command::new("readout").about("Read out the current state"))
        .subcommand(Command::new("archive_history_csv").about("Read out the full history to CSV"))
}

pub async fn archive_history_csv(peripheral: &Peripheral) -> Result<String> {
    let local_name = get_local_name(&peripheral).await.unwrap(); // must be Ok to be found
    let now = Local::now();
    let output_filename = format!(
        "{}_{}_history.csv",
        now.to_rfc3339(),
        local_name.replace(" ", "_")
    );
    let mut output_file = File::create(&output_filename).expect(&format!(
        "Could not create writeable file {}",
        &output_filename
    ));
    let (ht, t, h, p, c) = get_history(&peripheral).await?;
    save_history_csv(ht, t, h, p, c, &mut output_file).await?;
    Ok(output_filename)
}

#[tokio::main]
async fn main() -> Result<(), Error> {
    color_eyre::install()?;

    let matches = cli().get_matches();

    // get the first bluetooth adapter
    let central = {
        let manager = Manager::new().await.unwrap();
        let adapters = manager.adapters().await?;
        adapters.into_iter().next().unwrap()
    };

    // start scanning for devices
    central
        .start_scan(ScanFilter {
            services: vec![ARANET4_SERVICE_UUID],
        })
        .await?;

    // Only look for devices for 3 seconds.
    // NB: Instead of waiting with a hard timeout, you can use central.events() to get a stream which will
    // notify you of new devices, for an example of that see examples/event_driven_discovery.rs
    tokio::time::sleep(Duration::from_secs(3)).await;
    let peripherals = central.peripherals().await.unwrap();
    if peripherals.is_empty() {
        return Err(eyre!("No devices found in the timeout period"));
    }
    let Some(sensor) = find_peripheral(&peripherals, "Aranet4").await else {
        return Err(eyre!("No devices matched selection 'Aranet4'"));
    };

    match matches.subcommand() {
        Some(("readout", _sub_matches)) => {
            let (sensor_name, data) = get_current_sensor_data(&sensor).await?;
            print_current_sensor_data(&sensor_name, &data);
        }
        Some(("archive_history_csv", _sub_matches)) => {
            let fname = archive_history_csv(&sensor).await?;
            println!("Wrote {}", fname);
        }
        _ => {
            return Err(eyre!("Invalid subcommand"));
        }
    }
    central.stop_scan().await.unwrap();
    Ok(())
}
