use serde::{de::DeserializeOwned, Serialize};

#[allow(dead_code)]
pub trait Storage<T>
where
    T: Serialize + DeserializeOwned + Unpin + Send + Sync + 'static,
{
    type Error;
    fn save_item(&self, data: &T) -> Result<(), Self::Error>;
    fn save_items(&self, data: &[T]) -> Result<(), Self::Error>;
    fn get_latest_items(&self, name_field: &str, timestamp_field: &str) -> Result<Vec<T>, Self::Error>;
}
