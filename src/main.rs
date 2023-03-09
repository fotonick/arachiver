use std::fmt;
use std::time::Duration;

use tokio::task::JoinSet;

use btleplug::Error as BtleplugError;
use btleplug::api::{Central, Manager as _, Peripheral as _, ScanFilter};
use btleplug::platform::{Manager, Peripheral};
use uuid::{Uuid, uuid};

const SENSOR_SERVICE_UUID: Uuid = uuid!("f0cd1400-95da-4f4b-9ac8-aa55d312af0c");
const ARANET4_CO2_MEASUREMENT_CHARACTERISTIC_UUID: Uuid = uuid!("f0cd1503-95da-4f4b-9ac8-aa55d312af0c");

#[derive(Debug)]
struct CO2Measurement {
    co2: u16,
    temperature: f32,
    pressure: f32,
    humidity: u8,
    battery: u8,
}

impl From<&[u8]> for CO2Measurement {
    fn from(item: &[u8]) -> Self {
        CO2Measurement {
            co2: u16::from_le_bytes([item[0], item[1]]),
            temperature: u16::from_le_bytes([item[2], item[3]]) as f32 / 20.0,
            pressure: u16::from_le_bytes([item[4], item[5]]) as f32 / 10.0,
            humidity: item[6],
            battery: item[7],
        }
    }
}

impl fmt::Display for CO2Measurement {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "CO₂: {} ppm\nT: {}°C\nP: {} mbar\nHumidity: {}%\nBattery: {}%\n",
            self.co2, self.temperature, self.pressure, self.humidity, self.battery)
    }
}

async fn get_sensor_data(sensor: &Peripheral) -> Result<(String, CO2Measurement), BtleplugError> {
    let local_name = sensor.properties().await.expect("expect property result").expect("expect some properties").local_name.unwrap();

    // connect to the device
    sensor.connect().await?;

    // discover services and characteristics
    sensor.discover_services().await?;

    // find the characteristic we want
    let chars = sensor.characteristics();
    let co2_char = chars.iter().find(|c| c.uuid == ARANET4_CO2_MEASUREMENT_CHARACTERISTIC_UUID).unwrap();

    // dance party
    let raw_c02_measurement = sensor.read(co2_char).await.unwrap();
    Ok((local_name, From::from(&raw_c02_measurement[..])))
}

fn print_sensor_data(sensor_name: &str, co2_measurement: CO2Measurement) {
    println!("{}\n{}\n{}", sensor_name, "=".repeat(sensor_name.len()), co2_measurement);
}

#[tokio::main]
async fn main() -> Result<(), BtleplugError> {
    let manager = Manager::new().await.unwrap();

    // get the first bluetooth adapter
    let adapters = manager.adapters().await?;
    let central = adapters.into_iter().nth(0).unwrap();

    // start scanning for devices
    central.start_scan(ScanFilter { services: vec![SENSOR_SERVICE_UUID] }).await?;
    // instead of waiting, you can use central.events() to get a stream which will
    // notify you of new devices, for an example of that see examples/event_driven_discovery.rs
    tokio::time::sleep(Duration::from_secs(2)).await;

    // query devices concurrently
    let mut set = JoinSet::new();
    for peripheral in central.peripherals().await.unwrap() {
        let local_name = peripheral.properties().await.expect("expect property result").expect("expect some properties").local_name;
        if local_name.iter().any(|n| n.contains("Aranet4")) {
            set.spawn(async move { get_sensor_data(&peripheral).await });
        }
    }

    // print serially as results are ready
    while let Some(Ok(Ok((local_name, data)))) = set.join_next().await {
        print_sensor_data(&local_name, data);
    }
    central.stop_scan().await.unwrap();
    Ok(())
}
