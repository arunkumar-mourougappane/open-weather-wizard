//! # Google Weather API Provider
//!
//! Real implementation of `WeatherProvider` against Google Maps Platform's
//! Weather API (`https://weather.googleapis.com`). See
//! `docs/GOOGLE_WEATHER_API.md` for the full endpoint/response research this
//! module is based on.
//!
//! Two things this API doesn't provide that `openweather_api.rs`'s single
//! endpoint does:
//!
//! - **Geocoding.** Google's endpoints take `location.latitude`/
//!   `location.longitude` only -- there's no city-name lookup. Resolved here
//!   via the free, keyless Open-Meteo Geocoding API rather than Google's own
//!   (billable, separately-enabled) Geocoding API, so this provider stays
//!   independent of any other provider's key and doesn't require enabling a
//!   second Google Cloud API.
//! - **Sunrise/sunset in `currentConditions`.** Those live in the daily
//!   forecast's `sunEvents` instead, so `get_weather` makes a supplementary
//!   `forecast/days:lookup?days=1` call purely to read today's sun events
//!   and min/max temperatures.
//!
//! Timestamps come back as RFC 3339 UTC strings plus an IANA zone id (e.g.
//! `"America/Los_Angeles"`), but the shared `ApiResponse` shape expects a
//! Unix timestamp and a UTC-offset-in-seconds (see `Sys`/`ApiResponse::
//! timezone`). The `jiff` crate resolves the correct DST-aware offset for
//! that zone id at that instant.

use crate::config::LocationConfig;
use crate::weather_api::forecast::{ForecastDay, ForecastResponse};
use crate::weather_api::openweather_api::{
    ApiError, ApiResponse, Main, Sys, Weather, Wind, get_weather_symbol,
};
use crate::weather_api::weather_provider::WeatherProvider;
use async_trait::async_trait;
use serde::Deserialize;

const WEATHER_API_BASE: &str = "https://weather.googleapis.com/v1";
const GEOCODING_API_BASE: &str = "https://geocoding-api.open-meteo.com/v1/search";
/// Matches `forecast::MAX_FORECAST_DAYS` -- no point requesting more days
/// from Google than the UI will ever show.
const FORECAST_DAYS: u8 = 5;

// --- Open-Meteo geocoding -----------------------------------------------

#[derive(Deserialize, Debug)]
struct GeocodeResult {
    latitude: f64,
    longitude: f64,
    /// Full admin-1 (state/province) name, e.g. "Illinois" -- present when
    /// the location has one. Used to disambiguate same-named cities in
    /// different states/provinces (e.g. Peoria, IL vs. Peoria, AZ), since
    /// Open-Meteo's `name` search has no state/province filter parameter.
    #[serde(default)]
    admin1: Option<String>,
}

/// Open-Meteo omits the `results` key entirely (rather than returning `[]`)
/// when nothing matches, hence `#[serde(default)]`.
#[derive(Deserialize, Debug, Default)]
struct GeocodeResponse {
    #[serde(default)]
    results: Vec<GeocodeResult>,
}

/// U.S. postal abbreviation -> full state name, used only to translate a
/// `LocationConfig.state` like `"IL"` into the `"Illinois"` Open-Meteo
/// returns as `admin1` -- `LocationConfig.state` is otherwise passed through
/// as-is (e.g. for non-US provinces already given in full, like "Ontario").
const US_STATE_ABBREVIATIONS: &[(&str, &str)] = &[
    ("AL", "Alabama"),
    ("AK", "Alaska"),
    ("AZ", "Arizona"),
    ("AR", "Arkansas"),
    ("CA", "California"),
    ("CO", "Colorado"),
    ("CT", "Connecticut"),
    ("DE", "Delaware"),
    ("FL", "Florida"),
    ("GA", "Georgia"),
    ("HI", "Hawaii"),
    ("ID", "Idaho"),
    ("IL", "Illinois"),
    ("IN", "Indiana"),
    ("IA", "Iowa"),
    ("KS", "Kansas"),
    ("KY", "Kentucky"),
    ("LA", "Louisiana"),
    ("ME", "Maine"),
    ("MD", "Maryland"),
    ("MA", "Massachusetts"),
    ("MI", "Michigan"),
    ("MN", "Minnesota"),
    ("MS", "Mississippi"),
    ("MO", "Missouri"),
    ("MT", "Montana"),
    ("NE", "Nebraska"),
    ("NV", "Nevada"),
    ("NH", "New Hampshire"),
    ("NJ", "New Jersey"),
    ("NM", "New Mexico"),
    ("NY", "New York"),
    ("NC", "North Carolina"),
    ("ND", "North Dakota"),
    ("OH", "Ohio"),
    ("OK", "Oklahoma"),
    ("OR", "Oregon"),
    ("PA", "Pennsylvania"),
    ("RI", "Rhode Island"),
    ("SC", "South Carolina"),
    ("SD", "South Dakota"),
    ("TN", "Tennessee"),
    ("TX", "Texas"),
    ("UT", "Utah"),
    ("VT", "Vermont"),
    ("VA", "Virginia"),
    ("WA", "Washington"),
    ("WV", "West Virginia"),
    ("WI", "Wisconsin"),
    ("WY", "Wyoming"),
    ("DC", "District of Columbia"),
];

