use std::collections::HashMap;

use chrono::{DateTime, Utc};

use super::models::{HeatingReading, HeatingSample};

/// Keep the first sample in each stretch where the device and reading stay the same.
///
/// Samples are ordered by `device_name`, then `timestamp`. A later sample with the
/// same reading is dropped, including across a hole in the heartbeats. A missing
/// threshold matches only another missing threshold.
pub fn collapse_heating_changes(mut samples: Vec<HeatingSample>) -> Vec<HeatingSample> {
    samples.sort_by(|left, right| {
        left.device_name
            .cmp(&right.device_name)
            .then(left.timestamp.cmp(&right.timestamp))
    });

    let mut kept = Vec::new();
    let mut previous: Option<(String, HeatingReading)> = None;
    for sample in samples {
        let reading = sample.reading();
        let unchanged = previous
            .as_ref()
            .is_some_and(|(device_name, previous_reading)| {
                device_name == &sample.device_name && previous_reading == &reading
            });
        if unchanged {
            continue;
        }
        previous = Some((sample.device_name.clone(), reading));
        kept.push(sample);
    }
    kept
}

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
    fn collapse_keeps_a_row_when_any_field_changes() -> Result<(), String> {
        let samples = vec![
            sample(
                "Nest",
                at(12, 0)?,
                "HEATING",
                Some("HEAT"),
                Some(20.0),
                None,
            ),
            sample(
                "Nest",
                at(12, 5)?,
                "HEATING",
                Some("HEAT"),
                Some(20.0),
                None,
            ),
            sample(
                "Nest",
                at(12, 10)?,
                "HEATING",
                Some("HEAT"),
                Some(21.0),
                None,
            ),
            sample(
                "Nest",
                at(12, 15)?,
                "HEATING",
                Some("HEAT"),
                Some(21.0),
                None,
            ),
            sample("Nest", at(12, 20)?, "OFF", Some("HEAT"), Some(21.0), None),
            sample(
                "Nest",
                at(14, 30)?,
                "COOLING",
                Some("HEATCOOL"),
                Some(18.0),
                Some(24.0),
            ),
            sample(
                "Nest",
                at(14, 35)?,
                "COOLING",
                Some("HEATCOOL"),
                Some(18.0),
                Some(24.0),
            ),
            sample(
                "Nest",
                at(14, 40)?,
                "COOLING",
                Some("HEATCOOL"),
                Some(18.0),
                Some(23.0),
            ),
            sample(
                "Nest",
                at(14, 45)?,
                "OFF",
                Some("HEATCOOL"),
                Some(18.0),
                Some(23.0),
            ),
        ];

        let kept = collapse_heating_changes(samples);
        let timestamps: Vec<_> = kept.into_iter().map(|sample| sample.timestamp).collect();
        assert_eq!(
            timestamps,
            vec![
                at(12, 0)?,
                at(12, 10)?,
                at(12, 20)?,
                at(14, 30)?,
                at(14, 40)?,
                at(14, 45)?
            ]
        );
        Ok(())
    }

    #[test]
    fn collapse_does_not_invent_a_row_for_a_hole() -> Result<(), String> {
        let samples = vec![
            sample(
                "Nest",
                at(12, 0)?,
                "HEATING",
                Some("HEAT"),
                Some(20.0),
                None,
            ),
            sample(
                "Nest",
                at(12, 5)?,
                "HEATING",
                Some("HEAT"),
                Some(20.0),
                None,
            ),
            sample(
                "Nest",
                at(13, 30)?,
                "HEATING",
                Some("HEAT"),
                Some(20.0),
                None,
            ),
            sample("Nest", at(13, 35)?, "OFF", Some("HEAT"), Some(20.0), None),
        ];

        let kept = collapse_heating_changes(samples);
        assert_eq!(kept.len(), 2);
        assert_eq!(kept[0].timestamp, at(12, 0)?);
        assert_eq!(kept[1].timestamp, at(13, 35)?);
        assert_eq!(kept[1].hvac_status, "OFF");
        Ok(())
    }

    #[test]
    fn collapse_keeps_each_device_separate() -> Result<(), String> {
        let samples = vec![
            sample(
                "Hall",
                at(12, 0)?,
                "HEATING",
                Some("HEAT"),
                Some(20.0),
                None,
            ),
            sample(
                "Kitchen",
                at(12, 1)?,
                "HEATING",
                Some("HEAT"),
                Some(20.0),
                None,
            ),
            sample(
                "Hall",
                at(12, 5)?,
                "HEATING",
                Some("HEAT"),
                Some(20.0),
                None,
            ),
        ];

        let kept = collapse_heating_changes(samples);
        assert_eq!(kept.len(), 2);
        assert_eq!(kept[0].device_name, "Hall");
        assert_eq!(kept[1].device_name, "Kitchen");
        Ok(())
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
