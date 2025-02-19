
=====
Ref: https://github.com/Anrijs/Aranet4-ESP32.git

Logic for requesting history.

======================
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

Ref: https://github.com/ariccio/COVID-CO2-tracker/blob/main/co2_client/src/features/bluetooth/notes.md

```
    "F0CD2005-95DA-4F4B-9AC8-AA55D312AF0C" likely contains all logged data? Done in a super complex multi-step async function in readLogData:
        value: function (t, n) {
            var u, s, c, f, h, v, b, x, I, R, T, C, S, P, U, M, N = this;
            return o.default.async(function (O) {
                for (;;) switch (O.prev = O.next) {
                case 0:
                    return O.next = 2, o.default.awrap(w.default.read(t.id, k, "F0CD2005-95DA-4F4B-9AC8-AA55D312AF0C"));
                case 2:
                    if (u = O.sent, s = Math.floor(Date.now() / 1e3), c = y.default.Buffer.from(u), 0 !== (f = c.readUInt8(0))) {
                        O.next = 8;
                        break
                    }
                    throw 'error in parameters';
                case 8:
                    if (129 !== f) {
                        O.next = 11;
                        break
                    }
                    return D.default.log('reading in progresss. schedule reading logs after 1 second'), O.abrupt("return", (0, E.delayed)(1e3).then(function () {
                        return N.readLogData(t, n)
                    }));
                case 11:
                    if (h = Object.keys(F).find(function (t) {
                            return F[t] === f
                        })) {
                        O.next = 14;
                        break
                    }
                    throw "unknown measurement " + f;
                case 14:
                    v = c.readUInt16LE(1), b = c.readUInt16LE(3), x = c.readUInt16LE(5), I = c.readUInt16LE(7), R = c.readUInt8(9), T = s - (x + b * v), C = 0;
                case 21:
                    if (!(C < R)) {
                        O.next = 32;
                        break
                    }
                    if (0 !== (S = I + C)) {
                        O.next = 25;
                        break
                    }
                    return O.abrupt("continue", 29);
                case 25:
                    P = h === A.HUMIDITY ? c.readUInt8(10 + C) : h === A.TEMPERATURE ? c.readInt16LE(10 + 2 * C) : c.readUInt16LE(10 + 2 * C), U = P * L[h], n[M = T + S * v] = (0, l.default)({}, n[M], (0, p.default)({}, h, U));
                case 29:
                    C++, O.next = 21;
                    break;
                case 32:
                    if (0 === R) {
                        O.next = 34;
                        break
                    }
                    return O.abrupt("return", this.readLogData(t, n));
                case 34:
                case "end":
                    return O.stop()
                }

    There's also a loadDataLogV2, which is curious, since it also writes to the "set history parameter":
        var n, u, s, f, h, p, v, y, x, A, I, R, T, C, S, P, U, L, M, N, O = this;
        return o.default.async(function (H) {
            for (;;) switch (H.prev = H.next) {
            case 0:
                return u = (0, l.default)({}, null == (n = b.default.getState().logs) ? void 0 : n[t.id]), s = Date.now() / 1e3, Object.keys(u).forEach(function (t) {
                    s - t > 1209600 && delete u[t]
                }), H.next = 5, o.default.awrap((0, E.retry)(5, function () {
                    return O.connectToDevice(t)
                }));
            case 5:
                return H.prev = 5, H.next = 8, o.default.awrap((0, E.retry)(5, function () {
                    return O.retrieveServices(t)
                }));
            case 8:
                return H.next = 10, o.default.awrap(this.getReadingsIntervalAndTimePassed(t.id));
            case 10:
                return f = H.sent, h = f.readingsInterval, p = f.timePassed, v = Math.round(Date.now() / 1e3) - p, H.next = 16, o.default.awrap(this.getLoggedRecordCount(t));
            case 16:
                y = H.sent, x = v - y * h, A = Object.keys(u), I = A.length, R = A[I - 1] - A[I - 2] > h ? 0 : Object.keys(u).reduce(function (t, n) {
                    return isNaN(n) || t > n ? t : parseInt(n)
                }, 0), T = 3, C = Math.max(0, Math.floor((R - x) / h) - T), S = B(C, 2), P = 0, U = Object.values(F);
            case 25:
                if (!(P < U.length)) {
                    H.next = 34;
                    break
                }
                return L = U[P], H.next = 29, o.default.awrap(w.default.write(t.id, k, "F0CD1402-95DA-4F4B-9AC8-AA55D312AF0C", [97, L].concat((0, c.default)(S))));
            case 29:
                return H.next = 31, o.default.awrap(this.readLogData(t, u));
            case 31:
                P++, H.next = 25;
                break;
            case 34:
                return H.prev = 34, H.next = 37, o.default.awrap(this.disconnectFromDevice(t).catch(D.default.log));
            case 37:
                return H.finish(34);
            case 38:
                M = Object.keys(u), N = M.shift() % 60, M.forEach(function (t) {
                    var n = t % 60;
                    if (n !== N) {
                        var s = t - n + N;
                        u[s] = u[s] || {}, Object.keys(F).forEach(function (n) {
                            var o;
                            u[s][n] = u[s][n] || (null == (o = u[t]) ? void 0 : o[n]) || null
                        }), delete u[t]
                    }
                }), b.default.dispatch({
                    type: 'setLogs',
                    device: t,
                    payload: u
                });
            case 42:
            case "end":
                return H.stop()
            }
```
