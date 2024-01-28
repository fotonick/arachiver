use btleplug::Error as BtleplugError;
use btleplug::{
    api::{bleuuid::uuid_from_u16, CharPropFlags, Characteristic, Peripheral as _, WriteType},
    platform::Peripheral,
};
use chrono::{DateTime, Duration, Utc};
use futures::StreamExt;
use std::{fmt, u16, u8};
use unicode_segmentation::UnicodeSegmentation;
use uuid::{uuid, Uuid};

pub const ARANET4_SERVICE_UUID: Uuid = uuid_from_u16(0xfce0);
const ARANET4_CURRENT_READINGS_UUID: Uuid = uuid!("f0cd3001-95da-4f4b-9ac8-aa55d312af0c");
const ARANET4_NOTIFY_HISTORY_UUID: Uuid = uuid!("f0cd2003-95da-4f4b-9ac8-aa55d312af0c");
const ARANET4_COMMAND_UUID: Uuid = uuid!("f0cd1402-95da-4f4b-9ac8-aa55d312af0c");
const ARANET4_TOTAL_READINGS_UUID: Uuid = uuid!("f0cd2001-95da-4f4b-9ac8-aa55d312af0c");
const ARANET4_TIME_SINCE_UPDATE_UUID: Uuid = uuid!("f0cd2004-95da-4f4b-9ac8-aa55d312af0c");
const ARANET4_UPDATE_INTERVAL_UUID: Uuid = uuid!("f0cd2002-95da-4f4b-9ac8-aa55d312af0c");

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum DataType {
    Temperature = 1,
    Humidity = 2,
    Pressure = 3,
    CO2 = 4,
}

impl TryFrom<u8> for DataType {
    type Error = ();

    fn try_from(n: u8) -> Result<Self, Self::Error> {
        match n {
            1 => Ok(DataType::Temperature),
            2 => Ok(DataType::Humidity),
            3 => Ok(DataType::Pressure),
            4 => Ok(DataType::CO2),
            _ => Err(()),
        }
    }
}

impl DataType {
    const fn label(self) -> &'static str {
        match self {
            DataType::Temperature => "Temperature (°C)",
            DataType::Humidity => "Humidity (%)",
            DataType::Pressure => "Pressure (mbar)",
            DataType::CO2 => "CO₂ (ppm)",
        }
    }
    const fn multiplier(self) -> f32 {
        match self {
            DataType::Temperature => 0.05,
            DataType::Humidity => 1.0,
            DataType::Pressure => 0.1,
            DataType::CO2 => 1.0,
        }
    }

    const fn bytes_per_elem(self) -> usize {
        match self {
            DataType::Temperature => 2,
            DataType::Humidity => 1,
            DataType::Pressure => 2,
            DataType::CO2 => 2,
        }
    }

    const fn display_precision(self) -> usize {
        match self {
            DataType::Temperature => 2,
            DataType::Humidity => 0,
            DataType::Pressure => 1,
            DataType::CO2 => 0,
        }
    }
}

#[derive(Debug)]
pub struct CurrentSensorMeasurement {
    co2: u16,
    temperature: u16,
    pressure: u16,
    humidity: u8,
    battery: u8,
    status: u8,
    interval: u16,
    ago: u16,
}

impl From<&[u8]> for CurrentSensorMeasurement {
    fn from(item: &[u8]) -> Self {
        CurrentSensorMeasurement {
            co2: u16::from_le_bytes([item[0], item[1]]),
            temperature: u16::from_le_bytes([item[2], item[3]]),
            pressure: u16::from_le_bytes([item[4], item[5]]),
            humidity: item[6],
            battery: item[7],
            status: item[8],
            interval: u16::from_le_bytes([item[9], item[10]]),
            ago: u16::from_le_bytes([item[11], item[12]]),
        }
    }
}

