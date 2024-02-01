use anyhow::{anyhow, Error as AnyhowError};
use btleplug::api::{Central, Manager as _, Peripheral as _, ScanFilter};
use btleplug::platform::{Manager, Peripheral};
use chrono::Duration;
use futures::future::join_all;

mod device;
use crate::device::*;
mod types;

async fn process_sensor(sensor: &Peripheral) {
    match get_current_sensor_data(sensor).await {
        Ok((local_name, data)) => print_current_sensor_data(&local_name, &data),
        Err(e) => eprintln!("Oh no: {}", e),
    };
    println!(
        "Computed start time = {}",
        get_history_start_time(sensor).await.unwrap()
    );
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
async fn main() -> Result<(), AnyhowError> {
    let manager = Manager::new().await.unwrap();

    // get the first bluetooth adapter
    let adapters = manager.adapters().await?;
    let central = adapters.into_iter().next().unwrap();

    // start scanning for devices
    central
        .start_scan(ScanFilter {
            services: vec![ARANET4_SERVICE_UUID],
        })
        .await?;
    // central.start_scan(ScanFilter::default()).await?;
    // instead of waiting, you can use central.events() to get a stream which will
    // notify you of new devices, for an example of that see examples/event_driven_discovery.rs
    tokio::time::sleep(Duration::seconds(3).to_std().unwrap()).await;

    // query devices concurrently
    let peripherals = central.peripherals().await.unwrap();
    if peripherals.is_empty() {
        return Err(anyhow!("No devices found"));
    }
    let mut tasks = Vec::new();
    for peripheral in peripherals {
        // tokio::time::timeout(Duration::from_millis(1000), peripheral.disconnect()).await?;
        let local_name = peripheral
            .properties()
            .await
            .expect("expect property result")
            .expect("expect some properties")
            .local_name;
        // if local_name.iter().any(|n| n.contains("Aranet4 1BA27")) {
        if local_name.iter().any(|n| n.contains("Aranet4")) {
            tasks.push(tokio::spawn(
                async move { process_sensor(&peripheral).await },
            ));
        }
    }
    join_all(tasks).await;

    central.stop_scan().await.unwrap();
    Ok(())
}
