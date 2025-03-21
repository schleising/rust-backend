use std::thread;

use thiserror::Error;

use ureq::tls::TlsConfig;

use mongodb::{
    IndexModel,
    options::{IndexOptions, InsertManyOptions},
};

use crate::sensors::models::{
    DeviceList, HUE_APPLICATION_KEY_HEADER, HUE_DEVICE_URL, HUE_DISCOVERY_URL, HUE_TEMPERATURE_URL,
    HueBridge, HueTemperatureList, TemperatureData,
};

#[derive(Error, Debug)]
pub enum SensorError {
    #[error("Ureq Request Error: {0}")]
    UreqError(#[from] ureq::Error),
    #[error("MongoDB Error: {0}")]
    MongoDBError(#[from] mongodb::error::Error),
}

#[derive(Debug)]
pub struct Sensor {
    id: String,
    name: String,
}

#[derive(Debug)]
pub struct Sensors {
    bridge_ip_address: String,
    hue_application_key: String,
    sensors: Vec<Sensor>,
    sensors_collection: mongodb::sync::Collection<TemperatureData>,
}

impl Sensors {
    pub fn new(hue_application_key: &str) -> Result<Self, SensorError> {
        log::trace!("Creating new Sensors");

        // Get the IP address of the Hue bridge
        let bridge_ip_address = Sensors::get_bridge()?;
        log::trace!("Bridge IP: {}", bridge_ip_address);

        // Get the sensors from the Hue bridge
        let sensor_list = Sensors::get_sensors(&bridge_ip_address, &hue_application_key)?;
        log::trace!("Sensors: {:?}", sensor_list);

        // Connect to the MongoDB database
        let client = mongodb::sync::Client::with_uri_str("mongodb://host.docker.internal:27017")?;

        // Get the database
        let database = client.database("web_database");

        // Get the collection
        let collection: mongodb::sync::Collection<TemperatureData> =
            database.collection("sensor_data");

        // Create a compound unique index on the device_name and timestamp fields
        let index_model = IndexModel::builder()
            .keys(mongodb::bson::doc! {
                "device_name": 1,
                "timestamp": 1,
            })
            .options(IndexOptions::builder().unique(true).build())
            .build();

        collection.create_index(index_model).run()?;

        // Create a new Sensors struct
        let sensors = Sensors {
            bridge_ip_address,
            hue_application_key: hue_application_key.to_string(),
            sensors: sensor_list,
            sensors_collection: collection,
        };

        // Return the Sensors struct
        Ok(sensors)
    }

    pub fn run(self) -> thread::JoinHandle<()> {
        // Spawn a new thread to get the temperature
        let handle = thread::spawn(move || {
            loop {
                match self.get_temperatures() {
                    Ok(temperatures) => {
                        log::debug!("Temperatures: {:?}", temperatures);
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
        });

        // Return the JoinHandle
        return handle;
    }

    fn get_bridge() -> Result<String, SensorError> {
        log::trace!("Getting bridge");

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
        log::trace!("Getting sensors");
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
                    .expect("Critical error: no temperature service found despite filtering")
                    .rid
                    .clone(),
                name: device.metadata.name.clone(),
            })
            .collect();

        // Return the vector of Sensor structs
        Ok(sensors)
    }

    fn get_temperatures(&self) -> Result<Vec<TemperatureData>, SensorError> {
        log::trace!("Getting temperatures");

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
                    .expect("Critical error: no sensor found for temperature")
                    .name
                    .clone(),
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
        log::trace!("Storing temperatures");

        let insert_options = InsertManyOptions::builder().ordered(false).build(); // Continue on error

        // Insert the temperatures into the MongoDB collection
        match self.sensors_collection
            .insert_many(temperatures)
            .with_options(insert_options)
            .run() {
                Ok(result) => {
                    log::debug!("Inserted {} documents", result.inserted_ids.len());
                }
                Err(error) => {
                    log::trace!("Error inserting documents: {}", error);
                }
            }

        // Return Ok
        Ok(())
    }
}
