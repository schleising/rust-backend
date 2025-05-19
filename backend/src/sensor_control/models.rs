use std::str;

use chrono;
use mongodb::bson::serde_helpers::chrono_datetime_as_bson_datetime;
use serde::{Deserialize, Serialize};

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

#[derive(Debug, Serialize)]
pub struct TemperatureData<'a> {
    pub device_name: &'a str,
    #[serde(with = "chrono_datetime_as_bson_datetime")]
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub online: bool,
    pub temperature: f32,
    pub humidity: f32,
}
