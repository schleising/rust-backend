use std::collections::HashMap;

use chrono::{DateTime, Utc};

use super::models::{HeatingReading, HeatingSample};

/// Samples whose reading is new or differs from the remembered value for that device.
pub fn select_heating_changes(
    remembered: &HashMap<String, HeatingReading>,
    samples: &[HeatingSample],
) -> Vec<HeatingSample> {
    samples
        .iter()
        .filter(|sample| remembered.get(&sample.device_name) != Some(&sample.reading()))
        .cloned()
        .collect()
}

/// Whether this poll should write Nest temperature rows.
///
/// The first successful poll writes. Later polls write once 300 seconds have elapsed.
pub fn nest_temperature_due(last_written_at: Option<DateTime<Utc>>, now: DateTime<Utc>) -> bool {
    match last_written_at {
        None => true,
        Some(last_written_at) => now - last_written_at >= chrono::Duration::seconds(300),
    }
}

#[cfg(test)]
mod tests {
    use chrono::TimeZone;

    use super::*;

    fn at(hour: u32, minute: u32) -> Result<DateTime<Utc>, String> {
        Utc.with_ymd_and_hms(2026, 10, 24, hour, minute, 0)
            .single()
            .ok_or_else(|| format!("invalid timestamp {hour}:{minute:02}"))
    }

    fn sample(
        device_name: &str,
        timestamp: DateTime<Utc>,
        hvac_status: &str,
        mode: Option<&str>,
        heat_celsius: Option<f32>,
        cool_celsius: Option<f32>,
    ) -> HeatingSample {
        HeatingSample {
            device_name: device_name.to_string(),
            timestamp,
            hvac_status: hvac_status.to_string(),
            mode: mode.map(str::to_string),
            heat_celsius,
            cool_celsius,
        }
    }

    #[test]
    fn a_missing_threshold_does_not_match_zero() -> Result<(), String> {
        let mut remembered = HashMap::new();
        let original = sample("Nest", at(12, 0)?, "OFF", Some("HEAT"), None, None);
        remembered.insert(original.device_name.clone(), original.reading());

        let changed = sample("Nest", at(12, 1)?, "OFF", Some("HEAT"), Some(0.0), None);
        let selected = select_heating_changes(&remembered, &[changed]);
        assert_eq!(selected.len(), 1);
        assert_eq!(selected[0].heat_celsius, Some(0.0));

        let repeated = sample("Nest", at(12, 2)?, "OFF", Some("HEAT"), None, None);
        assert!(select_heating_changes(&remembered, &[repeated]).is_empty());
        Ok(())
    }

    #[test]
    fn temperature_is_due_on_the_first_poll_and_then_every_five_minutes() -> Result<(), String> {
        let first = at(12, 0)?;
        assert!(nest_temperature_due(None, first));
        assert!(!nest_temperature_due(Some(first), at(12, 4)?));
        assert!(nest_temperature_due(Some(first), at(12, 5)?));
        Ok(())
    }
}
