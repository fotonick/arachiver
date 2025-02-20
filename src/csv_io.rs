use std::io::Write;

use color_eyre::eyre::Result;
use csv;

use crate::device::HistoryTime;
use crate::types::{CO2Data, HumidityData, Metadata, PressureData, TemperatureData};

pub async fn save_history_csv<W: Write>(
    history_time: HistoryTime,
    temperature: TemperatureData,
    humidity: HumidityData,
    pressure: PressureData,
    co2: CO2Data,
    dest: &mut W,
) -> Result<()> {
    let mut dest = csv::Writer::from_writer(dest);
    dest.write_record([
        "timestamp",
        temperature.label(),
        humidity.label(),
        pressure.label(),
        co2.label(),
    ])
    .expect("Failed while writing CSV header");
    for i in 0..temperature.values.len() {
        dest.write_record([
            history_time.get_timestamp(i)?.to_string(),
            temperature.get_value_string(i),
            humidity.get_value_string(i),
            pressure.get_value_string(i),
            co2.get_value_string(i),
        ])
        .expect(&format!(
            "Failed while writing CSV row {} (data record {})",
            i + 1,
            i
        ));
    }
    Ok(())
}
