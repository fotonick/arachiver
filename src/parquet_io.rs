use std::io::Write;
use std::sync::Arc;

use color_eyre::eyre::Result;
use parquet::{
    basic::{Compression, Repetition, Type, ZstdLevel},
    data_type::{FloatType, Int32Type, Int64Type},
    file::{metadata::KeyValue, properties::WriterProperties, writer::SerializedFileWriter},
    schema::types,
};

use crate::device::HistoryTime;
use crate::types::{CO2Data, HumidityData, Metadata, PressureData, TemperatureData};

fn required_field(name: &str, ty: Type) -> Arc<types::Type> {
    Arc::new(
        types::Type::primitive_type_builder(name, ty)
            .with_repetition(Repetition::REQUIRED)
            .build()
            .unwrap(),
    )
}

pub async fn save_history_parquet<W: Write + Send + Sync>(
    history_time: HistoryTime,
    temperature: TemperatureData,
    humidity: HumidityData,
    pressure: PressureData,
    co2: CO2Data,
    dest: &mut W,
) -> Result<()> {
    let schema = Arc::new(
        types::Type::group_type_builder("schema")
            .with_fields(vec![
                required_field("timestamp", Type::INT64),
                required_field("temperature", Type::FLOAT),
                required_field("humidity", Type::INT32),
                required_field("pressure", Type::FLOAT),
                required_field("co2", Type::INT32),
            ])
            .build()
            .unwrap(),
    );
    const COMPRESSION_LEVEL: i32 = 1; // Zstd has a max compression level of 22
    let props = Arc::new(
        WriterProperties::builder()
            .set_compression(Compression::ZSTD(
                ZstdLevel::try_new(COMPRESSION_LEVEL).unwrap(),
            ))
            .set_key_value_metadata(Some(vec![
                KeyValue::new("timestamp_unit".to_string(), Some("UNIX time".to_string())),
                KeyValue::new(
                    "temperature_unit".to_string(),
                    Some(temperature.label().to_string()),
                ),
                KeyValue::new(
                    "humidity_unit".to_string(),
                    Some(humidity.label().to_string()),
                ),
                KeyValue::new(
                    "pressure_unit".to_string(),
                    Some(pressure.label().to_string()),
                ),
                KeyValue::new("co2_unit".to_string(), Some(co2.label().to_string())),
            ]))
            .build(),
    );
    let mut writer = SerializedFileWriter::new(dest, schema, props).unwrap();
    let mut row_group_writer = writer.next_row_group().unwrap();
    if let Some(mut col_writer) = row_group_writer.next_column().unwrap() {
        col_writer
            .typed::<Int64Type>()
            .write_batch(&history_time.to_vec(), None, None)
            .unwrap();
        col_writer.close().unwrap()
    }
    if let Some(mut col_writer) = row_group_writer.next_column().unwrap() {
        let float_temp: Vec<f32> = (0..temperature.values.len())
            .map(|i| temperature.get_f32_value(i))
            .collect();
        col_writer
            .typed::<FloatType>()
            .write_batch(&float_temp, None, None)
            .unwrap();
        col_writer.close().unwrap()
    }
    if let Some(mut col_writer) = row_group_writer.next_column().unwrap() {
        let i32_humidity: Vec<i32> = humidity.values.iter().map(|v| *v as i32).collect();
        col_writer
            .typed::<Int32Type>()
            .write_batch(&i32_humidity, None, None)
            .unwrap();
        col_writer.close().unwrap()
    }
    if let Some(mut col_writer) = row_group_writer.next_column().unwrap() {
        let float_pressure: Vec<f32> = (0..pressure.values.len())
            .map(|i| pressure.get_f32_value(i))
            .collect();
        col_writer
            .typed::<FloatType>()
            .write_batch(&float_pressure, None, None)
            .unwrap();
        col_writer.close().unwrap()
    }
    if let Some(mut col_writer) = row_group_writer.next_column().unwrap() {
        let i32_co2: Vec<i32> = co2.values.iter().map(|v| *v as i32).collect();
        col_writer
            .typed::<Int32Type>()
            .write_batch(&i32_co2, None, None)
            .unwrap();
        col_writer.close().unwrap()
    }
    row_group_writer.close().unwrap();
    writer.close().unwrap();
    Ok(())
}

#[cfg(test)]
mod test {
    use crate::device::HistoryTime;
    use crate::parquet_io::save_history_parquet;
    use crate::types::{CO2Data, HumidityData, PressureData, TemperatureData};
    use chrono::Utc;
    use tokio;

    #[tokio::test]
    async fn test_save_history_parquet() {
        let bytes = [144u8, 1, 164, 1];
        let temperature = TemperatureData::try_from(&bytes[..]).unwrap();
        assert_eq!(temperature.get_f32_value(0), 20.0);
        assert_eq!(temperature.get_f32_value(1), 21.0);
        let humidity = HumidityData::try_from(&bytes[0..2]).unwrap();
        let pressure = PressureData::try_from(&bytes[..]).unwrap();
        let co2 = CO2Data::try_from(&bytes[..]).unwrap();
        let history_time = HistoryTime {
            num_samples: 2,
            update_interval: 300,
            since_update: 24,
            now: Utc::now(),
        };
        let mut output = Vec::new();
        save_history_parquet(
            history_time,
            temperature,
            humidity,
            pressure,
            co2,
            &mut output,
        )
        .await
        .unwrap();
        assert_eq!(output[0..4], ['P' as u8, 'A' as u8, 'R' as u8, '1' as u8]);
        assert_eq!(
            output[(output.len() - 4)..output.len()],
            ['P' as u8, 'A' as u8, 'R' as u8, '1' as u8]
        );
    }
}
