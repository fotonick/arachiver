arachiver
=========

Arachiver is a cross-platform tool for archiving data from Aranet4 CO2 sensors. Aranet4 devices retain approximately 17.5 days of data with the default measurement interval. At each measurement time, it records CO₂ abundance (ppm), temperature (°C), relative humidity (%), and pressure (hPa).
The Arachiver tool is oriented toward data analysis. It can output to CSV or Apache Parquet formats. While temperature and pressure are stored on-device as integers that require applying scale factors that you Just Have To Know, Arachiver applies the appropriate scale factors and stores temperature and pressure as floating-point numbers. Timestamps are UNIX timestamps.

Installation
------------

1. Install a Rust toolchain, either through your system package manager or via the [officially recommended rustup tool](https://www.rust-lang.org/tools/install): `curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh`
1. `cargo install --git https://github.com/fotonick/arachiver`

The underlying Bluetooth library is [btleplug](https://github.com/deviceplug/btleplug), which claims to support Windows, Linux, macOS, Android, and iOS, though I have only tested macOS.

Usage
-----

```
> arachiver --help
Aranet4 archiver

Usage: arachiver [OPTIONS] <COMMAND>

Commands:
  device_info              Print device information
  readout                  Print the current sensor readings to stdout
  archive_history_csv      Save the full history to CSV
  archive_history_parquet  Save the full history to Parquet
  help                     Print this message or the help of the given subcommand(s)

Options:
  -d, --device <device_pattern>  Select an Aranet4 device with <device_pattern> in its name; by default, the first device with 'Aranet' in its name will be used [default: Aranet]
  -h, --help                     Print help
```
```
> arachiver device_info
Aranet4 1BA27
=============
Model number: Aranet4
Serial number: 317960113191
Hardware revision: 12
Software revision: v0.4.14
Manufacturer name: SAF Tehnika
Firmware revision: v1.4.14
```
```
> arachiver readout
Aranet4 1BA27
=============
CO₂: 926 ppm
T: 20.65°C
P: 1017.4 hPa
Humidity: 33%
Battery: 22%
Status: 1
Interval: 300 s
Ago: 255 s
```
```
> arachiver archive_history_csv
Wrote 2025-02-21T02:16:51.917392-08:00_Aranet4_1BA27_history.csv
> head -n 3 2025-02-21T02:16:51.917392-08:00_Aranet4_1BA27_history.csv
timestamp,Temperature (°C),Humidity (%),Pressure (hPa),CO₂ (ppm)
1738621029,14.90,29,999.8,592
1738621329,14.95,29,999.7,590
```
```
> arachiver archive_history_parquet
Wrote 2025-02-21T02:18:10.840587-08:00_Aranet4_1BA27_history.parquet
> parquet-tools inspect 2025-02-21T02:18:10.840587-08:00_Aranet4_1BA27_history.parquet

############ file meta data ############
created_by: parquet-rs version 54.2.0
num_columns: 5
num_rows: 5040
num_row_groups: 1
format_version: 1.0
serialized_size: 745


############ Columns ############
timestamp
temperature
humidity
pressure
co2

############ Column(timestamp) ############
name: timestamp
path: timestamp
max_definition_level: 0
max_repetition_level: 0
physical_type: INT64
logical_type: None
converted_type (legacy): NONE
compression: ZSTD (space_saved: 67%)

############ Column(temperature) ############
name: temperature
path: temperature
max_definition_level: 0
max_repetition_level: 0
physical_type: FLOAT
logical_type: None
converted_type (legacy): NONE
compression: ZSTD (space_saved: 33%)

############ Column(humidity) ############
name: humidity
path: humidity
max_definition_level: 0
max_repetition_level: 0
physical_type: INT32
logical_type: None
converted_type (legacy): NONE
compression: ZSTD (space_saved: 10%)

############ Column(pressure) ############
name: pressure
path: pressure
max_definition_level: 0
max_repetition_level: 0
physical_type: FLOAT
logical_type: None
converted_type (legacy): NONE
compression: ZSTD (space_saved: 7%)

############ Column(co2) ############
name: co2
path: co2
max_definition_level: 0
max_repetition_level: 0
physical_type: INT32
logical_type: None
converted_type (legacy): NONE
compression: ZSTD (space_saved: 12%)
```

The final example used [parquet-tools](https://pypi.org/project/parquet-tools/) to inspect the Parquet file.

Related tools
-------------

* https://github.com/Anrijs/Aranet4-ESP32
* https://github.com/Anrijs/Aranet4-Python

Comparatively, Arachiver was designed with enabling data analysis as its primary focus and is not concerned with exposing every control knob.

License
-------

This software is released under the MIT license.

Contribution
------------

Unless you explicitly state otherwise, any contribution intentionally submitted for inclusion into this project shall be licensed as MIT, without any additional terms or conditions.
