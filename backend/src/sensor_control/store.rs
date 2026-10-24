use chrono::{DateTime, Utc};
use std::collections::HashMap;

use crate::database::errors::DatabaseError;
use crate::datastore::storage::Storage;

use super::models::{HeatingSample, TemperatureData};

/// Persist readings that are newer than the latest stored timestamp per `device_name`.
pub fn store_temperatures<T>(
    data_store: &T,
    temperatures: Vec<TemperatureData>,
) -> Result<(), DatabaseError>
where
    T: Storage<TemperatureData, Error = DatabaseError>,
{
    log::debug!("Storing temperatures");

    let latest_items = data_store.get_latest_items("device_name", "timestamp")?;

    log::trace!("Latest items: {latest_items:?}");

    let temp_map = latest_items
        .into_iter()
        .map(|item| (item.device_name.clone(), item.timestamp))
        .collect::<HashMap<String, DateTime<Utc>>>();

    log::trace!("Temp map: {temp_map:?}");

    let temperatures: Vec<TemperatureData> = temperatures
        .into_iter()
        .filter(|temp| {
            !temp_map.contains_key(&temp.device_name)
                || temp_map[&temp.device_name] < temp.timestamp
        })
        .collect();

    if !temperatures.is_empty() {
        data_store.save_items(&temperatures)?;
        log::debug!("Stored {} new temperature record(s)", temperatures.len());
    } else {
        log::debug!("No new temperatures to store");
    }

    Ok(())
}

/// Insert one heating heartbeat per sample.
///
/// A sample is written on every successful poll, including when `hvac_status`
/// did not change. The gap between heartbeats is a missed poll; a steady status
/// must not look like a gap. This does not use the temperature "newer than latest"
/// filter.
pub fn store_heating_samples<T>(
    data_store: &T,
    samples: Vec<HeatingSample>,
) -> Result<(), DatabaseError>
where
    T: Storage<HeatingSample, Error = DatabaseError>,
{
    if samples.is_empty() {
        log::debug!("No heating samples to store");
        return Ok(());
    }

    data_store.save_items(&samples)?;
    log::debug!("Stored {} heating sample(s)", samples.len());
    Ok(())
}
