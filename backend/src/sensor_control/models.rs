use std::str;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_with::serde_as;
use bson::serde_helpers::datetime;

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

pub const THERMOSTAT_TYPE: &str = "sdm.devices.types.THERMOSTAT";
