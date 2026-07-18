# Nest Thermostat Temperature & Humidity Integration

## Goal

Poll ambient temperature and humidity from a Google Nest Thermostat and store readings in MongoDB using the **same document shape** as the existing Philips Hue temperature sensors.

No schema migration. Nest rows land in `web_database.sensor_data` alongside Hue rows, distinguished only by `device_name`.

---

## Current state

The backend is a Hue CLIP v2 poller that writes to MongoDB every second.


| Concern       | Today                                                |
| ------------- | ---------------------------------------------------- |
| Source        | Philips Hue Bridge (`/clip/v2/resource/temperature`) |
| Storage       | MongoDB `web_database.sensor_data`                   |
| Identity      | Hue device metadata name → `device_name`             |
| Humidity      | Field exists; always written as `0.0`                |
| Nest / Google | Not implemented                                      |




### Existing document format (`TemperatureData`)

```rust
pub struct TemperatureData {
    pub device_name: String,
    pub timestamp: DateTime<Utc>,
    pub online: bool,
    pub temperature: f32,
    pub humidity: f32,
}
```

Unique index: `{ device_name: 1, timestamp: -1 }`.

Dedup rule: skip insert when the new reading’s `timestamp` is not newer than the latest stored row for that `device_name`.

---



## Proposed Nest mapping

Map SDM thermostat traits onto the same `TemperatureData` fields:


| `TemperatureData` field | Nest SDM source                                                                                                                           |
| ----------------------- | ----------------------------------------------------------------------------------------------------------------------------------------- |
| `device_name`           | `sdm.devices.traits.Info.customName` (fallback: a configured name, e.g. `"Nest Thermostat"`)                                              |
| `timestamp`             | Wall-clock time of the successful poll (`Utc::now()`). SDM GET responses do not include a “last changed” field for ambient temp/humidity. |
| `online`                | `sdm.devices.traits.Connectivity.status == "ONLINE"`                                                                                      |
| `temperature`           | `sdm.devices.traits.Temperature.ambientTemperatureCelsius`                                                                                |
| `humidity`              | `sdm.devices.traits.Humidity.ambientHumidityPercent`                                                                                      |


Example stored document:

```json
{
  "device_name": "Living Room Nest",
  "timestamp": { "$date": "2026-07-18T14:30:00.000Z" },
  "online": true,
  "temperature": 21.5,
  "humidity": 42.0
}
```

Notes:

- Temperature is always Celsius from the API, matching Hue.
- Humidity is a percent (`0.0`–`100.0`), not a fraction.
- Nest remote temperature sensors are **not** exposed by the SDM API — only the thermostat’s own ambient sensors (or whichever sensor is currently “active” in the Nest app for that thermostat).

---



## Architecture sketch

Keep Hue polling as-is. Add a parallel Nest poller that produces the same `TemperatureData` and uses the same `Storage` trait / Mongo client.

```
main
  ├─ MongoClient(web_database, sensor_data) + unique index
  ├─ Hue Sensors::run()          // existing 1s poll
  └─ NestThermostat::run()       // new, e.g. 30–60s poll
         │
         ├─ refresh OAuth access token if expired
         ├─ GET .../enterprises/{project_id}/devices
         ├─ filter type == sdm.devices.types.THERMOSTAT
         ├─ map traits → TemperatureData
         └─ store via Storage (same dedup as Hue)
```

Suggested secrets (alongside `secrets/hue_application_key.txt`):


| File                             | Contents                                    |
| -------------------------------- | ------------------------------------------- |
| `secrets/nest_client_id.txt`     | OAuth 2.0 Client ID                         |
| `secrets/nest_client_secret.txt` | OAuth 2.0 Client Secret                     |
| `secrets/nest_refresh_token.txt` | Long-lived refresh token from one-time auth |
| `secrets/nest_project_id.txt`    | Device Access project UUID                  |


Optional: a single `secrets/nest_credentials.json` instead of four files.

Poll interval: Nest ambient values update slowly; **30–60 seconds** is enough (Hue’s 1s cadence is unnecessary and burns quota).

Access tokens expire after ~1 hour — refresh before each poll (or when a call returns `UNAUTHENTICATED`).

---



