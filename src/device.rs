use btleplug::api::{
    bleuuid::uuid_from_u16, Central as _, CentralEvent, CharPropFlags, Characteristic,
    Peripheral as _, ScanFilter, WriteType,
};
use btleplug::platform::{Adapter, Peripheral};
use chrono::{DateTime, TimeDelta, Utc};
use color_eyre::{eyre::eyre, Result};
use std::mem::size_of;
use std::time::{Duration, Instant};
use tokio_stream::StreamExt;
use unicode_segmentation::UnicodeSegmentation;
use uuid::{uuid, Uuid};

use crate::types::*;

pub const ARANET4_SERVICE_UUID: Uuid = uuid_from_u16(0xfce0);
const ARANET4_CURRENT_READINGS_UUID: Uuid = uuid!("f0cd3001-95da-4f4b-9ac8-aa55d312af0c");
const ARANET4_NOTIFY_HISTORY_UUID: Uuid = uuid!("f0cd2003-95da-4f4b-9ac8-aa55d312af0c");
const ARANET4_COMMAND_UUID: Uuid = uuid!("f0cd1402-95da-4f4b-9ac8-aa55d312af0c");
const ARANET4_TOTAL_READINGS_UUID: Uuid = uuid!("f0cd2001-95da-4f4b-9ac8-aa55d312af0c");
const ARANET4_TIME_SINCE_UPDATE_UUID: Uuid = uuid!("f0cd2004-95da-4f4b-9ac8-aa55d312af0c");
const ARANET4_UPDATE_INTERVAL_UUID: Uuid = uuid!("f0cd2002-95da-4f4b-9ac8-aa55d312af0c");

const GENERIC_GATT_DEVICE_MODEL_NUMBER_STRING_UUID: Uuid =
    uuid!("00002a24-0000-1000-8000-00805f9b34fb");
const GENERIC_GATT_SERIAL_NUMBER_STRING_UUID: Uuid = uuid!("00002a25-0000-1000-8000-00805f9b34fb");
const GENERIC_GATT_HARDWARE_REVISION_STRING_UUID: Uuid =
    uuid!("00002a27-0000-1000-8000-00805f9b34fb");
const GENERIC_GATT_SOFTWARE_REVISION_STRING_UUID: Uuid =
    uuid!("00002a28-0000-1000-8000-00805f9b34fb");
const GENERIC_GATT_MANUFACTURER_NAME_STRING_UUID: Uuid =
    uuid!("00002a29-0000-1000-8000-00805f9b34fb");
const GENERIC_GATT_FIRMWARE_REVISION_STRING_UUID: Uuid =
    uuid!("00002a26-0000-1000-8000-00805f9b34fb");

#[derive(Debug, Clone)]
pub struct DeviceInfo {
    device_name: String,
    model_number: String,
    serial_number: String,
    hardware_revision: String,
    software_revision: String,
    manufacturer_name: String,
    firmware_revision: String,
}
async fn get_string(sensor: &Peripheral, uuid: Uuid) -> Result<String> {
    let char = get_characteristic(sensor, uuid)?;
    let bytes = sensor.read(&char).await?;
    Ok(String::from_utf8_lossy(&bytes).to_string())
}

impl DeviceInfo {
    pub async fn new(sensor: &Peripheral) -> Result<Self> {
        // connect to the device
        sensor.connect().await?;

        // discover services and characteristics
        sensor.discover_services().await?;

        let device_name = get_local_name(sensor)
            .await
            .unwrap_or("<Missing device name>".to_string());
        let model_number = get_string(sensor, GENERIC_GATT_DEVICE_MODEL_NUMBER_STRING_UUID).await?;
        let serial_number = get_string(sensor, GENERIC_GATT_SERIAL_NUMBER_STRING_UUID).await?;
        let hardware_revision =
            get_string(sensor, GENERIC_GATT_HARDWARE_REVISION_STRING_UUID).await?;
        let software_revision =
            get_string(sensor, GENERIC_GATT_SOFTWARE_REVISION_STRING_UUID).await?;
        let manufacturer_name =
            get_string(sensor, GENERIC_GATT_MANUFACTURER_NAME_STRING_UUID).await?;
        let firmware_revision =
            get_string(sensor, GENERIC_GATT_FIRMWARE_REVISION_STRING_UUID).await?;
        Ok(DeviceInfo {
            device_name,
            model_number,
            serial_number,
            hardware_revision,
            software_revision,
            manufacturer_name,
            firmware_revision,
        })
    }
}

