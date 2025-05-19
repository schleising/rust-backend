use serde::Serialize;

use crate::sensor_control::errors::SensorError;

pub trait TempWriter {
    fn write_temps<T: Serialize + Send + Sync>(&self, data: Vec<T>) -> Result<(), SensorError>;
}