/// Resolves a `LocationConfig` to coordinates via the free, keyless
/// Open-Meteo Geocoding API. Requests several candidates and, when a state/
/// province was given, prefers the one whose `admin1` matches it -- plain
/// `name`-only search can't tell "Peoria, IL" from "Peoria, AZ" apart, and
/// picking the wrong one silently returns a real, plausible-looking, but
/// entirely wrong forecast.
async fn geocode(location: &LocationConfig) -> Result<(f64, f64), ApiError> {
    let client = reqwest::Client::new();
    let mut query = vec![
        ("name", location.city.clone()),
        ("count", "10".to_string()),
        ("language", "en".to_string()),
        ("format", "json".to_string()),
    ];
    if !location.country.is_empty() {
        query.push(("countryCode", location.country.clone()));
    }

    let response = client
        .get(GEOCODING_API_BASE)
        .query(&query)
        .send()
        .await
        .map_err(ApiError::RequestFailed)?;

    let parsed = response
        .json::<GeocodeResponse>()
        .await
        .map_err(|_| ApiError::InvalidResponse)?;

    let target_state = location.state.trim();
    let target_state = US_STATE_ABBREVIATIONS
        .iter()
        .find(|(abbr, _)| abbr.eq_ignore_ascii_case(target_state))
        .map(|(_, full)| *full)
        .unwrap_or(target_state);

    let results = parsed.results;
    if !target_state.is_empty()
        && let Some(matched) = results.iter().find(|r| {
            r.admin1
                .as_deref()
                .is_some_and(|admin1| admin1.eq_ignore_ascii_case(target_state))
        })
    {
        return Ok((matched.latitude, matched.longitude));
    }

    results
        .into_iter()
        .next()
        .map(|r| (r.latitude, r.longitude))
        .ok_or(ApiError::CityNotFound)
}

// --- Google Weather API response types ----------------------------------

#[derive(Deserialize, Debug)]
struct GTimeZone {
    id: String,
}

#[derive(Deserialize, Debug)]
struct GDescription {
    text: String,
}

#[derive(Deserialize, Debug)]
struct GWeatherCondition {
    description: GDescription,
    #[serde(rename = "type")]
    condition_type: String,
}

/// Shared shape for `temperature`/`feelsLikeTemperature`/`maxTemperature`/etc.
#[derive(Deserialize, Debug)]
struct GDegrees {
    degrees: f64,
}

#[derive(Deserialize, Debug)]
struct GWindDirection {
    degrees: i64,
}

#[derive(Deserialize, Debug)]
struct GWindSpeed {
    value: f64,
}

#[derive(Deserialize, Debug)]
struct GWind {
    direction: GWindDirection,
    speed: GWindSpeed,
}

