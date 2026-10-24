use std::str;

use bson::serde_helpers::datetime;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_with::serde_as;

#[derive(Deserialize)]
pub struct HueBridge {
    pub internalipaddress: String,
}

#[derive(Deserialize, Debug)]
pub struct Metadata {
    pub name: String,
}

#[derive(Deserialize, Debug)]
pub struct Service {
    pub rid: String,
    pub rtype: String,
}

#[derive(Deserialize, Debug)]
pub struct Device {
    pub metadata: Metadata,
    pub services: Vec<Service>,
}

#[derive(Deserialize, Debug)]
pub struct DeviceList {
    pub data: Vec<Device>,
}

#[derive(Deserialize, Debug)]
pub struct TemperatureReport {
    pub changed: chrono::DateTime<chrono::Utc>,
    pub temperature: f32,
}

#[derive(Deserialize, Debug)]
pub struct Temperature {
    pub temperature_report: TemperatureReport,
}

#[derive(Deserialize, Debug)]
pub struct HueTemperatureData {
    pub id: String,
    pub temperature: Temperature,
}

#[derive(Deserialize, Debug)]
pub struct HueTemperatureList {
    pub data: Vec<HueTemperatureData>,
}

#[serde_as]
#[derive(Debug, Serialize, Deserialize)]
pub struct TemperatureData {
    pub device_name: String,
    #[serde_as(as = "datetime::FromChrono04DateTime")]
    pub timestamp: DateTime<Utc>,
    pub online: bool,
    pub temperature: f32,
    pub humidity: f32,
}

#[derive(Debug, Deserialize)]
pub struct NestCredentials {
    pub client_id: String,
    pub client_secret: String,
    pub refresh_token: String,
    pub project_id: String,
}

#[derive(Debug, Deserialize)]
pub struct NestTokenResponse {
    pub access_token: String,
    pub expires_in: u32,
}

#[derive(Debug, Deserialize)]
pub struct NestDeviceList {
    #[serde(default)]
    pub devices: Vec<NestDevice>,
}

#[derive(Debug, Deserialize)]
pub struct NestDevice {
    #[allow(dead_code)]
    pub name: String,
    #[serde(rename = "type")]
    pub device_type: String,
    pub traits: NestTraits,
    #[serde(default, rename = "parentRelations")]
    pub parent_relations: Vec<NestParentRelation>,
}

impl NestDevice {
    pub fn display_name(&self) -> String {
        if let Some(custom_name) = self
            .traits
            .info
            .as_ref()
            .map(|info| info.custom_name.trim())
            .filter(|name| !name.is_empty())
        {
            return custom_name.to_string();
        }

        if let Some(room_name) = self
            .parent_relations
            .iter()
            .map(|relation| relation.display_name.trim())
            .find(|name| !name.is_empty())
        {
            return format!("Nest ({room_name})");
        }

        "Nest Thermostat".to_string()
    }
}

#[derive(Debug, Deserialize)]
pub struct NestParentRelation {
    #[serde(rename = "displayName")]
    pub display_name: String,
}

#[derive(Debug, Deserialize)]
pub struct NestTraits {
    #[serde(default, rename = "sdm.devices.traits.Info")]
    pub info: Option<NestInfoTrait>,
    #[serde(default, rename = "sdm.devices.traits.Humidity")]
    pub humidity: Option<NestHumidityTrait>,
    #[serde(default, rename = "sdm.devices.traits.Connectivity")]
    pub connectivity: Option<NestConnectivityTrait>,
    #[serde(default, rename = "sdm.devices.traits.Temperature")]
    pub temperature: Option<NestTemperatureTrait>,
    #[serde(default, rename = "sdm.devices.traits.ThermostatHvac")]
    pub hvac: Option<NestThermostatHvacTrait>,
    #[serde(default, rename = "sdm.devices.traits.ThermostatMode")]
    pub thermostat_mode: Option<NestThermostatModeTrait>,
    #[serde(default, rename = "sdm.devices.traits.ThermostatTemperatureSetpoint")]
    pub temperature_setpoint: Option<NestTemperatureSetpointTrait>,
}

#[derive(Debug, Deserialize)]
pub struct NestInfoTrait {
    #[serde(default, rename = "customName")]
    pub custom_name: String,
}

#[derive(Debug, Deserialize)]
pub struct NestHumidityTrait {
    #[serde(rename = "ambientHumidityPercent")]
    pub ambient_humidity_percent: f32,
}

