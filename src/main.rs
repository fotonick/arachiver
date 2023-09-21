use std::fmt;
use std::time::Duration;

use anyhow::{Error as AnyhowError, anyhow};
use btleplug::Error as BtleplugError;
use btleplug::api::{Central, Manager as _, Peripheral as _, ScanFilter, bleuuid::uuid_from_u16, Characteristic, WriteType};
use btleplug::platform::{Manager, Peripheral};
use futures::future::join_all;
use uuid::{Uuid, uuid};

const ARANET4_SERVICE_UUID: Uuid = uuid_from_u16(0xfce0);
const ARANET4_CO2_MEASUREMENT_CHARACTERISTIC_UUID: Uuid = uuid!("f0cd1503-95da-4f4b-9ac8-aa55d312af0c");
const ARANET4_CO2_LOGS_CHARACTERISTIC_UUID: Uuid = uuid!("f0cd2005-95da-4f4b-9ac8-aa55d312af0c");
const ARANET4_MEASUREMENT_INTERVAL_UUID: Uuid = uuid!("f0cd2002-95da-4f4b-9ac8-aa55d312af0c");
const ARANET_SET_INTERVAL_UUID: Uuid = uuid!("f0cd1402-95da-4f4b-9ac8-aa55d312af0c");

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

#[derive(Debug, PartialEq)]
enum LogStatus {
    ErrorInArguments,
    NothingNew,
    Four,
    ReadInProgress,
}

impl TryFrom<u8> for LogStatus {
    type Error = BtleplugError;
    fn try_from(value: u8) -> Result<LogStatus, Self::Error> {
        match value {
            0 => Ok(LogStatus::ErrorInArguments),
            3 => Ok(LogStatus::NothingNew),
            4 => Ok(LogStatus::Four),
            129 => Ok(LogStatus::ReadInProgress),
            _ => Err(BtleplugError::Other(format!("invalid log status {}", value).into())),
        }
    }
}

#[derive(Debug)]
struct HistoryResponseHeader {
    code: LogStatus,  // f; 0 == error in parameters; 129 == reading in progress; maybe what kind of data we're looking at?; 3 == humidity; 4 == C02
    interval_s: u16,  // v; 60 or 300; sampling interval?
    total_samples: u16,  // b; 195 or 2016; how many samples in memory
    time_since_last_s: u16,  // x; 21 or 10 or 4; how long since last sample
    start_index: u16,  // I; 4620 or 1804 or 1687; start index of current packet
    packet_num_elements: u8,  // R; elements in current packet

    // s == current time in seconds
    // S = I + C == current sample out of total
    // T = s - (x + b * v) == start time
    // L[h] is an array of scale factors (h is some lookup of f)
    // P = value
    // U = P * L[h] == transformed value
    // n == output array
    // k == UUID of characteristic
}

impl From<&[u8]> for HistoryResponseHeader {
    fn from(item: &[u8]) -> Self {
        HistoryResponseHeader {
            code: LogStatus::try_from(item[0]).unwrap(),
            interval_s: u16::from_le_bytes([item[1], item[2]]),
            total_samples: u16::from_le_bytes([item[3], item[4]]),
            time_since_last_s: u16::from_le_bytes([item[5], item[6]]),
            start_index: u16::from_le_bytes([item[7], item[8]]),
            packet_num_elements: item[9],
        }
    }
}

impl fmt::Display for HistoryResponseHeader {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "code: {:?}\ninterval_s: {}\ntotal_samples: {}\ntime_since_last_s: {}\nstart_index: {}\npacket_num_elements: {}\n",
            self.code, self.interval_s, self.total_samples, self.time_since_last_s, self.start_index, self.packet_num_elements)
    }
}

async fn read_log(sensor: &Peripheral) -> Result<(HistoryResponseHeader, Vec<u8>), BtleplugError> {
    let chars = sensor.characteristics();
    let log_char = chars.iter().find(|c| c.uuid == ARANET4_CO2_LOGS_CHARACTERISTIC_UUID).unwrap();
    let mut log_data = sensor.read(log_char).await.unwrap();
    let mut header: HistoryResponseHeader = From::from(&log_data[..10]);
    while header.code == LogStatus::ReadInProgress {
        println!("reading in progress. schedule reading logs after 1 second");
        tokio::time::sleep(Duration::from_secs(1)).await;
        log_data = sensor.read(log_char).await.unwrap();
    }
    if header.code == LogStatus::ErrorInArguments {
        return Err(BtleplugError::Other(format!("Invalid argument").into()));
    }
    while header.packet_num_elements != 0 {
        header = From::from(&log_data[..10]);
        log_data = sensor.read(log_char).await.unwrap();
    }
    let payload = log_data.split_off(10);  // copy all but the first ten bytes
    Ok((header, payload))
}

