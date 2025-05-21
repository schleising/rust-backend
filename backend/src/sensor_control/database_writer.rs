use mongodb::{
    IndexModel,
    options::{IndexOptions, InsertManyOptions},
};

use super::errors::SensorError;
use super::models::TemperatureData;
use super::temp_writer::TempWriter;

pub struct DatabaseWriter {
    client: mongodb::sync::Client,
    database_name: String,
    collection_name: String,
}

impl DatabaseWriter {
    pub fn new(
        database_url: &str,
        database_name: &str,
        collection_name: &str,
    ) -> Result<Self, SensorError> {
        // Connect to the MongoDB database setting a timeout of 5 seconds
        let mut client_options = mongodb::options::ClientOptions::parse(database_url).run()?;
        client_options.server_selection_timeout = Some(std::time::Duration::new(5, 0));
        client_options.connect_timeout = Some(std::time::Duration::new(5, 0));
        let client = mongodb::sync::Client::with_options(client_options)?;

        // Get the database
        let database = client.database(database_name);

        // Get the collection
        let collection = database.collection::<TemperatureData>(collection_name);

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
    fn write_temps(&self, data: Vec<TemperatureData>) -> Result<(), SensorError> {
        log::debug!("Storing temperatures");

        let insert_options = InsertManyOptions::builder().ordered(false).build(); // Continue on error

        // Get the MongoDB collection
        let collection = self
            .client
            .database(&self.database_name)
            .collection::<TemperatureData>(&self.collection_name);

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
