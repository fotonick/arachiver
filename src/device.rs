use btleplug::Error as BtleplugError;
use btleplug::{
    api::{bleuuid::uuid_from_u16, CharPropFlags, Characteristic, Peripheral as _, WriteType},
    platform::Peripheral,
};
use chrono::{DateTime, Duration, Utc};
use futures::StreamExt;
use std::mem::size_of;
use std::vec::Vec;
use std::{fmt, u16, u8};
use thiserror::Error;
use unicode_segmentation::UnicodeSegmentation;
use uuid::{uuid, Uuid};

pub const ARANET4_SERVICE_UUID: Uuid = uuid_from_u16(0xfce0);
const ARANET4_CURRENT_READINGS_UUID: Uuid = uuid!("f0cd3001-95da-4f4b-9ac8-aa55d312af0c");
const ARANET4_NOTIFY_HISTORY_UUID: Uuid = uuid!("f0cd2003-95da-4f4b-9ac8-aa55d312af0c");
const ARANET4_COMMAND_UUID: Uuid = uuid!("f0cd1402-95da-4f4b-9ac8-aa55d312af0c");
const ARANET4_TOTAL_READINGS_UUID: Uuid = uuid!("f0cd2001-95da-4f4b-9ac8-aa55d312af0c");
const ARANET4_TIME_SINCE_UPDATE_UUID: Uuid = uuid!("f0cd2004-95da-4f4b-9ac8-aa55d312af0c");
const ARANET4_UPDATE_INTERVAL_UUID: Uuid = uuid!("f0cd2002-95da-4f4b-9ac8-aa55d312af0c");

#[derive(Error, Debug)]
pub enum Aranet4Error {
    #[error("There was a Bluetooth error")]
    Btleplug {
        #[from]
        source: BtleplugError,
    },
    #[error("Aranet returned a response that didn't match our expectations")]
    InvalidResponse(String),
    #[error("Did not find requested characteristic")]
    CharacteristicNotFound,
}

#[derive(Debug)]
pub struct SensorData<Storage, const SENSORTYPE: u8> {
    pub values: Vec<Storage>,
}

pub trait Metadata {
    const DISPLAY_MULTIPLIER: f32;
    const DISPLAY_PRECISION: usize;
    fn label(&self) -> &'static str;
}

const TEMPERATURE: u8 = 1;
const HUMIDITY: u8 = 2;
const PRESSURE: u8 = 3;
const CO2: u8 = 4;

type TemperatureData = SensorData<u16, TEMPERATURE>;
type HumidityData = SensorData<u8, HUMIDITY>;
type PressureData = SensorData<u16, PRESSURE>;
type CO2Data = SensorData<u16, CO2>;

impl Metadata for TemperatureData {
    const DISPLAY_MULTIPLIER: f32 = 0.05;
    const DISPLAY_PRECISION: usize = 2;
    fn label(&self) -> &'static str {
        "Temperature (°C)"
    }
}

impl Metadata for HumidityData {
    const DISPLAY_MULTIPLIER: f32 = 1.0;
    const DISPLAY_PRECISION: usize = 0;
    fn label(&self) -> &'static str {
        "Humidity (%)"
    }
}

impl Metadata for PressureData {
    const DISPLAY_MULTIPLIER: f32 = 0.1;
    const DISPLAY_PRECISION: usize = 1;
    fn label(&self) -> &'static str {
        "Pressure (mbar)"
    }
}

impl Metadata for CO2Data {
    const DISPLAY_MULTIPLIER: f32 = 1.0;
    const DISPLAY_PRECISION: usize = 0;
    fn label(&self) -> &'static str {
        "CO₂ (ppm)"
    }
}

impl<const T: u8> TryFrom<&[u8]> for SensorData<u16, T> {
    type Error = Aranet4Error;
    fn try_from(bytes: &[u8]) -> Result<Self, Self::Error> {
        if bytes.len() % 2 == 0 {
            Ok(Self {
                values: bytes
                    .chunks_exact(2)
                    .map(|x| u16::from_le_bytes([x[0], x[1]]))
                    .collect(),
            })
        } else {
            Err(Aranet4Error::InvalidResponse(
                "expected an even number of bytes".to_string(),
            ))
        }
    }
}