#[derive(Debug, Deserialize)]
pub struct NestConnectivityTrait {
    pub status: String,
}

#[derive(Debug, Deserialize)]
pub struct NestTemperatureTrait {
    #[serde(rename = "ambientTemperatureCelsius")]
    pub ambient_temperature_celsius: f32,
}

/// `sdm.devices.traits.ThermostatHvac`. Absent status is not stored as `OFF`.
#[derive(Debug, Deserialize)]
pub struct NestThermostatHvacTrait {
    #[serde(default)]
    pub status: Option<String>,
}

/// `sdm.devices.traits.ThermostatMode`. Stored for a later usage page. Not drawn.
#[derive(Debug, Deserialize)]
pub struct NestThermostatModeTrait {
    #[serde(default)]
    pub mode: Option<String>,
}

/// Active-mode targets from `sdm.devices.traits.ThermostatTemperatureSetpoint`.
/// A GET includes only the fields for the mode that is selected.
#[derive(Debug, Deserialize)]
pub struct NestTemperatureSetpointTrait {
    #[serde(default, rename = "heatCelsius")]
    pub heat_celsius: Option<f32>,
    #[serde(default, rename = "coolCelsius")]
    pub cool_celsius: Option<f32>,
}

/// One Nest poll heartbeat in `web_database.heating_samples`.
///
/// Written even when status is unchanged. `mode` and the setpoint fields are
/// omitted when that poll's GET did not include them.
#[serde_as]
#[derive(Debug, Serialize, Deserialize)]
pub struct HeatingSample {
    pub device_name: String,
    #[serde_as(as = "datetime::FromChrono04DateTime")]
    pub timestamp: DateTime<Utc>,
    pub hvac_status: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mode: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub heat_celsius: Option<f32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cool_celsius: Option<f32>,
}

impl HeatingSample {
    /// Build a sample from one thermostat device.
    ///
    /// Returns `None` when `ThermostatHvac` is absent or has no status.
    /// Does not invent `OFF`, and does not copy setpoint fields from any earlier sample.
    pub fn from_thermostat(device: &NestDevice, timestamp: DateTime<Utc>) -> Option<Self> {
        let status = device
            .traits
            .hvac
            .as_ref()
            .and_then(|hvac| hvac.status.as_deref())
            .map(str::trim)
            .filter(|status| !status.is_empty())?;

        let mode = device
            .traits
            .thermostat_mode
            .as_ref()
            .and_then(|mode| mode.mode.as_deref())
            .map(str::trim)
            .filter(|mode| !mode.is_empty())
            .map(ToString::to_string);

        let heat_celsius = device
            .traits
            .temperature_setpoint
            .as_ref()
            .and_then(|setpoint| setpoint.heat_celsius);
        let cool_celsius = device
            .traits
            .temperature_setpoint
            .as_ref()
            .and_then(|setpoint| setpoint.cool_celsius);

        Some(Self {
            device_name: device.display_name(),
            timestamp,
            hvac_status: status.to_string(),
            mode,
            heat_celsius,
            cool_celsius,
        })
    }
}

pub const THERMOSTAT_TYPE: &str = "sdm.devices.types.THERMOSTAT";

#[cfg(test)]
mod tests {
    use super::*;

    fn device(traits: serde_json::Value) -> NestDevice {
        let mut value = serde_json::json!({
            "name": "enterprises/project/devices/thermostat",
            "type": THERMOSTAT_TYPE,
            "parentRelations": [{ "displayName": "Living Room" }]
        });
        value["traits"] = traits;
        serde_json::from_value(value).expect("device JSON should parse")
    }

