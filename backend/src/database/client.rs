use std::process::exit;

use serde::{Serialize, de::DeserializeOwned};

use mongodb::sync::Client;

use once_cell::sync::Lazy;

use super::errors::DatabaseError;

use crate::datastore::storage::Storage;

// Default MongoDB URL
const MONGO_URL: &str = "mongodb://localhost:27017";

// A singleton MongoDB client that is initialized once and reused across the application.
static MONGO_CLIENT: Lazy<Client> = Lazy::new(|| {
    // Get the MongoDB URL from the environment variable or use the default
    log::info!("Initializing MongoDB client");
    let database_url = match std::env::var("MONGO_URL") {
        Ok(url) => url,
        Err(_) => {
            log::debug!("MONGO_URL environment variable not set, using default");
            MONGO_URL.to_string()
        }
    };

    // Set up MongoDB client options with a timeout of 5 seconds
    let mut client_options = mongodb::options::ClientOptions::parse(database_url)
        .run()
        .expect("Failed to parse MongoDB URI");
    client_options.server_selection_timeout = Some(std::time::Duration::new(5, 0));
    client_options.connect_timeout = Some(std::time::Duration::new(5, 0));

    // Create a new MongoDB client with the options
    let client = mongodb::sync::Client::with_options(client_options)
        .expect("Failed to create MongoDB client");

    // Ping the client to ensure it's connected
    match client
        .database("admin")
        .run_command(mongodb::bson::doc! { "ping": 1 })
        .run()
    {
        Ok(_) => log::info!("MongoDB connection established"),
        Err(e) => {
            log::error!("Failed to ping MongoDB: {e}");
            exit(1)
        }
    };

    // Return the client
    client
});

/// A MongoDB client that implements the Storage trait.
pub struct MongoClient<T> {
    client: Client,
    database_name: String,
    collection_name: String,
    _marker: std::marker::PhantomData<T>,
}

impl<T> MongoClient<T>
where
    T: Serialize + Send + Sync + 'static,
{
    /// Creates a new MongoClient instance with the specified database and collection names.
    pub fn new(database_name: &str, collection_name: &str) -> Result<Self, DatabaseError> {
        log::info!("Creating MongoClient for database: {database_name}, collection: {collection_name}");
        Ok(MongoClient {
            client: MONGO_CLIENT.clone(),
            database_name: database_name.to_string(),
            collection_name: collection_name.to_string(),
            _marker: std::marker::PhantomData,
        })
    }

    /// Get the collection from the MongoDB client
    pub fn get_collection(&self) -> mongodb::sync::Collection<T> {
        self.client
            .database(&self.database_name)
            .collection::<T>(&self.collection_name)
    }
}

impl<T> Storage<T> for MongoClient<T>
where
    T: Serialize + DeserializeOwned + Unpin + Send + Sync + 'static,
{
    type Error = DatabaseError;
    fn save_item(&self, data: &T) -> Result<(), Self::Error> {
        log::debug!("Saving item to MongoDB");

        // Get the collection from the MongoDB client
        let collection = self
            .client
            .database(&self.database_name)
            .collection::<T>(&self.collection_name);

        // Insert the data into the collection
        collection.insert_one(data).run()?;
        log::debug!("Item saved to MongoDB");
        Ok(())
    }

    fn save_items(&self, data: &[T]) -> Result<(), Self::Error> {
        log::debug!("Saving items to MongoDB");

        // Check if the data is empty, if so, return early to avoid an empty insert operation error
        if data.is_empty() {
            log::debug!("No items to save to MongoDB");
            return Ok(());
        }

        // Get the collection from the MongoDB client
        let collection = self
            .client
            .database(&self.database_name)
            .collection::<T>(&self.collection_name);

        // Set the insert options to allow unordered inserts, this ensures that all items are inserted even if some fail
        let insert_options = mongodb::options::InsertManyOptions::builder()
            .ordered(false)
            .build();

        // Insert the data into the collection
        collection
            .insert_many(data)
            .with_options(insert_options)
            .run()?;

        log::debug!("Items saved to MongoDB");

        Ok(())
    }

    fn get_latest_items(
        &self,
        name_field: &str,
        timestamp_field: &str,
    ) -> Result<Vec<T>, Self::Error> {
        log::debug!("Getting latest items from MongoDB");

        // Get the collection from the MongoDB client
        let collection = self
            .client
            .database(&self.database_name)
            .collection::<T>(&self.collection_name);

        // Get all of the unique device names
        let device_names: Vec<String> = collection
            .distinct(name_field, mongodb::bson::doc! {})
            .run()?
            .into_iter()
            .filter_map(|item| item.as_str().map(String::from))
            .collect();

        log::debug!("Device names: {device_names:?}");

        // Prepare a vector to hold the latest items
        let mut items = Vec::new();

        // Iterate over each device name and find the latest item for each
        for device_name in device_names {
            let filter = mongodb::bson::doc! { name_field: &device_name };
            let options = mongodb::options::FindOptions::builder()
                .sort(mongodb::bson::doc! { timestamp_field: -1 })
                .limit(1)
                .build();

            // Find the latest item for the current device name
            let result = collection.find(filter)
                .with_options(options)
                .run()?;

            // If a result is found, push it to the items vector
            if let Some(item) = result.into_iter().next() {
                match item {
                    Ok(item) => items.push(item),
                    Err(e) => {
                        log::error!("Error retrieving item for device {device_name}: {e}");
                        return Err(DatabaseError::from(e));
                    }
                }
            } else {
                log::warn!("No items found for device: {device_name}");
            }
        }

        log::debug!("Latest items retrieved from MongoDB");
        Ok(items)
    }
}