#[derive(Deserialize, Debug)]
struct GVisibility {
    distance: f64,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
struct GAirPressure {
    mean_sea_level_millibars: f64,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
struct CurrentConditionsResponse {
    weather_condition: GWeatherCondition,
    temperature: GDegrees,
    feels_like_temperature: GDegrees,
    relative_humidity: i64,
    wind: GWind,
    visibility: GVisibility,
    air_pressure: GAirPressure,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
struct SunEvents {
    sunrise_time: String,
    sunset_time: String,
}

#[derive(Deserialize, Debug)]
struct DisplayDate {
    year: i32,
    month: u32,
    day: u32,
}

#[derive(Deserialize, Debug)]
struct PrecipProbability {
    percent: i64,
}

#[derive(Deserialize, Debug)]
struct GPrecipitation {
    probability: PrecipProbability,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
struct DayNightForecast {
    weather_condition: GWeatherCondition,
    relative_humidity: i64,
    wind: GWind,
    precipitation: GPrecipitation,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
struct ForecastDayItem {
    display_date: DisplayDate,
    max_temperature: GDegrees,
    min_temperature: GDegrees,
    feels_like_max_temperature: GDegrees,
    sun_events: SunEvents,
    daytime_forecast: DayNightForecast,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
struct ForecastDaysResponse {
    forecast_days: Vec<ForecastDayItem>,
    time_zone: GTimeZone,
}

// --- Unit conversions -----------------------------------------------------
// Google's METRIC unit system (requested explicitly below) returns wind
// speed in km/h and visibility in km; the shared `Wind`/visibility fields
// elsewhere in this codebase are documented as m/s and meters respectively
// (see `openweather_api::Wind`/`ApiResponse::visibility`).

fn kmh_to_mps(kmh: f64) -> f64 {
    kmh / 3.6
}

fn km_to_meters(km: f64) -> f64 {
    km * 1000.0
}

/// Maps Google's `WeatherCondition.Type` enum values onto the OpenWeatherMap
/// "main" condition strings `get_weather_symbol` already knows how to turn
/// into an icon -- reuses that lookup rather than adding a parallel symbol
/// table. Unrecognized/unspecified types fall through to `""`, which
/// `get_weather_symbol` maps to `WeatherSymbol::Default`.
fn google_condition_to_owm_main(condition_type: &str) -> &'static str {
    match condition_type {
        "CLEAR" | "MOSTLY_CLEAR" => "Clear",
        "PARTLY_CLOUDY" | "MOSTLY_CLOUDY" | "CLOUDY" | "WINDY" => "Clouds",
        "THUNDERSTORM"
        | "THUNDERSHOWER"
        | "LIGHT_THUNDERSTORM_RAIN"
        | "SCATTERED_THUNDERSTORMS"
        | "HEAVY_THUNDERSTORM" => "Thunderstorm",
        "LIGHT_SNOW_SHOWERS"
        | "CHANCE_OF_SNOW_SHOWERS"
        | "SCATTERED_SNOW_SHOWERS"
        | "SNOW_SHOWERS"
        | "HEAVY_SNOW_SHOWERS"
        | "LIGHT_TO_MODERATE_SNOW"
        | "MODERATE_TO_HEAVY_SNOW"
        | "SNOW"
        | "LIGHT_SNOW"
        | "HEAVY_SNOW"
        | "SNOWSTORM"
        | "SNOW_PERIODICALLY_HEAVY"
        | "HEAVY_SNOW_STORM"
        | "BLOWING_SNOW"
        | "RAIN_AND_SNOW"
        | "HAIL"
        | "HAIL_SHOWERS" => "Snow",
        "WIND_AND_RAIN"
        | "LIGHT_RAIN_SHOWERS"
        | "CHANCE_OF_SHOWERS"
        | "SCATTERED_SHOWERS"
        | "RAIN_SHOWERS"
        | "HEAVY_RAIN_SHOWERS"
        | "LIGHT_TO_MODERATE_RAIN"
        | "MODERATE_TO_HEAVY_RAIN"
        | "RAIN"
        | "LIGHT_RAIN"
        | "HEAVY_RAIN"
        | "RAIN_PERIODICALLY_HEAVY" => "Rain",
        _ => "",
    }
}

/// Parses an RFC 3339 UTC timestamp (as Google returns for `sunriseTime`/
/// `sunsetTime`) into a Unix epoch and the UTC offset (seconds) that
/// `iana_zone_id` observes at that instant -- the offset varies with DST, so
/// this can't be a fixed lookup table. Falls back to `(0, 0)` on any parse
/// failure rather than failing the whole fetch over a display-only field.
fn resolve_epoch_and_offset(rfc3339: &str, iana_zone_id: &str) -> (i64, i64) {
    let Ok(timestamp) = rfc3339.parse::<jiff::Timestamp>() else {
        log::warn!("Failed to parse Google Weather timestamp: {rfc3339}");
        return (0, 0);
    };
    let epoch = timestamp.as_second();
    let offset = match timestamp.in_tz(iana_zone_id) {
        Ok(zoned) => zoned.offset().seconds() as i64,
        Err(e) => {
            log::warn!("Failed to resolve timezone {iana_zone_id}: {e}");
            0
        }
    };
    (epoch, offset)
}

async fn fetch_current_conditions(
    api_key: &str,
    lat: f64,
    lon: f64,
) -> Result<CurrentConditionsResponse, ApiError> {
    let client = reqwest::Client::new();
    let response = client
        .get(format!("{WEATHER_API_BASE}/currentConditions:lookup"))
        .query(&[
            ("key", api_key.to_string()),
            ("location.latitude", lat.to_string()),
            ("location.longitude", lon.to_string()),
            ("unitsSystem", "METRIC".to_string()),
        ])
        .send()
        .await
        .map_err(ApiError::RequestFailed)?;

    if !response.status().is_success() {
        log::error!(
            "Google currentConditions request failed: {}",
            response.status()
        );
        return Err(ApiError::CityNotFound);
    }

    response
        .json::<CurrentConditionsResponse>()
        .await
        .map_err(|e| {
            log::error!("Failed to parse Google currentConditions response: {e}");
            ApiError::InvalidResponse
        })
}

async fn fetch_forecast_days(
    api_key: &str,
    lat: f64,
    lon: f64,
    days: u8,
) -> Result<ForecastDaysResponse, ApiError> {
    let client = reqwest::Client::new();
    let response = client
        .get(format!("{WEATHER_API_BASE}/forecast/days:lookup"))
        .query(&[
            ("key", api_key.to_string()),
            ("location.latitude", lat.to_string()),
            ("location.longitude", lon.to_string()),
            ("unitsSystem", "METRIC".to_string()),
            ("days", days.to_string()),
        ])
        .send()
        .await
        .map_err(ApiError::RequestFailed)?;

    if !response.status().is_success() {
        log::error!("Google forecast/days request failed: {}", response.status());
        return Err(ApiError::CityNotFound);
    }

    response.json::<ForecastDaysResponse>().await.map_err(|e| {
        log::error!("Failed to parse Google forecast/days response: {e}");
        ApiError::InvalidResponse
    })
}

fn map_forecast_day(item: &ForecastDayItem) -> ForecastDay {
    let day = &item.daytime_forecast;
    ForecastDay {
        date: format!(
            "{:04}-{:02}-{:02}",
            item.display_date.year, item.display_date.month, item.display_date.day
        ),
        temp_min: item.min_temperature.degrees,
        temp_max: item.max_temperature.degrees,
        description: day.weather_condition.description.text.clone(),
        symbol: get_weather_symbol(google_condition_to_owm_main(
            &day.weather_condition.condition_type,
        )),
        feels_like: item.feels_like_max_temperature.degrees,
        humidity: day.relative_humidity,
        wind_speed: kmh_to_mps(day.wind.speed.value),
        wind_deg: day.wind.direction.degrees,
        // Google's per-day forecast doesn't return barometric pressure or
        // visibility -- both fields are `#[allow(dead_code)]` on
        // `ForecastDay` today (nothing reads them yet), so this isn't a
        // display regression.
        pressure: 0,
        visibility: 0,
        pop: day.precipitation.probability.percent as f64 / 100.0,
    }
}

/// A real implementation of the `WeatherProvider` trait for Google Maps
/// Platform's Weather API.
pub struct GoogleWeatherProvider {
    api_key: String,
}

impl GoogleWeatherProvider {
    /// Creates a new `GoogleWeatherProvider` with the given Google Cloud API
    /// key (must have the Weather API enabled on its project).
    pub fn new(api_key: String) -> Self {
        Self { api_key }
    }
}

#[async_trait]
impl WeatherProvider for GoogleWeatherProvider {
    async fn get_weather(&self, location: &LocationConfig) -> Result<ApiResponse, ApiError> {
        let (lat, lon) = geocode(location).await?;

        let current = fetch_current_conditions(&self.api_key, lat, lon).await?;
        // Sunrise/sunset and today's min/max only come from the daily
        // forecast, not currentConditions -- see the module doc.
        let forecast = fetch_forecast_days(&self.api_key, lat, lon, 1).await?;
        let today = forecast
            .forecast_days
            .first()
            .ok_or(ApiError::InvalidResponse)?;

        let (sunrise, sunrise_offset) =
            resolve_epoch_and_offset(&today.sun_events.sunrise_time, &forecast.time_zone.id);
        let (sunset, _) =
            resolve_epoch_and_offset(&today.sun_events.sunset_time, &forecast.time_zone.id);

        Ok(ApiResponse {
            weather: vec![Weather {
                main: google_condition_to_owm_main(&current.weather_condition.condition_type)
                    .to_string(),
                description: current.weather_condition.description.text.clone(),
            }],
            main: Main {
                temp: current.temperature.degrees,
                feels_like: current.feels_like_temperature.degrees,
                temp_min: today.min_temperature.degrees,
                temp_max: today.max_temperature.degrees,
                pressure: current.air_pressure.mean_sea_level_millibars.round() as i64,
                humidity: current.relative_humidity,
            },
            wind: Wind {
                speed: kmh_to_mps(current.wind.speed.value),
                deg: current.wind.direction.degrees,
            },
            visibility: km_to_meters(current.visibility.distance) as i64,
            sys: Sys { sunrise, sunset },
            timezone: sunrise_offset,
            name: location.city.clone(),
        })
    }