impl<const T: u8> TryFrom<&[u8]> for SensorData<u8, T> {
    type Error = Aranet4Error;
    fn try_from(bytes: &[u8]) -> Result<Self, Self::Error> {
        Ok(Self {
            values: bytes.to_vec(),
        })
    }
}

impl<Storage, const SENSORTYPE: u8> fmt::Display for SensorData<Storage, SENSORTYPE>
where
    f32: From<Storage>,
    SensorData<Storage, SENSORTYPE>: Metadata,
    Storage: Copy,
{
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let mut result = write!(f, "[");
        if !self.values.is_empty() {
            result = result.and(write!(
                f,
                "{:.*}",
                Self::DISPLAY_PRECISION,
                f32::from(self.values[0]) * Self::DISPLAY_MULTIPLIER
            ));
        }
        if self.values.len() > 1 {
            self.values[1..].iter().for_each(|x| {
                result = result.and(write!(
                    f,
                    ", {:.*}",
                    Self::DISPLAY_PRECISION,
                    f32::from(*x) * Self::DISPLAY_MULTIPLIER
                ));
            });
        }
        result.and(write!(f, "]"))
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

impl From<[u8; 13]> for CurrentSensorMeasurement {
    fn from(item: [u8; 13]) -> Self {
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
            CO2Data::DISPLAY_PRECISION,
            (self.co2 as f32) * CO2Data::DISPLAY_MULTIPLIER,
            TemperatureData::DISPLAY_PRECISION,
            (self.temperature as f32) * TemperatureData::DISPLAY_MULTIPLIER,
            PressureData::DISPLAY_PRECISION,
            (self.pressure as f32) * PressureData::DISPLAY_MULTIPLIER,
            HumidityData::DISPLAY_PRECISION,
            (self.humidity as f32) * HumidityData::DISPLAY_MULTIPLIER,
            self.battery,
            self.status,
            self.interval,
            self.ago,
        )
    }
}

#[derive(Debug)]
struct HistoryResponseHeader {
    type_code: u8,
    start_index: u16,
    packet_num_elem: u8,
}

impl From<[u8; 4]> for HistoryResponseHeader {
    fn from(item: [u8; 4]) -> Self {
        HistoryResponseHeader {
            type_code: item[0],
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
            self.type_code, self.start_index, self.packet_num_elem,
        )
    }
}

pub fn print_current_sensor_data(sensor_name: &str, measurement: &CurrentSensorMeasurement) {
    println!(
        "{}\n{}\n{}",
        sensor_name,
        "=".repeat(sensor_name.graphemes(true).count()),
        measurement
    );
}

pub fn print_history<T, const S: u8>(data: SensorData<T, S>)
where
    f32: From<T>,
    SensorData<T, S>: Metadata,
    T: Copy,
{
    let label = data.label();
    println!(
        "{}\n{}\n{}",
        label,
        "=".repeat(label.graphemes(true).count()),
        data,
    );
}

fn bytes_to_single_u16(bytes: &[u8]) -> Result<u16, Aranet4Error> {
    if bytes.len() == 2 {
        Ok(u16::from_le_bytes(bytes.try_into().unwrap()))
    } else {
        Err(Aranet4Error::InvalidResponse(
            "Result of total readings is not 2 bytes".to_string(),
        ))
    }
}

async fn get_total_readings(sensor: &Peripheral) -> Result<u16, Aranet4Error> {
    let char = get_characteristic(sensor, ARANET4_TOTAL_READINGS_UUID)?;
    let bytes = sensor.read(&char).await?;
    bytes_to_single_u16(&bytes)
}

async fn get_time_since_update(sensor: &Peripheral) -> Result<u16, Aranet4Error> {
    let char = get_characteristic(sensor, ARANET4_TIME_SINCE_UPDATE_UUID)?;
    let bytes = sensor.read(&char).await?;
    bytes_to_single_u16(&bytes)
}

async fn get_update_interval(sensor: &Peripheral) -> Result<u16, Aranet4Error> {
    let char = get_characteristic(sensor, ARANET4_UPDATE_INTERVAL_UUID)?;
    let bytes = sensor.read(&char).await?;
    bytes_to_single_u16(&bytes)
}