async fn get_current_sensor_data(sensor: &Peripheral) -> Result<(String, CO2Measurement), BtleplugError> {
    let local_name = sensor.properties().await.expect("expect property result").expect("expect some properties").local_name.unwrap();

    // connect to the device
    sensor.connect().await?;

    // discover services and characteristics
    sensor.discover_services().await?;

    // find the characteristic we want
    let chars = sensor.characteristics();

    // instantaneous measurement for nice printing
    let co2_char = chars.iter().find(|c| c.uuid == ARANET4_CO2_MEASUREMENT_CHARACTERISTIC_UUID).unwrap();
    let raw_c02_measurement = sensor.read(co2_char).await.unwrap();
    Ok((local_name, From::from(&raw_c02_measurement[..])))
}

async fn update_sensor_log(sensor: &Peripheral) -> Result<(String, Vec<u8>), BtleplugError> {
    let local_name = sensor.properties().await.expect("expect property result").expect("expect some properties").local_name.unwrap();

    // connect to the device
    sensor.connect().await?;

    // discover services and characteristics
    sensor.discover_services().await?;

    // historical data on device
    let (header, log_data) = read_log(sensor).await?;
    println!("header\n======\n{}\n", header);
    Ok((local_name, log_data))
}

async fn get_sensor_log(sensor: &Peripheral) -> Result<(String, Vec<u8>), BtleplugError> {
    let local_name = sensor.properties().await.expect("expect property result").expect("expect some properties").local_name.unwrap();

    // connect to the device
    sensor.connect().await?;

    // discover services and characteristics
    sensor.discover_services().await?;

    // find the characteristic we want
    let chars = sensor.characteristics();

    // TODO: Attempt to set measurement interval to reset the devices internal knowledge of what
    //       has previously been queried. This isn't yet right.
    let get_interval_char = chars.iter().find(|c| c.uuid == ARANET4_MEASUREMENT_INTERVAL_UUID).unwrap();
    let set_interval_char = chars.iter().find(|c| c.uuid == ARANET_SET_INTERVAL_UUID).unwrap();
    let interval_date = sensor.read(get_interval_char).await?;
    sensor.write(set_interval_char, &interval_date, WriteType::WithResponse).await?;

    let (header, log_data) = read_log(sensor).await?;
    println!("header\n======\n{}\n", header);
    Ok((local_name, log_data))
}

fn print_sensor_data(sensor_name: &str, co2_measurement: CO2Measurement) {
    println!("{}\n{}\n{}", sensor_name, "=".repeat(sensor_name.len()), co2_measurement);
}

fn print_log_data(sensor_name: &str, data: &[u8]) {
    println!("{}\n{}\n{:?}", sensor_name, "=".repeat(sensor_name.len()), data);
}
async fn process_sensor(sensor: &Peripheral) -> () {
    match get_current_sensor_data(&sensor).await {
        Ok((local_name, data)) => print_sensor_data(&local_name, data),
        Err(e) => eprintln!("Oh no: {}", e),
    };
    match get_sensor_log(&sensor).await {
        Ok((local_name, data)) => print_log_data(&local_name, &data),
        Err(e) => eprintln!("Oh no: {}", e),
    };
}

#[tokio::main]
async fn main() -> Result<(), AnyhowError> {
    let manager = Manager::new().await.unwrap();

    // get the first bluetooth adapter
    let adapters = manager.adapters().await?;
    let central = adapters.into_iter().nth(0).unwrap();

    // start scanning for devices
    central.start_scan(ScanFilter { services: vec![ARANET4_SERVICE_UUID] }).await?;
    // central.start_scan(ScanFilter::default()).await?;
    // instead of waiting, you can use central.events() to get a stream which will
    // notify you of new devices, for an example of that see examples/event_driven_discovery.rs
    tokio::time::sleep(Duration::from_secs(2)).await;

    // query devices concurrently
    let peripherals = central.peripherals().await.unwrap();
    if peripherals.is_empty() {
        return Err(anyhow!("No devices found"));
    }
    let mut tasks = Vec::new();
    for peripheral in peripherals {
        // tokio::time::timeout(Duration::from_millis(1000), peripheral.disconnect()).await?;
        let local_name = peripheral.properties().await.expect("expect property result").expect("expect some properties").local_name;
        // if local_name.iter().any(|n| n.contains("Aranet4 1BA27")) {
        if local_name.iter().any(|n| n.contains("Aranet4")) {
            tasks.push(tokio::spawn(async move { process_sensor(&peripheral).await }));
        }
    }
    join_all(tasks).await;

    central.stop_scan().await.unwrap();
    Ok(())
}