    async fn get_forecast(&self, location: &LocationConfig) -> Result<ForecastResponse, ApiError> {
        let (lat, lon) = geocode(location).await?;
        let forecast = fetch_forecast_days(&self.api_key, lat, lon, FORECAST_DAYS).await?;

        Ok(ForecastResponse {
            location_name: location.city.clone(),
            days: forecast
                .forecast_days
                .iter()
                .map(map_forecast_day)
                .collect(),
        })
    }

    fn name(&self) -> &'static str {
        "Google Weather"
    }

    fn requires_api_key(&self) -> bool {
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_condition_mapping() {
        assert_eq!(google_condition_to_owm_main("CLEAR"), "Clear");
        assert_eq!(google_condition_to_owm_main("MOSTLY_CLEAR"), "Clear");
        assert_eq!(google_condition_to_owm_main("CLOUDY"), "Clouds");
        assert_eq!(google_condition_to_owm_main("HEAVY_RAIN"), "Rain");
        assert_eq!(google_condition_to_owm_main("SNOWSTORM"), "Snow");
        assert_eq!(
            google_condition_to_owm_main("HEAVY_THUNDERSTORM"),
            "Thunderstorm"
        );
        assert_eq!(google_condition_to_owm_main("SOMETHING_UNKNOWN"), "");
    }

    #[test]
    fn test_unit_conversions() {
        assert!((kmh_to_mps(3.6) - 1.0).abs() < 1e-9);
        assert!((km_to_meters(10.0) - 10_000.0).abs() < 1e-9);
    }

    #[test]
    fn test_geocode_response_with_results() {
        let json = r#"{"results":[{"latitude":37.422,"longitude":-122.0841}]}"#;
        let parsed: GeocodeResponse = serde_json::from_str(json).unwrap();
        assert_eq!(parsed.results.len(), 1);
        assert_eq!(parsed.results[0].latitude, 37.422);
    }

    #[test]
    fn test_geocode_response_missing_results_key() {
        // Open-Meteo omits `results` entirely (rather than `[]`) when
        // nothing matches -- this must not fail to deserialize.
        let json = r#"{"generationtime_ms":0.5}"#;
        let parsed: GeocodeResponse = serde_json::from_str(json).unwrap();
        assert!(parsed.results.is_empty());
    }

    /// Regression test for a real bug caught by the live smoke test: with
    /// only `count=1` and no state disambiguation, "Peoria" resolved to
    /// Peoria, AZ instead of Peoria, IL. Verifies the `admin1`-matching
    /// selects the right same-named city out of several candidates.
    #[test]
    fn test_geocode_disambiguates_same_named_city_by_state() {
        let json = r#"{"results":[
            {"latitude":33.5806,"longitude":-112.2374,"admin1":"Arizona"},
            {"latitude":40.6936,"longitude":-89.5890,"admin1":"Illinois"}
        ]}"#;
        let parsed: GeocodeResponse = serde_json::from_str(json).unwrap();
        let location = LocationConfig {
            city: "Peoria".to_string(),
            state: "IL".to_string(),
            country: "US".to_string(),
        };
        let target_state = US_STATE_ABBREVIATIONS
            .iter()
            .find(|(abbr, _)| abbr.eq_ignore_ascii_case(&location.state))
            .map(|(_, full)| *full)
            .unwrap();
        assert_eq!(target_state, "Illinois");
        let matched = parsed
            .results
            .iter()
            .find(|r| r.admin1.as_deref() == Some(target_state))
            .unwrap();
        assert_eq!((matched.latitude, matched.longitude), (40.6936, -89.5890));
    }

