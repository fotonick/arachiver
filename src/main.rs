use std::time::Duration;

use btleplug::api::{Central, Manager as _, ScanFilter};
use btleplug::platform::{Manager, Peripheral};
use color_eyre::eyre::{eyre, Error, Result};

mod csv_io;
mod device;
mod types;
use crate::csv_io::save_history_csv_all;
use crate::device::{
    disconnect_all, get_co2_history, get_current_sensor_data, get_humidity_history,
    get_pressure_history, get_temperature_history, print_current_sensor_data, ARANET4_SERVICE_UUID,
};

#[allow(dead_code)]
async fn process_sensor(sensor: &Peripheral) {
    match get_current_sensor_data(sensor).await {
        Ok((local_name, data)) => print_current_sensor_data(&local_name, &data),
        Err(e) => eprintln!("Oh no: {}", e),
    };
    match get_temperature_history(sensor).await {
        Ok(data) => println!("{}", data),
        Err(e) => eprintln!("Oh no: {}", e),
    };
    match get_humidity_history(sensor).await {
        Ok(data) => println!("{}", data),
        Err(e) => eprintln!("Oh no: {}", e),
    };
    match get_pressure_history(sensor).await {
        Ok(data) => println!("{}", data),
        Err(e) => eprintln!("Oh no: {}", e),
    };
    match get_co2_history(sensor).await {
        Ok(data) => println!("{}", data),
        Err(e) => eprintln!("Oh no: {}", e),
    };
}

#[tokio::main]
async fn main() -> Result<(), Error> {
    color_eyre::install()?;

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

    // query devices concurrently
    let peripherals = central.peripherals().await.unwrap();
    if peripherals.is_empty() {
        return Err(eyre!("No devices found"));
    }
    for fname in save_history_csv_all(&peripherals).await? {
        println!("Wrote {}", fname);
    }
    disconnect_all(&peripherals).await;
    central.stop_scan().await.unwrap();
    Ok(())
}
