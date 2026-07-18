use std::sync::Arc;
use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;
use std::time::Duration;

use chrono::{DateTime, Utc};
use signal_hook::flag::register;

use super::errors::SensorError;
use super::models::{
    NestCredentials, NestDeviceList, NestTokenResponse, TemperatureData, THERMOSTAT_TYPE,
};
use super::store;

use crate::database::errors::DatabaseError;
use crate::datastore::storage::Storage;

const NEST_CREDENTIALS_PATH: &str = "secrets/nest_credentials.json";
const TOKEN_URL: &str = "https://oauth2.googleapis.com/token";
const SDM_DEVICES_URL: &str = "https://smartdevicemanagement.googleapis.com/v1/enterprises";
const POLL_INTERVAL_SECS: u64 = 60;
const TOKEN_EXPIRY_SKEW_SECS: i64 = 60;

struct CachedAccessToken {
    token: String,
    expires_at: DateTime<Utc>,
}

pub struct NestThermostat<T> {
    credentials: NestCredentials,
    access_token: Mutex<Option<CachedAccessToken>>,
    data_store: T,
}

impl<T> NestThermostat<T>
where
    T: Storage<TemperatureData, Error = DatabaseError> + Send + 'static,
{
    pub fn new(data_store: T) -> Result<Self, SensorError> {
        log::trace!("Creating NestThermostat");

        let credentials_json = std::fs::read_to_string(NEST_CREDENTIALS_PATH)?;
        let credentials: NestCredentials = serde_json::from_str(&credentials_json)?;

        Ok(NestThermostat {
            credentials,
            access_token: Mutex::new(None),
            data_store,
        })
    }

    pub fn run(self) -> Result<thread::JoinHandle<()>, SensorError> {
        let term = Arc::new(AtomicBool::new(false));
        register(signal_hook::consts::SIGTERM, Arc::clone(&term))?;
        register(signal_hook::consts::SIGINT, Arc::clone(&term))?;

        Ok(thread::spawn(move || {
            while !term.load(Ordering::Relaxed) {
                match self.poll_once() {
                    Ok(()) => log::debug!("Nest poll complete"),
                    Err(error) => log::error!("Error polling Nest thermostat: {error}"),
                }

                thread::sleep(Duration::from_secs(POLL_INTERVAL_SECS));
            }
        }))
    }

    fn poll_once(&self) -> Result<(), SensorError> {
        let readings = self.get_temperatures()?;
        if let Err(error) = store::store_temperatures(&self.data_store, readings) {
            log::error!("Error saving Nest temperatures: {error}");
        }
        Ok(())
    }

    fn invalidate_access_token(&self) {
        let mut guard = self
            .access_token
            .lock()
            .expect("Nest access token mutex poisoned");
        *guard = None;
    }

    fn get_access_token(&self) -> Result<String, SensorError> {
        {
            let guard = self
                .access_token
                .lock()
                .expect("Nest access token mutex poisoned");
            if let Some(cached) = guard.as_ref() {
                let refresh_after =
                    cached.expires_at - chrono::Duration::seconds(TOKEN_EXPIRY_SKEW_SECS);
                if Utc::now() < refresh_after {
                    return Ok(cached.token.clone());
                }
            }
        }

        log::info!("Refreshing Nest access token");
        let mut response = ureq::post(TOKEN_URL).send_form([
            ("client_id", self.credentials.client_id.as_str()),
            ("client_secret", self.credentials.client_secret.as_str()),
            ("refresh_token", self.credentials.refresh_token.as_str()),
            ("grant_type", "refresh_token"),
        ])?;

        let token_response: NestTokenResponse = response.body_mut().read_json()?;
        let expires_at =
            Utc::now() + chrono::Duration::seconds(i64::from(token_response.expires_in));

        let mut guard = self
            .access_token
            .lock()
            .expect("Nest access token mutex poisoned");
        *guard = Some(CachedAccessToken {
            token: token_response.access_token.clone(),
            expires_at,
        });

        Ok(token_response.access_token)
    }

    fn fetch_devices(&self, access_token: &str) -> Result<NestDeviceList, SensorError> {
        let url = format!(
            "{}/{}/devices",
            SDM_DEVICES_URL, self.credentials.project_id
        );

        let mut response = ureq::get(&url)
            .header("Authorization", &format!("Bearer {access_token}"))
            .header("Content-Type", "application/json")
            .call()?;

        Ok(response.body_mut().read_json()?)
    }

    fn get_temperatures(&self) -> Result<Vec<TemperatureData>, SensorError> {
        log::debug!("Getting Nest thermostat readings");

        let access_token = self.get_access_token()?;
        let body = match self.fetch_devices(&access_token) {
            Ok(body) => body,
            Err(SensorError::Ureq(ureq::Error::StatusCode(401))) => {
                log::warn!("Nest API returned 401; forcing token refresh");
                self.invalidate_access_token();
                let access_token = self.get_access_token()?;
                self.fetch_devices(&access_token)?
            }
            Err(error) => return Err(error),
        };

        let now = Utc::now();

        let temperatures: Vec<TemperatureData> = body
            .devices
            .iter()
            .filter(|device| device.device_type == THERMOSTAT_TYPE)
            .filter_map(|device| {
                let temperature = device
                    .traits
                    .temperature
                    .as_ref()
                    .map(|t| t.ambient_temperature_celsius)?;
                let humidity = device
                    .traits
                    .humidity
                    .as_ref()
                    .map(|h| h.ambient_humidity_percent)
                    .unwrap_or(0.0);
                let online = device
                    .traits
                    .connectivity
                    .as_ref()
                    .map(|c| c.status.eq_ignore_ascii_case("ONLINE"))
                    .unwrap_or(false);

                Some(TemperatureData {
                    device_name: device.display_name(),
                    timestamp: now,
                    online,
                    temperature,
                    humidity,
                })
            })
            .collect();

        for reading in &temperatures {
            log::info!(
                "Nest {}: {:.2}°C, {:.0}% RH, online={}",
                reading.device_name,
                reading.temperature,
                reading.humidity,
                reading.online
            );
        }

        Ok(temperatures)
    }
}
