use serde::Serialize;

use crate::sensor_control::errors::SensorError;
use crate::sensor_control::temp_writer::TempWriter;

use mongodb::{
    IndexModel,
    options::{IndexOptions, InsertManyOptions},
};

pub struct DatabaseWriter {
    client: mongodb::sync::Client,
    database_name: String,
    collection_name: String,
}

impl DatabaseWriter {
    pub fn new<T>(
        database_url: &str,
        database_name: &str,
        collection_name: &str,
    ) -> Result<Self, SensorError>
    where
        T: Serialize + Send + Sync,
    {
        // Connect to the MongoDB database
        let client = mongodb::sync::Client::with_uri_str(database_url)?;

        // Get the database
        let database = client.database(database_name);

        // Get the collection
        let collection = database.collection::<T>(collection_name);

        // Create a compound unique index on the device_name and timestamp fields
        let index_model = IndexModel::builder()
            .keys(mongodb::bson::doc! {
                "device_name": 1,
                "timestamp": 1,
            })
            .options(IndexOptions::builder().unique(true).build())
            .build();

        collection.create_index(index_model).run()?;

        Ok(DatabaseWriter {
            client,
            database_name: database_name.to_string(),
            collection_name: collection_name.to_string(),
        })
    }
}

impl TempWriter for DatabaseWriter {
    fn write_temps<T>(&self, data: Vec<T>) -> Result<(), SensorError>
    where
        T: Serialize + Send + Sync,
    {
        log::debug!("Storing temperatures");

        let insert_options = InsertManyOptions::builder().ordered(false).build(); // Continue on error

        // Get the MongoDB collection
        let collection = self
            .client
            .database(&self.database_name)
            .collection::<T>(&self.collection_name);

        // Insert the temperatures into the MongoDB collection
        match collection
            .insert_many(data)
            .with_options(insert_options)
            .run()
        {
            Ok(result) => {
                log::debug!("Inserted {} documents", result.inserted_ids.len());
            }
            Err(error) => {
                log::trace!("Error inserting documents: {}", error);
            }
        }

        // Return Ok
        Ok(())
    }
}
