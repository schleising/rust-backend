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
