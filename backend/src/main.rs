use std::io::Write;

mod sensor_control;
use sensor_control::sensors::Sensors;

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

    // Create a new Sensors struct
    let sensors = match Sensors::new(&hue_application_key) {
        // If the IP address was successfully retrieved, store it in the variable
        Ok(sensors) => sensors,
        // If there was an error retrieving the IP address, print the error message and exit
        Err(error) => {
            log::error!("{}", error);
            return;
        }
    };

    // Run the sensors
    let handle = sensors.run();

    // Wait for the thread to finish
    handle.join().unwrap();
}
