use btleplug::Error as BtleplugError;
use std::fmt;
use std::vec::Vec;
use thiserror::Error;
use unicode_segmentation::UnicodeSegmentation;

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

pub type TemperatureData = SensorData<u16, TEMPERATURE>;
pub type HumidityData = SensorData<u8, HUMIDITY>;
pub type PressureData = SensorData<u16, PRESSURE>;
pub type CO2Data = SensorData<u16, CO2>;

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
        let label = self.label();
        let mut result = write!(
            f,
            "{}\n{}\n",
            label,
            "=".repeat(label.graphemes(true).count())
        );
        result = result.and(write!(f, "["));
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
pub struct HistoryResponseHeader {
    pub type_code: u8,
    pub start_index: u16,
    pub packet_num_elem: u8,
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
