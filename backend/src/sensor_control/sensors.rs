use std::thread;

use ureq::tls::TlsConfig;

use super::errors::SensorError;
use super::models::{DeviceList, HueBridge, HueTemperatureList, TemperatureData};
use super::temp_writer::TempWriter;

#[derive(Debug)]
pub struct Sensor {
    id: String,
    name: String,
}

pub struct Sensors {
    bridge_ip_address: String,
    hue_application_key: String,
    sensors: Vec<Sensor>,
    temp_writer: Box<dyn TempWriter + Send>,
}

const HUE_DOMAIN: &str = "hue-bridge";
pub const HUE_DISCOVERY_URL: &str = "https://discovery.meethue.com/";
pub const HUE_APPLICATION_KEY_HEADER: &str = "hue-application-key";
pub const HUE_DEVICE_URL: &str = "/clip/v2/resource/device";
pub const HUE_TEMPERATURE_URL: &str = "/clip/v2/resource/temperature";

impl Sensors {
    pub fn new(
        hue_application_key: &str,
        temp_writer: Box<dyn TempWriter + Send>,
    ) -> Result<Self, SensorError> {
        log::trace!("Creating new Sensors");

        // Get the IP address of the Hue bridge
        let bridge_ip_address = Sensors::get_bridge()?;
        log::debug!("Bridge IP: {}", bridge_ip_address);

        // Get the sensors from the Hue bridge
        let sensor_list = Sensors::get_sensors(&bridge_ip_address, hue_application_key)?;
        log::trace!("Sensors: {:?}", sensor_list);

        // Create a new Sensors struct
        let sensors = Sensors {
            bridge_ip_address,
            hue_application_key: hue_application_key.to_string(),
            sensors: sensor_list,
            temp_writer,
        };

        // Return the Sensors struct
        Ok(sensors)
    }

    pub fn run(self) -> thread::JoinHandle<()> {
        // Spawn a new thread to get the temperature
        thread::spawn(move || {
            loop {
                match self.get_temperatures() {
                    Ok(temperatures) => {
                        log::trace!("Temperatures: {:?}", temperatures);
                        match self.store_temperatures(temperatures) {
                            Ok(_) => {
                                log::debug!("Stored temperatures");
                            }
                            Err(error) => {
                                log::error!("Error storing temperatures: {}", error);
                            }
                        }
                    }
                    Err(error) => {
                        log::error!("Error getting temperatures: {}", error);
                    }
                }

                thread::sleep(std::time::Duration::from_secs(60));
            }
        })
    }

    fn get_bridge() -> Result<String, SensorError> {
        log::info!("Getting bridge");

        // Try getting the config from hue-bridge
        let response = ureq::get(format!("http://{}/api/0/config", HUE_DOMAIN)).call()?;

        if response.status() == 200 {
            log::info!("Got response from hue-bridge");
            return Ok(HUE_DOMAIN.to_string());
        }

        log::info!("No response from hue-bridge, trying discovery");

        // Make a GET request to the Hue discovery URL
        let mut response = ureq::get(HUE_DISCOVERY_URL).call()?;

        log::trace!("Got response");

        // Parse the response body into a Vec<HueBridge>
        let body = response.body_mut().read_json::<Vec<HueBridge>>()?;

        log::trace!("Parsed body");

        // Get the IP address of the first bridge in the list
        let ip_address = &body[0].internalipaddress;

        // Return the IP address as a String
        Ok(ip_address.to_string())
    }

    fn get_sensors(
        bridge_ip_address: &str,
        hue_application_key: &str,
    ) -> Result<Vec<Sensor>, SensorError> {
        log::debug!("Getting sensors");
        let hue_device_url = format!("https://{}{}", bridge_ip_address, HUE_DEVICE_URL);
        log::debug!("Hue Device URL: {}", hue_device_url);

        // Make a GET request to the Hue device URL
        let mut response = ureq::get(&hue_device_url)
            .header(HUE_APPLICATION_KEY_HEADER, hue_application_key)
            .config()
            .tls_config(TlsConfig::builder().disable_verification(true).build())
            .build()
            .call()?;
        log::trace!("Got response");

        // Parse the response body into a Device struct
        let body = response.body_mut().read_json::<DeviceList>()?;

        log::trace!("Parsed body");

        // Create a vector of Sensor structs filtering out sensors that are not temperature sensors (services[n].rtype == "temperature")
        let sensors: Vec<Sensor> = body
            .data
            .iter()
            .filter(|device| {
                device
                    .services
                    .iter()
                    .any(|service| service.rtype == "temperature")
            })
            .map(|device| Sensor {
                id: device
                    .services
                    .iter()
                    .find(|service| service.rtype == "temperature")
                    .map(|service| service.rid.clone())
                    .unwrap_or_else(|| {
                        log::warn!(
                            "No temperature service found for device: {}",
                            device.metadata.name
                        );
                        String::new()
                    }),
                name: device.metadata.name.clone(),
            })
            .collect();

        // Find length of the name of the sensor with the longest name
        let max_name_length = sensors
            .iter()
            .map(|sensor| sensor.name.len())
            .max()
            .unwrap_or(0);

        // Log the sensors
        log::info!("Sensors:");
        log::info!(
            "{}{:width$} - {:36}",
            "Name",
            "",
            "Sensor ID",
            width = max_name_length - "Name".len()
        );
        for sensor in &sensors {
            log::info!(
                "{:width$} - {}",
                sensor.name,
                sensor.id,
                width = max_name_length
            );
        }

        // Log the number of sensors
        log::info!("Number of sensors: {}", sensors.len());

        log::trace!("Sensors: {:?}", sensors);

        // Return the vector of Sensor structs
        Ok(sensors)
    }

    fn get_temperatures(&self) -> Result<Vec<TemperatureData>, SensorError> {
        log::debug!("Getting temperatures");

        let hue_temperature_url =
            format!("https://{}{}", self.bridge_ip_address, HUE_TEMPERATURE_URL);

        log::debug!("Hue Temperature URL: {}", hue_temperature_url);

        // Request the temperature data for all sensors
        let mut response = ureq::get(hue_temperature_url)
            .header(HUE_APPLICATION_KEY_HEADER, &self.hue_application_key)
            .config()
            .tls_config(TlsConfig::builder().disable_verification(true).build())
            .build()
            .call()?;
        log::trace!("Got response");

        // Log the response body
        // log::debug!("Response body: {:?}", response.body_mut().read_to_string());

        // Parse the response body into a Temperatures struct
        let body = response.body_mut().read_json::<HueTemperatureList>()?;

        log::trace!("Parsed body");

        // Create a vector of Temperature structs
        let temperatures: Vec<TemperatureData> = body
            .data
            .iter()
            .map(|temperature| TemperatureData {
                device_name: self
                    .sensors
                    .iter()
                    .find(|sensor| sensor.id == temperature.id)
                    .map(|sensor| sensor.name.as_str())
                    .unwrap_or_else(|| {
                        log::warn!("Sensor not found for temperature data: {}", temperature.id);
                        "Unknown"
                    }),
                online: true,
                timestamp: temperature.temperature.temperature_report.changed,
                temperature: temperature.temperature.temperature_report.temperature,
                humidity: 0.0,
            })
            .collect();

        // Return the vector of Temperature structs
        Ok(temperatures)
    }

    fn store_temperatures(&self, temperatures: Vec<TemperatureData>) -> Result<(), SensorError> {
        log::debug!("Storing temperatures");

        // Store the temperatures in the database
        self.temp_writer.write_temps(temperatures)?;

        // Return Ok
        Ok(())
    }
}
