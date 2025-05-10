use std::str;

use chrono;
use mongodb::bson::serde_helpers::chrono_datetime_as_bson_datetime;
use serde::{Deserialize, Serialize};

pub const HUE_DISCOVERY_URL: &str = "https://discovery.meethue.com/";
pub const HUE_APPLICATION_KEY_HEADER: &str = "hue-application-key";
pub const HUE_DEVICE_URL: &str = "/clip/v2/resource/device";
pub const HUE_TEMPERATURE_URL: &str = "/clip/v2/resource/temperature";

#[derive(Deserialize)]
pub struct HueBridge {
    pub internalipaddress: String,
}

#[derive(Serialize, Deserialize)]
pub struct ProductData {
    model_id: String,
    manufacturer_name: String,
    product_name: String,
    product_archetype: String,
    certified: bool,
    software_version: String,
    hardware_platform_type: Option<String>,
}

#[derive(Serialize, Deserialize)]
pub struct Metadata {
    pub name: String,
    archetype: String,
}

#[derive(Serialize, Deserialize)]
pub struct Service {
    pub rid: String,
    pub rtype: String,
}

#[derive(Serialize, Deserialize)]
pub struct Device {
    pub id: String,
    id_v1: Option<String>,
    product_data: ProductData,
    pub metadata: Metadata,
    pub services: Vec<Service>,
    #[serde(rename = "type")]
    type_: String,
}

#[derive(Serialize, Deserialize)]
pub struct DeviceList {
    pub data: Vec<Device>,
}

#[derive(Serialize, Deserialize)]
pub struct TemperatureReport {
    pub changed: chrono::DateTime<chrono::Utc>,
    pub temperature: f32,
}

#[derive(Serialize, Deserialize)]
pub struct Temperature {
    pub temperature_report: TemperatureReport,
}

#[derive(Serialize, Deserialize)]
pub struct HueTemperatureData {
    #[serde(rename = "type")]
    type_: String,
    pub id: String,
    enabled: bool,
    pub temperature: Temperature,
}

#[derive(Serialize, Deserialize)]
pub struct HueTemperatureList {
    pub data: Vec<HueTemperatureData>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TemperatureData {
    pub device_name: String,
    #[serde(with = "chrono_datetime_as_bson_datetime")]
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub online: bool,
    pub temperature: f32,
    pub humidity: f32,
}