impl fmt::Display for CurrentSensorMeasurement {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "CO₂: {:.*} ppm\nT: {:.*}°C\nP: {:.*} mbar\nHumidity: {:.*}%\nBattery: {}%\nStatus: {}\nInterval: {} s\nAgo: {} s\n",
            DataType::CO2.display_precision(),
            (self.co2 as f32) * DataType::CO2.multiplier(),
            DataType::Temperature.display_precision(),
            (self.temperature as f32) * DataType::Temperature.multiplier(),
            DataType::Pressure.display_precision(),
            (self.pressure as f32) * DataType::Pressure.multiplier(),
            DataType::Humidity.display_precision(),
            (self.humidity as f32) * DataType::Humidity.multiplier(),
            self.battery,
            self.status,
            self.interval,
            self.ago,
        )
    }
}

#[derive(Debug)]
struct HistoryResponseHeader {
    history_type: DataType,
    start_index: u16,    // v; 60 or 300; sampling interval?
    packet_num_elem: u8, // b; 195 or 2016; how many samples in memory
}

impl From<&[u8]> for HistoryResponseHeader {
    fn from(item: &[u8]) -> Self {
        HistoryResponseHeader {
            history_type: DataType::try_from(item[0]).unwrap(),
            start_index: u16::from_le_bytes([item[1], item[2]]),
            packet_num_elem: item[3],
        }
    }
}

impl fmt::Display for HistoryResponseHeader {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "history_type: {}\nstart_index: {}\npacket_num_elem: {}",
            self.history_type.label(),
            self.start_index,
            self.packet_num_elem
        )
    }
}

pub fn print_current_sensor_data(sensor_name: &str, measurement: &CurrentSensorMeasurement) {
    println!(
        "{}\n{}\n{}",
        sensor_name,
        "=".repeat(sensor_name.len()),
        measurement
    );
}

pub fn print_history<T>(data_type: DataType, data: &[T])
where
    f32: std::convert::From<T>,
    T: Copy,
{
    let label = data_type.label();
    let multiplier = data_type.multiplier();
    let precision = data_type.display_precision();
    print!(
        "{}\n{}\n[",
        label,
        "=".repeat(label.graphemes(true).count())
    );
    if !data.is_empty() {
        print!("{:.*}", precision, Into::<f32>::into(data[0]) * multiplier)
    }
    if data.len() > 1 {
        data[1..]
            .iter()
            .for_each(|x| print!(", {:.*}", precision, Into::<f32>::into(*x) * multiplier));
    }
    println!("]")
}

async fn get_total_readings(sensor: &Peripheral) -> Result<u16, BtleplugError> {
    let char = get_characteristic(sensor, ARANET4_TOTAL_READINGS_UUID).unwrap();
    let bytes = sensor.read(&char).await?;
    assert!(bytes.len() == 2, "Result of total readings is not 2 bytes");
    Ok(u16::from_le_bytes(bytes.try_into().unwrap()))
}

async fn get_time_since_update(sensor: &Peripheral) -> Result<u16, BtleplugError> {
    let char = get_characteristic(sensor, ARANET4_TIME_SINCE_UPDATE_UUID).unwrap();
    let bytes = sensor.read(&char).await?;
    assert!(bytes.len() == 2, "Result of total readings is not 2 bytes");
    Ok(u16::from_le_bytes(bytes.try_into().unwrap()))
}

async fn get_update_interval(sensor: &Peripheral) -> Result<u16, BtleplugError> {
    let char = get_characteristic(sensor, ARANET4_UPDATE_INTERVAL_UUID).unwrap();
    let bytes = sensor.read(&char).await?;
    assert!(bytes.len() == 2, "Result of total readings is not 2 bytes");
    Ok(u16::from_le_bytes(bytes.try_into().unwrap()))
}

pub async fn get_current_sensor_data(
    sensor: &Peripheral,
) -> Result<(String, CurrentSensorMeasurement), BtleplugError> {
    let local_name = sensor
        .properties()
        .await
        .expect("expect property result")
        .expect("expect some properties")
        .local_name
        .unwrap();

    // connect to the device
    sensor.connect().await?;

    // discover services and characteristics
    sensor.discover_services().await?;

    // instantaneous measurement for nice printing
    let co2_char = get_characteristic(sensor, ARANET4_CURRENT_READINGS_UUID).unwrap();
    let current_readings = sensor.read(&co2_char).await.unwrap();
    assert!(
        current_readings.len() == 13,
        "Unexpected current measurement length"
    );
    Ok((local_name, From::from(&current_readings[..])))
}

