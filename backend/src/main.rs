use std::io::Write;

mod datastore;

mod database;
use database::client::MongoClient;

mod sensor_control;
use mongodb::{IndexModel, options::IndexOptions};
use sensor_control::nest::NestThermostat;
use sensor_control::sensors::Sensors;

const DATABASE_NAME: &str = "web_database";
const COLLECTION_NAME: &str = "sensor_data";

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

    // Create a compound unique index on the device_name and timestamp fields
    let index_model = IndexModel::builder()
        .keys(mongodb::bson::doc! {
            "device_name": 1,
            "timestamp": -1,
        })
        .options(IndexOptions::builder().unique(true).build())
        .build();

    match hue_mongo_client
        .get_collection()
        .create_index(index_model)
        .run()
    {
        Ok(_) => log::info!("Index created successfully"),
        Err(e) => {
            log::error!("Error creating index: {e}");
            return;
        }
    }

    log::info!("Created MongoClient");

    // Separate client for Nest (shares the underlying Mongo connection pool)
    let nest_mongo_client = match MongoClient::new(DATABASE_NAME, COLLECTION_NAME) {
        Ok(client) => client,
        Err(error) => {
            log::error!("Error creating Nest MongoClient: {error}");
            return;
        }
    };

    log::info!("Creating Hue Sensors");

    let sensors = match Sensors::new(&hue_application_key, hue_mongo_client) {
        Ok(sensors) => sensors,
        Err(error) => {
            log::error!("{error}");
            return;
        }
    };

    log::info!("Creating Nest Thermostat");

    let nest = match NestThermostat::new(nest_mongo_client) {
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

    hue_handle.join().unwrap();
    nest_handle.join().unwrap();

    log::info!("Sensors finished");
    log::info!("Exiting");
}
