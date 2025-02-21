use std::fs::File;

use btleplug::api::{Central, Manager as _};
use btleplug::platform::{Manager, Peripheral};
use chrono::Local;
use clap::{Arg, Command};
use color_eyre::eyre::{eyre, Error, Result};
use unicode_segmentation::UnicodeSegmentation;

mod csv_io;
mod device;
mod parquet_io;
mod types;
use crate::csv_io::save_history_csv;
use crate::device::{
    get_current_sensor_data, get_history, get_local_name, scan_for_sensor, DeviceInfo,
};
use crate::parquet_io::save_history_parquet;
use crate::types::CurrentSensorMeasurement;

fn cli() -> Command {
    Command::new("arachiver")
        .about("Aranet4 archiver")
        .subcommand_required(true)
        .arg_required_else_help(true)
        .arg(
            Arg::new("device_pattern")
                .short('d')
                .long("device")
                .default_value("Aranet")
                .required(false)
                .help("Select an Aranet4 device with <device_pattern> in its name; by default, the first device with 'Aranet' in its name will be used"),
        )
        .subcommand(Command::new("device_info").about("Print device information"))
        .subcommand(Command::new("readout").about("Print the current sensor readings to stdout"))
        .subcommand(Command::new("archive_history_csv").about("Save the full history to CSV"))
        .subcommand(
            Command::new("archive_history_parquet").about("Save the full history to Parquet"),
        )
}

fn print_device_info(info: &DeviceInfo) {
    println!(
        "{}\n{}\nModel number: {}\nSerial number: {}\nHardware revision: {}\nSoftware revision: {}\nManufacturer name: {}\nFirmware revision: {}",
        info.device_name,
        "=".repeat(info.device_name.graphemes(true).count()),
        info.model_number,
        info.serial_number,
        info.hardware_revision,
        info.software_revision,
        info.manufacturer_name,
        info.firmware_revision
    );
}

fn print_current_sensor_data(sensor_name: &str, measurement: &CurrentSensorMeasurement) {
    println!(
        "{}\n{}\n{}",
        sensor_name,
        "=".repeat(sensor_name.graphemes(true).count()),
        measurement
    );
}

async fn archive_history_csv(peripheral: &Peripheral) -> Result<String> {
    let local_name = get_local_name(&peripheral).await.unwrap();
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

async fn archive_history_parquet(peripheral: &Peripheral) -> Result<String> {
    let local_name = get_local_name(&peripheral).await.unwrap();
    let now = Local::now();
    let output_filename = format!(
        "{}_{}_history.parquet",
        now.to_rfc3339(),
        local_name.replace(" ", "_")
    );
    let mut output_file = File::create(&output_filename).expect(&format!(
        "Could not create writeable file {}",
        &output_filename
    ));
    let (ht, t, h, p, c) = get_history(&peripheral).await?;
    save_history_parquet(ht, t, h, p, c, &mut output_file).await?;
    Ok(output_filename)
}

#[tokio::main]
async fn main() -> Result<(), Error> {
    color_eyre::install()?;

    let matches = cli().get_matches();

    // use the first bluetooth adapter
    let central = {
        let manager = Manager::new().await.unwrap();
        let adapters = manager.adapters().await?;
        adapters.into_iter().next().unwrap()
    };
    let sensor = scan_for_sensor(
        &central,
        matches
            .get_one::<String>("device_pattern")
            .unwrap_or(&"Aranet".to_string()),
    )
    .await?;

    match matches.subcommand() {
        Some(("device_info", _sub_matches)) => {
            let info = DeviceInfo::read_from_sensor(&sensor).await?;
            print_device_info(&info);
        }
        Some(("readout", _sub_matches)) => {
            let (sensor_name, data) = get_current_sensor_data(&sensor).await?;
            print_current_sensor_data(&sensor_name, &data);
        }
        Some(("archive_history_csv", _sub_matches)) => {
            let fname = archive_history_csv(&sensor).await?;
            println!("Wrote {}", fname);
        }
        Some(("archive_history_parquet", _sub_matches)) => {
            let fname = archive_history_parquet(&sensor).await?;
            println!("Wrote {}", fname);
        }
        _ => {
            return Err(eyre!("Invalid subcommand"));
        }
    }
    central.stop_scan().await.unwrap();
    Ok(())
}