fn get_characteristic(sensor: &Peripheral, char_uuid: Uuid) -> Option<Characteristic> {
    let chars = sensor.characteristics();
    chars.iter().find(|c| c.uuid == char_uuid).cloned()
}

pub async fn get_history_bytes(
    sensor: &Peripheral,
    data_type: DataType,
) -> Result<Vec<u8>, BtleplugError> {
    // connect to the device
    sensor.connect().await?;

    // discover services and characteristics
    sensor.discover_services().await?;

    let subscribe_char = get_characteristic(sensor, ARANET4_NOTIFY_HISTORY_UUID).unwrap();
    let command_char = get_characteristic(sensor, ARANET4_COMMAND_UUID).unwrap();
    assert!(
        subscribe_char.properties.contains(CharPropFlags::NOTIFY),
        "No NOTIFY flag on subscribe characteristic!"
    );

    // Perform the arcane ritual
    let total_readings = get_total_readings(sensor).await?;
    let get_history_command_bytes: &[u8] = &[
        0x82,
        data_type as u8,
        0x00,
        0x00,
        0x01,
        0x00,
        (total_readings & 0xFF) as u8,
        (total_readings >> 8) as u8,
    ];
    sensor.unsubscribe(&subscribe_char).await?;
    sensor
        .write(
            &command_char,
            get_history_command_bytes,
            WriteType::WithResponse,
        )
        .await?;
    sensor.subscribe(&subscribe_char).await?;

    // Now get that sweet, sweet data
    let bytes_per_elem = data_type.bytes_per_elem();
    let mut notification_stream = sensor.notifications().await?;
    let mut history_bytes = Vec::new();
    while let Some(data) = notification_stream.next().await {
        assert!(
            data.uuid == ARANET4_NOTIFY_HISTORY_UUID,
            "Expected notification UUID to match ARANET4_NOTIFY_HISTORY_UUID"
        );
        let header = HistoryResponseHeader::from(&data.value[..4]);
        assert!(
            header.history_type == data_type,
            "History type doesn't match what we requested"
        );
        let bytes_end = 4 + bytes_per_elem * (header.packet_num_elem as usize);
        history_bytes.extend_from_slice(&data.value[4..bytes_end]);
        if history_bytes.len() >= bytes_per_elem * (total_readings as usize) {
            break;
        }
    }
    sensor.unsubscribe(&subscribe_char).await?;
    assert!(
        history_bytes.len() == bytes_per_elem * (total_readings as usize),
        "Received unexpected number of bytes"
    );
    Ok(history_bytes)
}

pub async fn get_history_u16(
    sensor: &Peripheral,
    data_type: DataType,
) -> Result<Vec<u16>, BtleplugError> {
    let history_bytes = get_history_bytes(sensor, data_type).await?;

    // Convert u8 to u16
    let history_data = history_bytes
        .chunks_exact(2)
        .map(|b| u16::from_le_bytes(b.try_into().unwrap()))
        .collect();
    Ok(history_data)
}

fn estimate_history_start_time(
    now: DateTime<Utc>,
    num_samples: u16,
    update_interval: u16,
    since_update: u16,
) -> DateTime<Utc> {
    now - Duration::seconds(
        (num_samples as i64 - 1) * (update_interval as i64) + (since_update as i64),
    )
}

pub async fn get_history_start_time(sensor: &Peripheral) -> Result<DateTime<Utc>, BtleplugError> {
    sensor.connect().await?;
    let num_samples = get_total_readings(sensor).await?;
    let update_interval = get_update_interval(sensor).await?;
    let since_update = get_time_since_update(sensor).await?;
    let now: DateTime<Utc> = Utc::now();
    Ok(estimate_history_start_time(
        now,
        num_samples,
        update_interval,
        since_update,
    ))
}