pub fn print_device_info(info: &DeviceInfo) {
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

pub fn print_current_sensor_data(sensor_name: &str, measurement: &CurrentSensorMeasurement) {
    println!(
        "{}\n{}\n{}",
        sensor_name,
        "=".repeat(sensor_name.graphemes(true).count()),
        measurement
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
    let local_name = get_local_name(sensor).await.unwrap();

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

async fn get_single_history_type<T, const SENSORTYPE: u8>(
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
    get_single_history_type(sensor).await
}

pub async fn get_humidity_history(sensor: &Peripheral) -> Result<HumidityData, Aranet4Error> {
    get_single_history_type(sensor).await
}

pub async fn get_pressure_history(sensor: &Peripheral) -> Result<PressureData, Aranet4Error> {
    get_single_history_type(sensor).await
}

pub async fn get_co2_history(sensor: &Peripheral) -> Result<CO2Data, Aranet4Error> {
    get_single_history_type(sensor).await
}

#[derive(Debug)]
pub struct HistoryTime {
    pub num_samples: usize,
    pub update_interval: u16,
    pub since_update: u16,
    pub now: DateTime<Utc>,
}

impl HistoryTime {
    pub async fn from_sensor(sensor: &Peripheral, num_samples: usize) -> Result<Self> {
        Ok(HistoryTime {
            num_samples,
            update_interval: get_update_interval(sensor).await?,
            since_update: get_time_since_update(sensor).await?,
            now: Utc::now(),
        })
    }

    pub fn get_timestamp(&self, sample: usize) -> Result<i64> {
        if sample >= self.num_samples {
            return Err(eyre!(
                "Invalid sample index {} (# samples = {})",
                sample,
                self.num_samples
            ));
        }
        let time = self.now
            - TimeDelta::seconds(
                (self.num_samples as i64 - sample as i64 - 1) * (self.update_interval as i64)
                    + (self.since_update as i64),
            );
        Ok(time.timestamp())
    }

    pub fn to_vec(&self) -> Vec<i64> {
        (0..self.num_samples)
            .map(|i| self.get_timestamp(i).unwrap())
            .collect()
    }
}

pub async fn get_local_name(peripheral: &Peripheral) -> Option<String> {
    peripheral
        .properties()
        .await
        .expect("expect property result")
        .expect("expect some properties")
        .local_name
}

pub async fn scan_for_sensor(central: &Adapter, device_pattern: &str) -> Result<Peripheral> {
    // Set global timeout as our main timeout mechanism, but also per-element
    // timeout since global timeout may not be evaluated if the Bluetooth
    // environment is very quiet and no events are generated.
    const TIMEOUT: Duration = Duration::from_secs(5);
    let start = Instant::now();
    let mut events = Box::pin(central.events().await?.timeout(TIMEOUT));
    central
        .start_scan(ScanFilter {
            services: vec![ARANET4_SERVICE_UUID],
        })
        .await
        .unwrap();
    while let Ok(Some(event)) = events.try_next().await {
        match event {
            CentralEvent::DeviceDiscovered(id) => {
                let peripheral = central.peripheral(&id).await?;
                if let Some(local_name) = get_local_name(&peripheral).await {
                    if local_name.contains(device_pattern) {
                        return Ok(peripheral);
                    }
                }
            }
            _ => {}
        }
        if Instant::now().duration_since(start) > TIMEOUT {
            break;
        }
    }
    Err(eyre!("No device found before timeout"))
}

pub async fn get_history(
    sensor: &Peripheral,
) -> Result<(
    HistoryTime,
    TemperatureData,
    HumidityData,
    PressureData,
    CO2Data,
)> {
    // Await each one sequentially because while we could do two separate devices in
    // parallel, there's no speedup to be had by multiply querying a single device and
    // it would probably confuse the device.
    let temperature = get_temperature_history(sensor).await?;
    let humidity = get_humidity_history(sensor).await?;
    let pressure = get_pressure_history(sensor).await?;
    let co2 = get_co2_history(sensor).await?;
    assert_eq!(temperature.values.len(), humidity.values.len());
    assert_eq!(temperature.values.len(), pressure.values.len());
    assert_eq!(temperature.values.len(), co2.values.len());
    let history_time = HistoryTime::from_sensor(sensor, temperature.values.len()).await?;
    Ok((history_time, temperature, humidity, pressure, co2))
}
