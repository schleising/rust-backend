use chrono;

use crate::sensors::models::{
    Data, Device, GoveeDeviceResponse, GoveeStatusRequest, GoveeStatusRequestPayload,
    GoveeStatusResponse, InstanceType, SensorData, Value,
};

const DEVICE_LIST_URL: &str = "https://openapi.api.govee.com/router/api/v1/user/devices";
const DEVICE_STATE_URL: &str = "https://openapi.api.govee.com/router/api/v1/device/state";

// The container for the list of devices
#[derive(Debug)]
pub struct Devices {
    pub api_key: String,
    pub devices: Vec<Device>,
}

impl Devices {
    pub fn new() -> Devices {
        let api_key = std::fs::read_to_string("secrets/govee_api_key.txt")
            .expect("Could not read the secret file");

        let mut device_list = Devices {
            api_key,
            devices: Vec::new(),
        };

        let devices = get_devices(&device_list.api_key).expect("Could not get devices");

        for device in devices.data {
            device_list.add_device(device);
        }

        device_list
    }

    fn add_device(&mut self, device: Device) {
        self.devices.push(device);
    }
}

fn get_devices(api_key: &str) -> Result<GoveeDeviceResponse, ureq::Error> {
    let url = DEVICE_LIST_URL;
    let response = ureq::get(&url)
        .set("Content-Type", "application/json")
        .set("Govee-API-Key", api_key)
        .call()?;

    let body = response.into_json()?;

    Ok(body)
}

impl Device {
    pub fn get_data(&self, api_key: &str) -> Option<SensorData> {
        let payload = GoveeStatusRequestPayload {
            sku: self.sku.clone(),
            device: self.device.clone(),
            capabilities: None,
        };

        let request = GoveeStatusRequest {
            request_id: "uuid".to_string(),
            payload,
        };

        let response: ureq::Response = match ureq::post(DEVICE_STATE_URL)
            .set("Content-Type", "application/json")
            .set("Govee-API-Key", api_key)
            .send_json(&request) {
            Ok(response) => response,
            Err(e) => {
                println!("Error: {}", e);
                return None;
            }
        };

        let response: GoveeStatusResponse = match response.into_json::<GoveeStatusResponse>() {
            Ok(response) => response,
            Err(e) => {
                println!("Error: {}", e);
                return None;
            }
        };

        let mut temperature: f32 = 0.0;
        let mut humidity: f32 = 0.0;
        let mut online: bool = false;

        if let Some(capabilities) = response.payload.capabilities {
            for capability in capabilities {
                match capability.instance {
                    InstanceType::Temperature => {
                        if let Some(Value {
                            value: Data::Temperature(temp),
                        }) = capability.state
                        {
                            temperature = farenheit_to_celsius(temp);
                        }
                    }
                    InstanceType::Humidity => {
                        if let Some(Value {
                            value: Data::Humidity(hum),
                        }) = capability.state
                        {
                            humidity = hum.current_humidity;
                        }
                    }
                    InstanceType::Online => {
                        if let Some(Value {
                            value: Data::Bool(online_status),
                        }) = capability.state
                        {
                            online = online_status;
                        }
                    }
                }
            }
        } else {
            return None;
        }

        Some(SensorData {
            device_name: self.device_name.clone(),
            timestamp: chrono::Utc::now(),
            temperature,
            humidity,
            online,
        })
    }
}

fn farenheit_to_celsius(farenheit: f32) -> f32 {
    (farenheit - 32.0) * 5.0 / 9.0
}