    #[test]
    fn heating_sample_stores_mode_and_heat_setpoint_only() {
        let parsed = device(serde_json::json!({
            "sdm.devices.traits.Info": { "customName": "Living Room Nest" },
            "sdm.devices.traits.ThermostatHvac": { "status": "HEATING" },
            "sdm.devices.traits.ThermostatMode": {
                "mode": "HEAT",
                "availableModes": ["HEAT", "OFF"]
            },
            "sdm.devices.traits.ThermostatTemperatureSetpoint": { "heatCelsius": 20.0 },
            "sdm.devices.traits.Temperature": { "ambientTemperatureCelsius": 19.4 },
            "sdm.devices.traits.Humidity": { "ambientHumidityPercent": 48.0 },
            "sdm.devices.traits.Connectivity": { "status": "ONLINE" }
        }));
        let timestamp = DateTime::parse_from_rfc3339("2026-10-24T17:00:00Z")
            .unwrap()
            .with_timezone(&Utc);

        let sample = HeatingSample::from_thermostat(&parsed, timestamp).expect("sample");

        assert_eq!(sample.device_name, "Living Room Nest");
        assert_eq!(sample.timestamp, timestamp);
        assert_eq!(sample.hvac_status, "HEATING");
        assert_eq!(sample.mode.as_deref(), Some("HEAT"));
        assert_eq!(sample.heat_celsius, Some(20.0));
        assert_eq!(sample.cool_celsius, None);

        let stored = serde_json::to_value(&sample).expect("json");
        assert_eq!(stored["hvac_status"], "HEATING");
        assert_eq!(stored["mode"], "HEAT");
        assert_eq!(stored["heat_celsius"], 20.0);
        assert!(stored.get("cool_celsius").is_none());
    }

    #[test]
    fn cool_setpoint_is_not_copied_from_an_earlier_heat_sample() {
        let heat = device(serde_json::json!({
            "sdm.devices.traits.Info": { "customName": "Living Room Nest" },
            "sdm.devices.traits.ThermostatHvac": { "status": "OFF" },
            "sdm.devices.traits.ThermostatMode": { "mode": "HEAT" },
            "sdm.devices.traits.ThermostatTemperatureSetpoint": { "heatCelsius": 21.5 }
        }));
        let cool = device(serde_json::json!({
            "sdm.devices.traits.Info": { "customName": "Living Room Nest" },
            "sdm.devices.traits.ThermostatHvac": { "status": "COOLING" },
            "sdm.devices.traits.ThermostatMode": { "mode": "COOL" },
            "sdm.devices.traits.ThermostatTemperatureSetpoint": { "coolCelsius": 24.0 }
        }));
        let now = Utc::now();

        let heat_sample = HeatingSample::from_thermostat(&heat, now).expect("heat");
        let cool_sample = HeatingSample::from_thermostat(&cool, now).expect("cool");

        assert_eq!(heat_sample.hvac_status, "OFF");
        assert_eq!(heat_sample.heat_celsius, Some(21.5));
        assert_eq!(heat_sample.cool_celsius, None);
        assert_eq!(cool_sample.hvac_status, "COOLING");
        assert_eq!(cool_sample.mode.as_deref(), Some("COOL"));
        assert_eq!(cool_sample.heat_celsius, None);
        assert_eq!(cool_sample.cool_celsius, Some(24.0));
    }

    #[test]
    fn heatcool_keeps_both_setpoints() {
        let parsed = device(serde_json::json!({
            "sdm.devices.traits.ThermostatHvac": { "status": "OFF" },
            "sdm.devices.traits.ThermostatMode": { "mode": "HEATCOOL" },
            "sdm.devices.traits.ThermostatTemperatureSetpoint": {
                "heatCelsius": 18.0,
                "coolCelsius": 24.5
            }
        }));

        let sample = HeatingSample::from_thermostat(&parsed, Utc::now()).expect("sample");

        assert_eq!(sample.device_name, "Nest (Living Room)");
        assert_eq!(sample.mode.as_deref(), Some("HEATCOOL"));
        assert_eq!(sample.heat_celsius, Some(18.0));
        assert_eq!(sample.cool_celsius, Some(24.5));
    }

    #[test]
    fn missing_hvac_is_skipped_even_when_mode_is_heat() {
        let parsed = device(serde_json::json!({
            "sdm.devices.traits.Info": { "customName": "Living Room Nest" },
            "sdm.devices.traits.ThermostatMode": { "mode": "HEAT" },
            "sdm.devices.traits.ThermostatTemperatureSetpoint": { "heatCelsius": 20.0 },
            "sdm.devices.traits.Temperature": { "ambientTemperatureCelsius": 19.0 }
        }));

        assert!(HeatingSample::from_thermostat(&parsed, Utc::now()).is_none());
    }

    #[test]
    fn blank_hvac_status_is_not_invented_as_off() {
        let missing = device(serde_json::json!({
            "sdm.devices.traits.ThermostatHvac": {}
        }));
        let blank = device(serde_json::json!({
            "sdm.devices.traits.ThermostatHvac": { "status": "  " }
        }));

        assert!(HeatingSample::from_thermostat(&missing, Utc::now()).is_none());
        assert!(HeatingSample::from_thermostat(&blank, Utc::now()).is_none());
    }
}