pub async fn get_current_sensor_data(
    sensor: &Peripheral,
) -> Result<(String, CurrentSensorMeasurement), Aranet4Error> {
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
    let co2_char = get_characteristic(sensor, ARANET4_CURRENT_READINGS_UUID)?;
    let measurement_bytes = sensor.read(&co2_char).await?;
    if measurement_bytes.len() != 13 {
        return Err(Aranet4Error::InvalidResponse(
            "Unexpected current measurement length".to_string(),
        ));
    }
    let measurement_bytes: [u8; 13] = measurement_bytes[..13].try_into().unwrap();
    Ok((local_name, measurement_bytes.into()))
}

fn get_characteristic(
    sensor: &Peripheral,
    char_uuid: Uuid,
) -> Result<Characteristic, Aranet4Error> {
    let chars = sensor.characteristics();
    chars
        .iter()
        .find(|c| c.uuid == char_uuid)
        .cloned()
        .ok_or(Aranet4Error::CharacteristicNotFound)
}

pub async fn get_history<T, const SENSORTYPE: u8>(
    sensor: &Peripheral,
) -> Result<SensorData<T, SENSORTYPE>, Aranet4Error>
where
    SensorData<T, SENSORTYPE>: Metadata + for<'a> TryFrom<&'a [u8], Error = Aranet4Error>,
{
    // connect to the device
    sensor.connect().await?;

    // discover services and characteristics
    sensor.discover_services().await?;

    let subscribe_char = get_characteristic(sensor, ARANET4_NOTIFY_HISTORY_UUID)?;
    let command_char = get_characteristic(sensor, ARANET4_COMMAND_UUID)?;
    if !subscribe_char.properties.contains(CharPropFlags::NOTIFY) {
        return Err(Aranet4Error::InvalidResponse(
            "No NOTIFY flag on subscribe characteristic!".to_string(),
        ));
    }

    // Perform the arcane ritual
    let total_readings = get_total_readings(sensor).await?;
    let get_history_command_bytes: &[u8] = &[
        0x82,
        SENSORTYPE,
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
    let bytes_per_elem = size_of::<T>();
    let mut notification_stream = sensor.notifications().await?;
    let mut history_bytes = Vec::new();
    while let Some(data) = notification_stream.next().await {
        if data.uuid != ARANET4_NOTIFY_HISTORY_UUID {
            return Err(Aranet4Error::InvalidResponse(
                "Expected notification UUID to match ARANET4_NOTIFY_HISTORY_UUID".to_string(),
            ));
        }
        if data.value.len() < 4 {
            return Err(Aranet4Error::InvalidResponse(
                "Expected at least 4 bytes for the header".to_string(),
            ));
        }
        let header_bytes: [u8; 4] = data.value[..4].try_into().unwrap();
        if header_bytes[0] != SENSORTYPE {
            return Err(Aranet4Error::InvalidResponse(
                "History type doesn't match what we requested".to_string(),
            ));
        }
        let header = HistoryResponseHeader::from(header_bytes);
        let bytes_end = 4 + bytes_per_elem * (header.packet_num_elem as usize);
        history_bytes.extend_from_slice(&data.value[4..bytes_end]);
        if history_bytes.len() >= bytes_per_elem * (total_readings as usize) {
            break;
        }
    }
    sensor.unsubscribe(&subscribe_char).await?;
    if history_bytes.len() != bytes_per_elem * (total_readings as usize) {
        return Err(Aranet4Error::InvalidResponse(
            "Received unexpected number of bytes".to_string(),
        ));
    }

    let history_data = history_bytes[..].try_into()?;
    Ok(history_data)
}

pub async fn get_temperature_history(sensor: &Peripheral) -> Result<TemperatureData, Aranet4Error> {
    get_history(sensor).await
}

pub async fn get_humidity_history(sensor: &Peripheral) -> Result<HumidityData, Aranet4Error> {
    get_history(sensor).await
}

pub async fn get_pressure_history(sensor: &Peripheral) -> Result<PressureData, Aranet4Error> {
    get_history(sensor).await
}

pub async fn get_co2_history(sensor: &Peripheral) -> Result<CO2Data, Aranet4Error> {
    get_history(sensor).await
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

pub async fn get_history_start_time(sensor: &Peripheral) -> Result<DateTime<Utc>, Aranet4Error> {
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