## Google setup — enable reading Nest sensor data

Complete these steps **once** with a **personal Gmail account** (not Google Workspace). The Nest device must already be linked to that Google account in the Nest / Google Home app.

Official overview: [Device Access Get Started](https://developers.google.com/nest/device-access/get-started)

### 1. Register for Device Access (US$5, one-time)

1. Open the [Device Access Console](https://console.nest.google.com/device-access).
2. Accept the Google API Terms and Device Access Sandbox Terms.
3. Pay the **one-time, non-refundable US$5** registration fee per Google account.
4. You cannot create a Device Access project until registration and payment succeed.



### 2. Confirm the thermostat is on a Google account

- Supported: all Google Nest Thermostats.
- Legacy Nest-only accounts are not supported — migrate to a Google account first if needed.
- Confirm the thermostat appears and reports temperature/humidity in the Google Home / Nest app under the same Gmail you will use for OAuth.



### 3. Create a Google Cloud project & enable the SDM API

Either use Google’s guided button on the [Get Started](https://developers.google.com/nest/device-access/get-started) page (“Enable the API and get an OAuth 2.0 Client ID”), or do it manually:

1. Go to [Google Cloud Console](https://console.cloud.google.com/) and create (or select) a project.
2. Enable **Smart Device Management API**:
  [API enablement](https://console.developers.google.com/apis/api/smartdevicemanagement.googleapis.com/overview)
3. Configure the OAuth consent screen (External is fine for personal use).
  For personal/sandbox use you do **not** need OAuth verification; the app will show as “unverified” — that is expected.  
   **Critical:** leave Publishing status as **Testing**, then add your Gmail under **Test users** (Google Cloud Console → **APIs & Services → OAuth consent screen → Audience**).  
   Being the GCP project owner is **not** enough — if your account is missing from Test users you get `Error 403: access_denied` (“…can only be accessed by developer-approved testers”).
4. Create OAuth credentials:
  - **APIs & Services → Credentials → Create Credentials → OAuth client ID**
  - Application type: **Web application**
  - Authorized redirect URI: `https://www.google.com`  
  (required so you can capture the auth code from the Partner Connections Manager flow)
5. Copy the **Client ID** and **Client Secret**, and download the JSON credentials file for safekeeping.

Manual credentials page: [Google Cloud Credentials](https://console.developers.google.com/apis/credentials)

### 4. Create a Device Access project

1. Return to the [Device Access Console](https://console.nest.google.com/device-access).
2. **Create project**.
3. Enter a project name.
4. Paste the **OAuth 2.0 Client ID** from step 3 (must be unique to this Device Access project).
5. Events (Pub/Sub): choose **Disable** for the first version (polling is enough). Enable later if you want push updates.
6. Note the assigned **Project ID** (UUID), e.g. `32c4c2bc-fe0d-461b-b51c-f3885afff2f0`.
  This is the Device Access project ID used in API URLs — **not** the Google Cloud project ID.



### 5. Authorize your Google account (one-time OAuth)

Build this URL, substituting your values:

```
https://nestservices.google.com/partnerconnections/PROJECT_ID/auth?redirect_uri=https://www.google.com&access_type=offline&prompt=consent&client_id=OAUTH_CLIENT_ID&response_type=code&scope=https://www.googleapis.com/auth/sdm.service
```


| Placeholder       | Value                                  |
| ----------------- | -------------------------------------- |
| `PROJECT_ID`      | Device Access project UUID from step 4 |
| `OAUTH_CLIENT_ID` | OAuth Client ID from step 3            |


Then:

1. Open the URL in a browser while signed into the Google account that owns the Nest.
2. In Partner Connections Manager, grant access to the home / thermostat.
3. After consent you are redirected to `https://www.google.com/?code=AUTHORIZATION_CODE&scope=...`.
4. Copy the `code` query parameter (URL-decode it if needed). Codes are short-lived — exchange promptly.

Exchange the code for tokens:

```bash
curl -L -X POST 'https://www.googleapis.com/oauth2/v4/token?client_id=OAUTH_CLIENT_ID&client_secret=OAUTH_CLIENT_SECRET&code=AUTHORIZATION_CODE&grant_type=authorization_code&redirect_uri=https://www.google.com'
```

Expected response:

```json
{
  "access_token": "...",
  "expires_in": 3599,
  "refresh_token": "...",
  "scope": "https://www.googleapis.com/auth/sdm.service",
  "token_type": "Bearer"
}
```

Save the **refresh token** securely — it is what the backend will use long-term. The access token lasts about one hour.

Docs: [Authorize an Account](https://developers.google.com/nest/device-access/authorize)

### 6. Verify you can read temperature and humidity

List devices (completes authorization and confirms linkage):

```bash
curl -X GET \
  'https://smartdevicemanagement.googleapis.com/v1/enterprises/PROJECT_ID/devices' \
  -H 'Content-Type: application/json' \
  -H 'Authorization: Bearer ACCESS_TOKEN'
```

Find the thermostat (`type` = `sdm.devices.types.THERMOSTAT`) and note its `name`  
(`enterprises/PROJECT_ID/devices/DEVICE_ID`).

Fetch that device (or inspect the list response traits):

```bash
curl -X GET \
  'https://smartdevicemanagement.googleapis.com/v1/enterprises/PROJECT_ID/devices/DEVICE_ID' \
  -H 'Content-Type: application/json' \
  -H 'Authorization: Bearer ACCESS_TOKEN'
```

Relevant traits in the response:

```json
{
  "type": "sdm.devices.types.THERMOSTAT",
  "traits": {
    "sdm.devices.traits.Connectivity": { "status": "ONLINE" },
    "sdm.devices.traits.Info": { "customName": "Living Room Nest" },
    "sdm.devices.traits.Temperature": { "ambientTemperatureCelsius": 21.5 },
    "sdm.devices.traits.Humidity": { "ambientHumidityPercent": 42.0 }
  }
}
```

If those traits appear with sensible values, Google-side setup is done.

### 7. Refresh tokens when the access token expires

```bash
curl -L -X POST 'https://www.googleapis.com/oauth2/v4/token?client_id=OAUTH_CLIENT_ID&client_secret=OAUTH_CLIENT_SECRET&refresh_token=REFRESH_TOKEN&grant_type=refresh_token'
```

Returns a new `access_token` (typically without a new refresh token).

---



## Implementation outline (when coding)

Rough module layout (names indicative, not final):

```
backend/src/sensor_control/
  sensors.rs          # existing Hue
  nest.rs             # new Nest poller
  models.rs           # shared TemperatureData + Nest response structs
```

Suggested Nest poller responsibilities:

1. Load client id/secret, refresh token, and Device Access project id from `secrets/`.
2. Obtain (and cache) an access token; refresh when expired or on `401`.
3. `GET /v1/enterprises/{project_id}/devices`.
4. Keep devices where `type == "sdm.devices.types.THERMOSTAT"`.
5. Build `TemperatureData` per the mapping table above.
6. Reuse existing `store_temperatures` / `Storage::save_items` dedup path (or extract a shared helper).

Out of scope for v1:

- Thermostat mode / setpoint control
- Pub/Sub event streaming
- Nest cameras / doorbells
- Changing the MongoDB schema

---



## Checklist before implementation

- [x] Device Access registration paid
- [x] Nest thermostat on consumer Gmail / Google Home
- [x] GCP project with Smart Device Management API enabled
- [x] OAuth Web client with redirect `https://www.google.com`
- [x] Device Access project linked to that OAuth Client ID
- [x] Partner Connections Manager consent completed
- [x] Refresh token saved under `secrets/`
- [x] Manual `curl` list/get returns Temperature + Humidity traits
- [x] Chosen Nest `customName` does not collide with an existing Hue `device_name`

---



## References

- [Device Access — Get Started](https://developers.google.com/nest/device-access/get-started)
- [Device Access — Authorize](https://developers.google.com/nest/device-access/authorize)
- [Device Access — Use the API](https://developers.google.com/nest/device-access/use-the-api)
- [Nest Thermostat device type & traits](https://developers.google.com/nest/device-access/api/thermostat)
- [Google Cloud — Smart Device Management API](https://console.developers.google.com/apis/api/smartdevicemanagement.googleapis.com/overview)
- [Device Access Console](https://console.nest.google.com/device-access)

