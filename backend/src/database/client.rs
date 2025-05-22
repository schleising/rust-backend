use std::process::exit;

use serde::Serialize;

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
            log::error!("Failed to ping MongoDB: {}", e);
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

impl<T> MongoClient<T> {
    /// Creates a new MongoClient instance with the specified database and collection names.
    pub fn new(database_name: &str, collection_name: &str) -> Result<Self, DatabaseError> {
        Ok(MongoClient {
            client: MONGO_CLIENT.clone(),
            database_name: database_name.to_string(),
            collection_name: collection_name.to_string(),
            _marker: std::marker::PhantomData,
        })
    }
}

impl<T> Storage<T> for MongoClient<T>
where
    T: Serialize + Send + Sync + 'static,
{
    type Error = DatabaseError;
    fn save_item(&self, data: T) -> Result<(), Self::Error> {
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

    fn save_items(&self, data: Vec<T>) -> Result<(), Self::Error> {
        log::debug!("Saving items to MongoDB");

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
}
