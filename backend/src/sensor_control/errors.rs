use thiserror::Error;

#[derive(Error, Debug)]
pub enum SensorError {
    #[error("Ureq Request Error: {0}")]
    UreqError(#[from] ureq::Error),
    #[error("MongoDB Error: {0}")]
    MongoDBError(#[from] mongodb::error::Error),
}