    #[test]
    fn test_current_conditions_deserialize_and_map() {
        let json = r#"{
            "weatherCondition": {
                "iconBaseUri": "https://maps.gstatic.com/weather/v1/sunny",
                "description": { "text": "Sunny", "languageCode": "en" },
                "type": "CLEAR"
            },
            "temperature": { "degrees": 22.5, "unit": "CELSIUS" },
            "feelsLikeTemperature": { "degrees": 22.0, "unit": "CELSIUS" },
            "relativeHumidity": 65,
            "wind": {
                "direction": { "degrees": 210, "cardinal": "SSW" },
                "speed": { "value": 12.96, "unit": "KILOMETERS_PER_HOUR" },
                "gust": { "value": 20.0, "unit": "KILOMETERS_PER_HOUR" }
            },
            "visibility": { "distance": 10.0, "unit": "KILOMETERS" },
            "airPressure": { "meanSeaLevelMillibars": 1015.0 },
            "cloudCover": 0
        }"#;
        let parsed: CurrentConditionsResponse = serde_json::from_str(json).unwrap();
        assert_eq!(parsed.weather_condition.condition_type, "CLEAR");
        assert_eq!(parsed.temperature.degrees, 22.5);
        assert_eq!(parsed.relative_humidity, 65);
        assert!((kmh_to_mps(parsed.wind.speed.value) - 3.6).abs() < 1e-6);
        assert_eq!(parsed.air_pressure.mean_sea_level_millibars, 1015.0);
    }

