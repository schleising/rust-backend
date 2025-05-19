use super::errors::SensorError;
use super::models::TemperatureData;

pub trait TempWriter {
    fn write_temps(&self, data: Vec<TemperatureData>) -> Result<(), SensorError>;
}
