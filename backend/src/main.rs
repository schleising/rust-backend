use std::io::Write;

mod datastore;

mod database;
use database::client::MongoClient;
use database::errors::DatabaseError;

mod sensor_control;
use mongodb::{IndexModel, options::IndexOptions};
use sensor_control::HeatingSample;
use sensor_control::nest::NestThermostat;
use sensor_control::sensors::Sensors;
use serde::Serialize;

const DATABASE_NAME: &str = "web_database";
const COLLECTION_NAME: &str = "sensor_data";
const HEATING_COLLECTION_NAME: &str = "heating_samples";

fn ensure_unique_device_timestamp_index<T>(
    client: &MongoClient<T>,
    collection_label: &str,
) -> Result<(), DatabaseError>
where
    T: Serialize + Send + Sync + 'static,
{
    let index_model = IndexModel::builder()
        .keys(mongodb::bson::doc! {
            "device_name": 1,
            "timestamp": -1,
        })
        .options(IndexOptions::builder().unique(true).build())
        .build();

    client.get_collection().create_index(index_model).run()?;
    log::info!("{collection_label} index created successfully");
    Ok(())
}

fn main() {
    // Initialize the logger
    env_logger::Builder::new()
        .filter(Some("rust_backend"), log::LevelFilter::Info)
        .format(|buf, record| {
            writeln!(
                buf,
                "{} : {:5} {} [{}] - {}",
                chrono::Local::now().format("%Y-%m-%dT%H:%M:%S"),
                record.level(),
                record.args(),
                record.file().unwrap_or("unknown"),
                record.line().unwrap_or(0),
            )
        })
        .init();

    // Log the application name and version from Cargo.toml
    log::info!(
        "Starting Rust Backend - Version: {}",
        env!("CARGO_PKG_VERSION")
    );

    log::info!("Reading Hue Application Key from file");

    // Read the Hue Application Key from the file
    let hue_application_key = match std::fs::read_to_string("secrets/hue_application_key.txt") {
        Ok(key) => key,
        Err(error) => {
            log::error!("Error reading Hue Application Key: {error}");
            return;
        }
    };

    log::info!("Creating Writer");

    // Create a new MongoClient struct for Hue
    let hue_mongo_client = match MongoClient::new(DATABASE_NAME, COLLECTION_NAME) {
        Ok(client) => client,
        Err(error) => {
            log::error!("Error creating MongoClient: {error}");
            return;
        }
    };

    if let Err(error) = ensure_unique_device_timestamp_index(&hue_mongo_client, "sensor_data") {
        log::error!("Error creating sensor_data index: {error}");
        return;
    }

    log::info!("Created MongoClient");

    // Separate client for Nest temperatures (shares the underlying Mongo connection pool)
    let nest_mongo_client = match MongoClient::new(DATABASE_NAME, COLLECTION_NAME) {
        Ok(client) => client,
        Err(error) => {
            log::error!("Error creating Nest MongoClient: {error}");
            return;
        }
    };

    // Heating heartbeats live beside sensor_data, not on the temperature document.
    let heating_mongo_client: MongoClient<HeatingSample> =
        match MongoClient::new(DATABASE_NAME, HEATING_COLLECTION_NAME) {
            Ok(client) => client,
            Err(error) => {
                log::error!("Error creating heating samples MongoClient: {error}");
                return;
            }
        };

    if let Err(error) =
        ensure_unique_device_timestamp_index(&heating_mongo_client, "heating_samples")
    {
        log::error!("Error creating heating_samples index: {error}");
        return;
    }

    log::info!("Creating Hue Sensors");

    let sensors = match Sensors::new(&hue_application_key, hue_mongo_client) {
        Ok(sensors) => sensors,
        Err(error) => {
            log::error!("{error}");
            return;
        }
    };

    log::info!("Creating Nest Thermostat");

    let nest = match NestThermostat::new(nest_mongo_client, heating_mongo_client) {
        Ok(nest) => nest,
        Err(error) => {
            log::error!("Error creating Nest Thermostat: {error}");
            return;
        }
    };

    log::info!("Starting Hue Sensors");

    let hue_handle = match sensors.run() {
        Ok(handle) => handle,
        Err(error) => {
            log::error!("Error starting Hue sensors: {error}");
            return;
        }
    };

    log::info!("Starting Nest Thermostat");

    let nest_handle = match nest.run() {
        Ok(handle) => handle,
        Err(error) => {
            log::error!("Error starting Nest Thermostat: {error}");
            return;
        }
    };

    log::info!("Sensors started");
    log::info!("Waiting for sensor threads to finish");

    if let Err(error) = hue_handle.join() {
        log::error!("Hue sensor thread panicked: {error:?}");
    }
    if let Err(error) = nest_handle.join() {
        log::error!("Nest thermostat thread panicked: {error:?}");
    }

    log::info!("Sensors finished");
    log::info!("Exiting");
}
