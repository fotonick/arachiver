use std::fs::File;
use std::io::Write;

use btleplug::platform::Peripheral;
use chrono::Local;
use color_eyre::eyre::{eyre, Result};
use futures::future::join_all;

use crate::device::HistoryTime;
use crate::device::{
    get_co2_history, get_humidity_history, get_local_name, get_pressure_history,
    get_temperature_history,
};
use crate::types::Metadata;

async fn save_history_csv<W: Write>(sensor: &Peripheral, dest: &mut W) -> Result<()> {
    let mut dest = csv::Writer::from_writer(dest);

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
            temperature.get_float_value(i).to_string(),
            humidity.get_float_value(i).to_string(),
            pressure.get_float_value(i).to_string(),
            co2.get_float_value(i).to_string(),
        ])
        .expect(&format!(
            "Failed while writing CSV row {} (data record {})",
            i + 1,
            i
        ));
    }
    Ok(())
}

pub async fn save_history_csv_all(peripherals: &[Peripheral]) -> Result<Vec<String>> {
    let mut tasks = Vec::new();
    let mut names = Vec::new();
    for peripheral in peripherals {
        let peripheral = peripheral.clone();
        let Some(local_name) = get_local_name(&peripheral).await else {
            continue;
        };
        if local_name.contains("Aranet4") {
            let now = Local::now();
            let output_filename = format!(
                "{}_{}_history.csv",
                now.to_rfc3339(),
                local_name.replace(" ", "_")
            );
            let mut output_file = File::create(&output_filename).expect(&format!(
                "Could not create writeable file {}",
                &output_filename
            ));
            tasks.push(tokio::spawn(async move {
                save_history_csv(&peripheral, &mut output_file).await
            }));
            names.push(output_filename);
        }
    }
    if join_all(tasks).await.iter().all(|result| result.is_ok()) {
        Ok(names)
    } else {
        Err(eyre!("Failed to save history CSV"))
    }
}
