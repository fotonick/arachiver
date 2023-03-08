use std::error::Error;
use std::time::Duration;

use btleplug::api::{Central, Manager as _, Peripheral as _, ScanFilter};
use btleplug::platform::{Adapter, Manager, Peripheral};
use tokio::time;
use uuid::{Uuid, uuid};

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

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let manager = Manager::new().await.unwrap();

    // get the first bluetooth adapter
    let adapters = manager.adapters().await?;
    let central = adapters.into_iter().nth(0).unwrap();

    // start scanning for devices
    central.start_scan(ScanFilter::default()).await?;
    // instead of waiting, you can use central.events() to get a stream which will
    // notify you of new devices, for an example of that see examples/event_driven_discovery.rs
    tokio::time::sleep(Duration::from_secs(2)).await;

    // find the device we're interested in
    let sensor = find_sensor(&central).await.unwrap();
    central.stop_scan();

    // connect to the device
    sensor.connect().await?;

    // discover services and characteristics
    sensor.discover_services().await?;

    // find the characteristic we want
    let chars = sensor.characteristics();
    let co2_char = chars.iter().find(|c| c.uuid == ARANET4_CO2_MEASUREMENT_CHARACTERISTIC_UUID).unwrap();

    // dance party
    loop {
        let sleep_future = time::sleep(Duration::from_secs(300));
        let raw_c02_measurement = sensor.read(co2_char).await.unwrap();
        let co2_measurement: CO2Measurement = From::from(&raw_c02_measurement[..]);
        dbg!(co2_measurement);
        sleep_future.await;
    }
}

async fn find_sensor(central: &Adapter) -> Option<Peripheral> {
    for p in central.peripherals().await.unwrap() {
        if p.properties()
            .await
            .unwrap()
            .unwrap()
            .local_name
            .iter()
            .any(|name| name.contains("Aranet4"))
        {
            return Some(p);
        }
    }
    None
}