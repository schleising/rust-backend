use serde::Serialize;

#[allow(dead_code)]
pub trait Storage<T>
where
    T: Serialize + Send + Sync + 'static,
{
    type Error;
    fn save_item(&self, data: T) -> Result<(), Self::Error>;
    fn save_items(&self, data: Vec<T>) -> Result<(), Self::Error>;
}
