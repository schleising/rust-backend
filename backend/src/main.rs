use std::{error::Error, sync::Arc};

use chrono;
use mongodb;

mod sensors;

#[allow(dead_code)]
fn get_device_data() {
    let devices = sensors::sensors::Devices::new();

    for device in devices.devices {
        match device.get_data(&devices.api_key) {
            // Print the data if it is available
            Some(data) => {
                println!("{}", data.device_name);
                // println!("Time:        {}", data.timestamp.to_rfc2822());
                println!("Online:      {}", data.online);
                println!("Temperature: {:.1}°C", data.temperature);
                println!("Humidity:    {}%", data.humidity);
            }
            None => println!("Could not get data"),
        }

        println!();
    }
}

#[allow(dead_code)]
fn get_average_humidity(
    start_time: chrono::DateTime<chrono::Utc>,
    end_time: chrono::DateTime<chrono::Utc>,
) -> Result<f32, Box<dyn Error>> {
    println!(
        "Getting average humidity from {} to {}",
        start_time.to_rfc2822(),
        end_time.to_rfc2822()
    );

    // Connect to the database
    let client = mongodb::sync::Client::with_uri_str("mongodb://macmini2:27017")?;
    let db = client.database("web_database");
    let collection: mongodb::sync::Collection<sensors::models::SensorData> =
        db.collection("sensor_data");

    // Test the connection
    db.run_command(mongodb::bson::doc! {"ping": 1}).run()?;

    println!("Connected to database");

    // Filter the data by device name and timestamp
    let filter = mongodb::bson::doc! {
        "device_name": "Bedroom Thermometer",
        "timestamp": {
            "$gte": start_time,
            "$lt": end_time,
        },
    };

    // Get the data from the database
    let cursor = match collection.find(filter).run() {
        Ok(cursor) => cursor,
        Err(e) => return Err(Box::new(e)),
    };

    // Calculate the average humidity
    let mut total_humidity = 0.0;
    let mut count = 0;

    // Iterate over the results and sum the humidity values
    for result in cursor {
        let data = result?;
        total_humidity += data.humidity;
        count += 1;
    }

    if count > 0 {
        // Return the average humidity
        return Ok(total_humidity / count as f32);
    } else {
        // Return 0 if there is no data
        return Err(Box::new(mongodb::bson::de::Error::Io(Arc::new(
            std::io::Error::new(std::io::ErrorKind::NotFound, "No data found"),
        ))));
    }
}

fn main() {
    // Get the average humidity for the last 24 hours
    let end_time = chrono::Utc::now() - chrono::Duration::days(0);
    let start_time = end_time - chrono::Duration::days(7);

    match get_average_humidity(start_time, end_time) {
        Ok(average_humidity) => {
            println!("Average humidity: {:.1}%", average_humidity);
        }
        Err(e) => {
            println!("Error: {}", e);
        }
    }
}
