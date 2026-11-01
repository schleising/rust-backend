use std::collections::HashMap;
use std::process::exit;

use mongodb::bson::{Document, doc};
use mongodb::options::ClientOptions;
use mongodb::sync::Client;
use mongodb::{IndexModel, options::IndexOptions};
use rust_backend::sensor_control::HeatingSample;
use rust_backend::sensor_control::collapse_heating_changes;
use thiserror::Error;

const MONGO_URL: &str = "mongodb://macmini2.home.arpa:27017";
const DATABASE_NAME: &str = "web_database";
const HEARTBEATS: &str = "heating_samples";
const CHANGES: &str = "heating_changes";

#[derive(Debug, Error)]
enum MigrateError {
    #[error("MongoDB error: {0}")]
    Mongo(#[from] mongodb::error::Error),
    #[error("BSON error: {0}")]
    Bson(#[from] mongodb::bson::de::Error),
    #[error(
        "heating_changes already exists. Drop that collection by hand before running the migration again"
    )]
    AlreadyMigrated,
    #[error("unexpected argument {0}")]
    UnexpectedArgument(String),
}

fn main() {
    env_logger::Builder::new()
        .filter_level(log::LevelFilter::Info)
        .init();

    if let Err(error) = run() {
        log::error!("{error}");
        exit(1);
    }
}

fn run() -> Result<(), MigrateError> {
    let dry_run = dry_run_flag()?;
    let client = connect()?;
    let database = client.database(DATABASE_NAME);

    let names = database.list_collection_names().run()?;
    if names.iter().any(|name| name == CHANGES) {
        return Err(MigrateError::AlreadyMigrated);
    }

    let heartbeats = read_heartbeats(&database)?;
    let kept = collapse_heating_changes(heartbeats.clone());
    report(&heartbeats, &kept);

    if dry_run {
        log::info!("Dry run: heating_samples was not modified and heating_changes was not created");
        return Ok(());
    }

    let changes = database.collection::<HeatingSample>(CHANGES);
    let index_model = IndexModel::builder()
        .keys(doc! {
            "device_name": 1,
            "timestamp": -1,
        })
        .options(IndexOptions::builder().unique(true).build())
        .build();
    changes.create_index(index_model).run()?;

    if !kept.is_empty() {
        changes.insert_many(kept).run()?;
    }
    log::info!("Wrote heating_changes. heating_samples was left in place");
    Ok(())
}

fn dry_run_flag() -> Result<bool, MigrateError> {
    let mut dry_run = false;
    for argument in std::env::args().skip(1) {
        if argument == "--dry-run" {
            dry_run = true;
        } else {
            return Err(MigrateError::UnexpectedArgument(argument));
        }
    }
    Ok(dry_run)
}

fn connect() -> Result<Client, MigrateError> {
    let options = ClientOptions::parse(MONGO_URL).run()?;
    Ok(Client::with_options(options)?)
}

fn read_heartbeats(database: &mongodb::sync::Database) -> Result<Vec<HeatingSample>, MigrateError> {
    let collection = database.collection::<Document>(HEARTBEATS);
    let options = mongodb::options::FindOptions::builder()
        .sort(doc! { "device_name": 1, "timestamp": 1 })
        .build();
    let cursor = collection.find(doc! {}).with_options(options).run()?;

    let mut samples = Vec::new();
    for document in cursor {
        let document = document?;
        match sample_from_document(document) {
            Ok(Some(sample)) => samples.push(sample),
            Ok(None) => {}
            Err(error) => return Err(error),
        }
    }
    Ok(samples)
}

fn sample_from_document(document: Document) -> Result<Option<HeatingSample>, MigrateError> {
    let id = document
        .get("_id")
        .map(ToString::to_string)
        .unwrap_or_else(|| "<missing _id>".to_string());
    let Some(status) = document_status(&document) else {
        log::error!("Skipping heating sample {id}: hvac_status is missing or blank");
        return Ok(None);
    };

    let mut sample: HeatingSample = mongodb::bson::from_document(document)?;
    sample.hvac_status = status;
    sample.mode = sample
        .mode
        .as_deref()
        .map(str::trim)
        .filter(|mode| !mode.is_empty())
        .map(ToString::to_string);
    Ok(Some(sample))
}

fn document_status(document: &Document) -> Option<String> {
    let mongodb::bson::Bson::String(status) = document.get("hvac_status")? else {
        return None;
    };
    let status = status.trim();
    if status.is_empty() {
        None
    } else {
        Some(status.to_string())
    }
}

fn report(heartbeats: &[HeatingSample], kept: &[HeatingSample]) {
    let mut input_counts: HashMap<&str, usize> = HashMap::new();
    for sample in heartbeats {
        *input_counts.entry(sample.device_name.as_str()).or_insert(0) += 1;
    }

    let mut kept_by_device: HashMap<&str, Vec<&HeatingSample>> = HashMap::new();
    for sample in kept {
        kept_by_device
            .entry(sample.device_name.as_str())
            .or_default()
            .push(sample);
    }

    let mut names: Vec<&str> = input_counts.keys().copied().collect();
    names.sort_unstable();
    if names.is_empty() {
        log::info!("heating_samples has no usable heartbeats");
        return;
    }

    for name in names {
        let input = input_counts[name];
        match kept_by_device.get(name) {
            Some(rows) => {
                let first = &rows[0];
                let last = &rows[rows.len() - 1];
                log::info!(
                    "{name}: {input} heartbeats -> {} changes, first {} {}, last {} {}",
                    rows.len(),
                    first.timestamp,
                    first.hvac_status,
                    last.timestamp,
                    last.hvac_status
                );
            }
            None => log::info!("{name}: {input} heartbeats -> 0 changes"),
        }
    }
}
