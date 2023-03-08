Ref: https://gist.github.com/ariccio/2882a435c79da28ba6035a14c5c65f22

```
export const ARANET_CO2_MEASUREMENT_CHARACTERISTIC_UUID = "f0cd1503-95da-4f4b-9ac8-aa55d312af0c";
export const ARANET_TOTAL_MEASUREMENTS_UUID = "f0cd2001-95da-4f4b-9ac8-aa55d312af0c";
export const ARANET_MEASUREMENT_INTERVAL_UUID = "f0cd2002-95da-4f4b-9ac8-aa55d312af0c";
export const ARANET_SECONDS_LAST_UPDATE_UUID = "f0cd2004-95da-4f4b-9ac8-aa55d312af0c";
export const ARANET_CO2_MEASUREMENT_WITH_INTERVAL_TIME_CHARACTERISTIC_UUID = "f0cd3001-95da-4f4b-9ac8-aa55d312af0c";
// const ARANET_DEVICE_NAME_UUID = GENERIC_GATT_DEVICE_NAME_UUID;
// const ARANET_UNKNOWN_FIELD_1_UUID = 'f0cd1401-95da-4f4b-9ac8-aa55d312af0c';
// const ARANET_UNKNOWN_FIELD_2_UUID = 'f0cd1502-95da-4f4b-9ac8-aa55d312af0c';
export const ARANET_SET_INTERVAL_UUID = 'f0cd1402-95da-4f4b-9ac8-aa55d312af0c';
export const ARANET_SET_HISTORY_PARAMETER_UUID = 'f0cd1402-95da-4f4b-9ac8-aa55d312af0c';
 
export const ARANET_SENSOR_SETTINGS_STATE_UUID = 'f0cd1401-95da-4f4b-9ac8-aa55d312af0c';
export const ARANET_SENSOR_CALIBRATION_DATA_UUID = 'f0cd1502-95da-4f4b-9ac8-aa55d312af0c';
export const ARANET_UNSED_GATT_UUID = 'f0cd2003-95da-4f4b-9ac8-aa55d312af0c';
export const ARANET_SENSOR_LOGS_UUID = 'f0cd2005-95da-4f4b-9ac8-aa55d312af0c';

export const aranet4KnownCharacteristicUUIDDescriptions = new Map([
    [ARANET_CO2_MEASUREMENT_CHARACTERISTIC_UUID, "Aranet4: current CO2 measurement"],
    [ARANET_TOTAL_MEASUREMENTS_UUID, "Aranet4: total number of measurements"],
    [ARANET_MEASUREMENT_INTERVAL_UUID, "Aranet4: measurement interval"],
    [ARANET_SECONDS_LAST_UPDATE_UUID, "Aranet4: seconds since last update"],
    [ARANET_CO2_MEASUREMENT_WITH_INTERVAL_TIME_CHARACTERISTIC_UUID, "Aranet4: CO2 measurements, interval, time since measurements"],
    [GENERIC_GATT_DEVICE_NAME_UUID, "Device Name"],
    [GENERIC_GATT_DEVICE_BATTERY_LEVEL_UUID, "Aranet4: Battery level"],
    [GENERIC_GATT_DEVICE_MODEL_NUMBER_STRING_UUID, "Model Number String"],
    [GENERIC_GATT_SERIAL_NUMBER_STRING_UUID, "Serial Number String"],
    [GENERIC_GATT_HARDWARE_REVISION_STRING_UUID, "Hardware Revision String"],
    [GENERIC_GATT_SOFTWARE_REVISION_STRING_UUID, "Software Revision String"],
    [GENERIC_GATT_MANUFACTURER_NAME_STRING_UUID, "Manufacturer Name String"],
    [ARANET_SET_INTERVAL_UUID, "Set measurement interval"],
    [ARANET_SET_HISTORY_PARAMETER_UUID, "Set \"History Parameter\""],
    [ARANET_SENSOR_SETTINGS_STATE_UUID, "Aranet4 sensor settings state"],
    [ARANET_SENSOR_CALIBRATION_DATA_UUID, "Aranet4 sensor calibration"],
    [ARANET_UNSED_GATT_UUID, "Aranet4 UNUSED GATT characteristic"],
    [ARANET_SENSOR_LOGS_UUID, "Aranet4 sensor logs"]
```

Ref: https://github.com/kasparsd/sensor-pilot/blob/master/src/components/Devices/Aranet4.js

```
const aranetServices = {
  sensor: {
    serviceUuid: SENSOR_SERVICE_UUID,
    resolvers: {
      // Sensor values.
      'f0cd3001-95da-4f4b-9ac8-aa55d312af0c': (value) => {
        return {
          co2: value.getUint16(0, true),
          temperature: value.getUint16(2, true) / 20,
          pressure: value.getUint16(4, true) / 10,
          humidity: value.getUint8(6),
          battery: value.getUint8(7),
        }
      },
      // Seconds since the last sensor update.
      'f0cd2004-95da-4f4b-9ac8-aa55d312af0c': (value) => Math.floor(Date.now() / 1000) - value.getUint16(0, true),
      // Configured interval in seconds between the updates.
      'f0cd2002-95da-4f4b-9ac8-aa55d312af0c': (value) => value.getUint16(0, true),
    },
  },
  device: {
    serviceUuid: 'device_information',
    resolvers: {
      manufacturer_name_string: (value) => decoder.decode(value),
      model_number_string: (value) => decoder.decode(value),
      serial_number_string: (value) => decoder.decode(value),
      hardware_revision_string: (value) => decoder.decode(value),
      software_revision_string: (value) => decoder.decode(value),
    },
  },
}
```
