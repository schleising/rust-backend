use std::{fs::File, io::Write, path::Path};

use super::errors::SensorError;
use super::models::TemperatureData;
use super::temp_writer::TempWriter;

pub struct FileWriter {
    filename: String,
}

impl FileWriter {
    pub fn new(filename: &str) -> Result<Self, SensorError> {
        // If the file doesn't exist, create it
        if !Path::new(filename).exists() {
            File::create(filename)?;
        }
        Ok(FileWriter {
            filename: filename.to_string(),
        })
    }
}

impl TempWriter for FileWriter {
    fn write_temps(&self, data: Vec<TemperatureData>) -> Result<(), SensorError> {
        log::debug!("Writing temperatures to file");

        // Open the file in append mode
        let file = File::options().append(true).open(&self.filename)?;

        // Write the temperature data to the file in CSV format
        // Write the csv header if the file is empty
        if file.metadata()?.len() == 0 {
            writeln!(&file, "device_name,timestamp,temperature")?;
        }
        for temp in data {
            writeln!(
                &file,
                "{},{},{}",
                temp.device_name, temp.timestamp, temp.temperature
            )?;
        }

        Ok(())
    }
}
