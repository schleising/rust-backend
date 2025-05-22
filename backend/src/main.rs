use std::io::Write;

mod datastore;

mod database;
use database::client::MongoClient;

mod sensor_control;
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

    log::info!("Reading Hue Application Key from file");

    // Read the Hue Application Key from the file
    let hue_application_key = match std::fs::read_to_string("secrets/hue_application_key.txt") {
        Ok(key) => key,
        Err(error) => {
            log::error!("Error reading Hue Application Key: {}", error);
            return;
        }
    };

    log::info!("Creating Writer");

    // Create a new MongoClient struct
    let mongo_client = match MongoClient::new(DATABASE_NAME, COLLECTION_NAME) {
        // If the MongoClient was successfully created, store it in the variable
        Ok(client) => client,
        // If there was an error creating the MongoClient, print the error message and exit
        Err(error) => {
            log::error!("Error creating MongoClient: {}", error);
            return;
        }
    };

    log::info!("Created MongoClient");

    log::info!("Creating Sensors");

    // Create a new Sensors struct
    let sensors = match Sensors::new(&hue_application_key, mongo_client) {
        // If the IP address was successfully retrieved, store it in the variable
        Ok(sensors) => sensors,
        // If there was an error retrieving the IP address, print the error message and exit
        Err(error) => {
            log::error!("{}", error);
            return;
        }
    };

    log::info!("Created Sensors");
    log::info!("Starting Sensors");

    // Run the sensors
    let handle = match sensors.run() {
        // If the thread was successfully spawned, store it in the variable
        Ok(handle) => handle,
        // If there was an error spawning the thread, print the error message and exit
        Err(error) => {
            log::error!("Error starting sensors: {}", error);
            return;
        }
    };

    log::info!("Sensors started");
    log::info!("Waiting for Sensors to finish");

    // Wait for the thread to finish
    handle.join().unwrap();

    log::info!("Sensors finished");
    log::info!("Exiting");
}
