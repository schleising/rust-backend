use thiserror::Error;

#[derive(Error, Debug)]
pub enum DatabaseError {
    #[error("MongoDB Error: {0}")]
    MongoDB(#[from] mongodb::error::Error),
}
