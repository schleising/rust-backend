use thiserror::Error;

#[derive(Error, Debug)]
pub enum SensorError {
    #[error("Ureq Request Error: {0}")]
    Ureq(#[from] ureq::Error),
    #[error("MongoDB Error: {0}")]
    MongoDB(#[from] mongodb::error::Error),
    #[error("File IO Error: {0}")]
    FileIO(#[from] std::io::Error),
}