    #[test]
    fn test_forecast_days_deserialize_and_map() {
        let json = r#"{
            "forecastDays": [
                {
                    "displayDate": { "year": 2026, "month": 7, "day": 4 },
                    "maxTemperature": { "degrees": 28.0, "unit": "CELSIUS" },
                    "minTemperature": { "degrees": 16.0, "unit": "CELSIUS" },
                    "feelsLikeMaxTemperature": { "degrees": 29.0, "unit": "CELSIUS" },
                    "sunEvents": {
                        "sunriseTime": "2026-07-04T11:00:00Z",
                        "sunsetTime": "2026-07-05T02:00:00Z"
                    },
                    "daytimeForecast": {
                        "weatherCondition": {
                            "iconBaseUri": "https://maps.gstatic.com/weather/v1/cloudy",
                            "description": { "text": "Cloudy", "languageCode": "en" },
                            "type": "CLOUDY"
                        },
                        "relativeHumidity": 40,
                        "wind": {
                            "direction": { "degrees": 90, "cardinal": "E" },
                            "speed": { "value": 7.2, "unit": "KILOMETERS_PER_HOUR" },
                            "gust": { "value": 10.0, "unit": "KILOMETERS_PER_HOUR" }
                        },
                        "precipitation": {
                            "probability": { "percent": 30, "type": "RAIN" }
                        }
                    }
                }
            ],
            "timeZone": { "id": "America/Chicago" }
        }"#;
        let parsed: ForecastDaysResponse = serde_json::from_str(json).unwrap();
        assert_eq!(parsed.time_zone.id, "America/Chicago");
        let day = map_forecast_day(&parsed.forecast_days[0]);
        assert_eq!(day.date, "2026-07-04");
        assert_eq!(day.temp_max, 28.0);
        assert_eq!(day.temp_min, 16.0);
        assert_eq!(day.description, "Cloudy");
        assert!((day.wind_speed - 2.0).abs() < 1e-6);
        assert!((day.pop - 0.3).abs() < 1e-9);
    }

    #[test]
    fn test_resolve_epoch_and_offset_valid() {
        let (epoch, offset) = resolve_epoch_and_offset("2026-07-04T11:00:00Z", "America/Chicago");
        assert!(epoch > 0);
        // America/Chicago is UTC-5 during summer (CDT).
        assert_eq!(offset, -5 * 3600);
    }

    #[test]
    fn test_resolve_epoch_and_offset_invalid_falls_back() {
        let (epoch, offset) = resolve_epoch_and_offset("not-a-timestamp", "America/Chicago");
        assert_eq!(epoch, 0);
        assert_eq!(offset, 0);
    }
}
