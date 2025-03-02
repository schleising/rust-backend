use std::io::Write;

use env_logger;
use log;

mod sensors;
use sensors::sensors::Sensors;

fn main() {
    // Initialize the logger
    env_logger::Builder::new()
        .filter(Some("rust_backend"), log::LevelFilter::Debug)
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

    // Get the IP address of the Hue bridge
    let sensors = Sensors::new(&hue_application_key);

    // Check if the IP address was successfully retrieved
    let sensors = match sensors {
        // If the IP address was successfully retrieved, store it in the variable
        Ok(sensors) => sensors,
        // If there was an error retrieving the IP address, print the error message and exit
        Err(error) => {
            log::error!("{}", error);
            return;
        }
    };

    // Get the temperature from the sensors
    let temperature = sensors.get_temperatures();

    // Check if the temperature was successfully retrieved
    let temperature = match temperature {
        // If the temperature was successfully retrieved, store it in the variable
        Ok(temperature) => temperature,
        // If there was an error retrieving the temperature, print the error message and exit
        Err(error) => {
            log::error!("{}", error);
            return;
        }
    };

    // Pretty print the temperature
    log::info!("Temperature: {:#?}", temperature);
}
