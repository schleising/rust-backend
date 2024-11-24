use std::str;

use chrono;
use mongodb::bson::serde_helpers::chrono_datetime_as_bson_datetime;
use serde::{Deserialize, Serialize};

// The instance type of the capability, e.g. "online" or "sensorTemperature"
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum InstanceType {
    #[serde(rename = "online")]
    Online,
    #[serde(rename = "sensorTemperature")]
    Temperature,
    #[serde(rename = "sensorHumidity")]
    Humidity,
}

// The value of the humidity capability
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HumidityValue {
    #[serde(rename = "currentHumidity")]
    pub current_humidity: f32,
}

// The state of the capability, e.g. the temperature value
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Data {
    Bool(bool),
    Temperature(f32),
    Humidity(HumidityValue),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Value {
    pub value: Data,
}

// The container for the capability data
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Capability {
    #[serde(rename = "type")]
    pub capability_type: String,
    pub instance: InstanceType,
    pub state: Option<Value>,
}

// The device information
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Device {
    pub sku: String,
    pub device: String,
    #[serde(rename = "deviceName")]
    pub device_name: String,
    #[serde(rename = "type")]
    pub device_type: String,
    pub capabilities: Vec<Capability>,
}

// The response from the Govee API for the list of devices
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GoveeDeviceResponse {
    pub code: i32,
    pub message: String,
    pub data: Vec<Device>,
}

// The payload for the status request
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GoveeStatusRequestPayload {
    pub sku: String,
    pub device: String,
    pub capabilities: Option<Vec<Capability>>,
}

// The request to get the status of a device
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GoveeStatusRequest {
    #[serde(rename = "requestId")]
    pub request_id: String,
    pub payload: GoveeStatusRequestPayload,
}

// The response from the Govee API for the status request
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GoveeStatusResponse {
    #[serde(rename = "requestId")]
    pub request_id: String,
    pub code: i32,
    pub msg: String,
    pub payload: GoveeStatusRequestPayload,
}

// Measurement data from the sensor stored in the database
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SensorData {
    pub device_name: String,
    #[serde(with = "chrono_datetime_as_bson_datetime")]
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub online: bool,
    pub temperature: f32,
    pub humidity: f32,
}
