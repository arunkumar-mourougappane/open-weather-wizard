//! # OpenWeatherMap API Provider
//!
//! # Overview
//!
//! This module provides the implementation for fetching and parsing weather data
//! from the OpenWeatherMap API.
//!
//! ## Features
//!
//! - **Geocoding**: Converts a city name into geographic coordinates (latitude/longitude).
//! - **Weather Fetching**: Retrieves current weather data for the given coordinates.
//! - **Data Structures**: Defines Rust structs (`ApiResponse`, `Weather`, `Main`) that
//!   map directly to the JSON responses from the API.
//! - **Error Handling**: Provides specific error types (`ApiError`, `GeocodeError`) for
//!   robust error management.
//! - **Provider Implementation**: Implements the `WeatherProvider` trait for seamless
//!   integration into the application's provider factory.
//!
use crate::config::LocationConfig;
use crate::weather_api::weather_provider::{WeatherProvider, location_config_to_location};
use async_trait::async_trait;
use reqwest;
use serde::{Deserialize, Serialize};

/// Represents weather conditions returned by the OpenWeatherMap API.
///
/// This struct contains the main weather type (e.g., "Clouds", "Rain") and a more detailed description
#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct Weather {
    pub main: String,
    pub description: String,
}

/// Contains the main meteorological data like temperature and humidity.
///
/// This struct is part of the `ApiResponse` and holds the primary weather metrics
#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct Main {
    pub temp: f64,
    pub feels_like: f64,
    pub temp_min: f64,
    pub temp_max: f64,
    pub pressure: i64,
    pub humidity: i64,
}

/// Wind speed (meters/sec, since requests use `units=metric`) and direction
/// (meteorological degrees, 0 = due north).
#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct Wind {
    pub speed: f64,
    pub deg: i64,
}

/// Sunrise/sunset as Unix (UTC) timestamps.
#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct Sys {
    pub sunrise: i64,
    pub sunset: i64,
}

/// Represents the top-level structure of the JSON response from the OpenWeatherMap API.
///
/// This struct aggregates the most relevant weather information, including a list of weather
/// conditions, the main meteorological data like temperature and humidity, and the name of the city.
///
/// Derives `Serialize` (in addition to `Deserialize`) so the headless CLI mode
/// (`src/cli.rs`, bin-only) can emit this directly as `--json` output --
/// nothing about parsing provider responses needs it, only that output path.
#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct ApiResponse {
    pub weather: Vec<Weather>,
    pub main: Main,
    pub wind: Wind,
    /// Meters. OpenWeatherMap caps this at 10000 ("10km+").
    pub visibility: i64,
    pub sys: Sys,
    /// Seconds offset from UTC for the queried location, used to render
    /// sunrise/sunset in local time rather than UTC.
    pub timezone: i64,
    pub name: String,
}

/// Represents possible errors that can occur when interacting with the OpenWeatherMap API.
///
/// This enum provides detailed error variants to distinguish between different failure modes:
/// - `RequestFailed`: Indicates a network or HTTP error occurred during the API request.
/// - `CityNotFound`: Returned when the requested city does not exist or cannot be found by the API.
/// - `InvalidResponse`: Indicates that the response from the API could not be parsed or was malformed.
#[derive(Debug)]
#[allow(dead_code)]
pub enum ApiError {
    RequestFailed(reqwest::Error),
    CityNotFound,
    InvalidResponse,
}

/// Represents a symbolic representation of a weather condition.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum WeatherSymbol {
    Clear,
    Clouds,
    Rain,
    Drizzle,
    Thunderstorm,
    Snow,
    Mist,
    Smoke,
    Haze,
    Dust,
    Fog,
    Sand,
    Ash,
    Squall,
    Tornado,
    Default,
}

/// Represents a geographic location returned by the Geocoding API.
///
/// This struct matches the structure of the JSON objects in the array
// returned by the OpenWeatherMap Geocoding API.
#[derive(Deserialize, Debug)]
pub struct Location {
    pub name: String,
    pub lat: f64,
    pub lon: f64,
    pub country: Option<String>,
    // Use `Option` for state, as it may not always be present in the response.
    pub state: Option<String>,
}

/// Represents possible errors that can occur when geocoding a location.
#[derive(Debug)]
pub enum GeocodeError {
    RequestFailed(reqwest::Error),
    LocationNotFound,
}

/// Fetches geographic coordinates for a given location using the OpenWeatherMap Geocoding API.
///
/// This function constructs a query from the city, state, and country, then calls the
/// `geo/1.0/direct` endpoint. It requests a single result (`limit=1`) to get the most
/// relevant location.
///
/// # Arguments
/// * `city` - The name of the city.
/// * `state` - The state or region (can be empty).
/// * `country` - The country code (e.g., "US", "CA").
/// * `api_key` - Your OpenWeatherMap API key.
async fn get_coords(
    city: &str,
    state: &str,
    country: &str,
    api_key: &str,
) -> Result<Location, GeocodeError> {
    // Build the query string, joining non-empty parts with commas.
    let location_query = [city, state, country]
        .iter()
        .filter(|s| !s.is_empty())
        .map(|s| s.trim())
        .collect::<Vec<_>>()
        .join(",");

    // Construct the full API URL. `limit=1` ensures we get only the most relevant result.
    let url = format!(
        "http://api.openweathermap.org/geo/1.0/direct?q={}&limit=1&appid={}",
        location_query, api_key
    );
    // Make the request and parse the JSON response into a Vec of Locations.
    // The API returns an array, even if it's empty or has one item.
    let locations = reqwest::get(&url)
        .await
        .map_err(GeocodeError::RequestFailed)?
        .json::<Vec<Location>>()
        .await
        .map_err(GeocodeError::RequestFailed)?;
    log::debug!("Geocoding response: {:?}", locations);
    select_location(locations)
}

/// The API returns an empty array `[]` if the location isn't found, and an
/// array of matches (most relevant first, though `get_coords` already
/// requests `limit=1`) otherwise -- take the first element if present, or
/// `LocationNotFound` for an empty response. Split out from `get_coords` so
/// this selection logic is testable without a live network call.
fn select_location(locations: Vec<Location>) -> Result<Location, GeocodeError> {
    locations
        .into_iter()
        .next()
        .ok_or(GeocodeError::LocationNotFound)
}

/// Resolves a `Location`'s coordinates via `get_coords`, mapping `GeocodeError`
/// into the `ApiError` variants shared by both current-weather and forecast fetches.
async fn resolve_location(location: &Location, api_key: &str) -> Result<Location, ApiError> {
    get_coords(
        &location.name,
        location.state.as_deref().unwrap_or(""),
        &location.country.clone().unwrap_or("".to_string()),
        api_key,
    )
    .await
    .map_err(|e| match e {
        GeocodeError::RequestFailed(err) => {
            log::error!("Error fetching coordinates: {:?}", err);
            ApiError::RequestFailed(err)
        }
        GeocodeError::LocationNotFound => {
            log::warn!("Location not found");
            ApiError::CityNotFound
        }
    })
}

/// Fetches weather data for a given location using the OpenWeatherMap API.
///
/// This is a two-step process:
/// 1. It first calls `get_coords` to convert the location name into latitude and longitude.
/// 2. It then uses these coordinates to fetch the current weather data.
///
/// # Arguments
/// * `location` - The location to fetch weather for.
/// * `api_key` - Your personal OpenWeatherMap API key.
pub async fn get_weather(location: &Location, api_key: &str) -> Result<ApiResponse, ApiError> {
    // Get coordinates for the location
    let weather_location = resolve_location(location, api_key).await?;

    // Construct the API URL. We use metric units for Celsius.
    let url = format!(
        "https://api.openweathermap.org/data/2.5/weather?lat={}&lon={}&appid={}&units=metric",
        weather_location.lat, weather_location.lon, api_key
    );

    // Make the asynchronous GET request
    let response = reqwest::get(&url).await.map_err(ApiError::RequestFailed)?;
    log::debug!("Weather API response: {}", response.status());
    // Check if the request was successful (e.g., status 200 OK)
    if response.status().is_success() {
        // Try to parse the JSON response into our ApiResponse struct
        response.json::<ApiResponse>().await.map_err(|e| {
            log::error!("Failed to parse API response: {e}");
            ApiError::InvalidResponse
        })
    } else {
        // If the city is not found, the API returns a 404 status
        log::error!("City not found: {}", location.name);
        Err(ApiError::CityNotFound)
    }
}

/// Fetches a 5-day/3-hour forecast for a given location using the OpenWeatherMap
/// API, aggregated into daily summaries by `forecast::aggregate_daily`.
///
/// # Arguments
/// * `location` - The location to fetch a forecast for.
/// * `api_key` - Your personal OpenWeatherMap API key.
pub async fn get_forecast(
    location: &Location,
    api_key: &str,
) -> Result<crate::weather_api::forecast::ForecastResponse, ApiError> {
    let weather_location = resolve_location(location, api_key).await?;

    let url = format!(
        "https://api.openweathermap.org/data/2.5/forecast?lat={}&lon={}&appid={}&units=metric",
        weather_location.lat, weather_location.lon, api_key
    );

    let response = reqwest::get(&url).await.map_err(ApiError::RequestFailed)?;
    log::debug!("Forecast API response: {}", response.status());
    if response.status().is_success() {
        let raw = response
            .json::<crate::weather_api::forecast::RawForecastResponse>()
            .await
            .map_err(|e| {
                log::error!("Failed to parse forecast API response: {e}");
                ApiError::InvalidResponse
            })?;
        Ok(crate::weather_api::forecast::aggregate_daily(raw))
    } else {
        log::error!("Forecast not found for: {}", location.name);
        Err(ApiError::CityNotFound)
    }
}

/// Maps a weather condition string from the API to a `WeatherSymbol` enum.
///
/// This allows the application to associate a specific icon or behavior with a
/// generalized weather condition.
///
/// # Arguments
/// * `weather_condition` - The "main" weather string from the API (e.g., "Clear", "Rain").
pub fn get_weather_symbol(weather_condition: &str) -> WeatherSymbol {
    match weather_condition {
        "Clear" => WeatherSymbol::Clear,
        "Clouds" => WeatherSymbol::Clouds,
        "Rain" => WeatherSymbol::Rain,
        "Drizzle" => WeatherSymbol::Drizzle,
        "Thunderstorm" => WeatherSymbol::Thunderstorm,
        "Snow" => WeatherSymbol::Snow,
        "Mist" => WeatherSymbol::Mist,
        "Smoke" => WeatherSymbol::Smoke,
        "Haze" => WeatherSymbol::Haze,
        "Dust" => WeatherSymbol::Dust,
        "Fog" => WeatherSymbol::Fog,
        "Sand" => WeatherSymbol::Sand,
        "Ash" => WeatherSymbol::Ash,
        "Squall" => WeatherSymbol::Squall,
        "Tornado" => WeatherSymbol::Tornado,
        _ => WeatherSymbol::Default,
    }
}

/// An implementation of the `WeatherProvider` trait for the OpenWeatherMap service.
pub struct OpenWeatherProvider {
    api_key: String,
}

impl OpenWeatherProvider {
    /// Creates a new `OpenWeatherProvider` with the given API key.
    ///
    /// # Arguments
    /// * `api_key` - The API key for the OpenWeatherMap service.
    pub fn new(api_key: String) -> Self {
        Self { api_key }
    }
}

#[async_trait]
impl WeatherProvider for OpenWeatherProvider {
    /// Fetches weather data by implementing the `WeatherProvider` trait.
    ///
    /// This function converts the application's `LocationConfig` into a `Location`
    /// struct suitable for the API and then calls the internal `get_weather` function.
    async fn get_weather(&self, location: &LocationConfig) -> Result<ApiResponse, ApiError> {
        let api_location = location_config_to_location(location);
        get_weather(&api_location, &self.api_key).await
    }

    /// Fetches a forecast by implementing the `WeatherProvider` trait.
    async fn get_forecast(
        &self,
        location: &LocationConfig,
    ) -> Result<crate::weather_api::forecast::ForecastResponse, ApiError> {
        let api_location = location_config_to_location(location);
        get_forecast(&api_location, &self.api_key).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_weather_symbol_known_conditions() {
        assert_eq!(get_weather_symbol("Clear"), WeatherSymbol::Clear);
        assert_eq!(get_weather_symbol("Rain"), WeatherSymbol::Rain);
        assert_eq!(
            get_weather_symbol("Thunderstorm"),
            WeatherSymbol::Thunderstorm
        );
        assert_eq!(get_weather_symbol("Tornado"), WeatherSymbol::Tornado);
    }

    #[test]
    fn test_get_weather_symbol_unknown_falls_back_to_default() {
        assert_eq!(
            get_weather_symbol("SomethingUnknown"),
            WeatherSymbol::Default
        );
        assert_eq!(get_weather_symbol(""), WeatherSymbol::Default);
    }

    #[test]
    fn test_select_location_empty_is_not_found() {
        let result = select_location(vec![]);
        assert!(matches!(result, Err(GeocodeError::LocationNotFound)));
    }

    #[test]
    fn test_select_location_picks_first_result() {
        let locations = vec![
            Location {
                name: "Peoria".to_string(),
                lat: 40.6936,
                lon: -89.589,
                country: Some("US".to_string()),
                state: Some("IL".to_string()),
            },
            Location {
                name: "Peoria".to_string(),
                lat: 33.5806,
                lon: -112.2374,
                country: Some("US".to_string()),
                state: Some("AZ".to_string()),
            },
        ];
        let selected = select_location(locations).unwrap();
        assert_eq!(selected.state.as_deref(), Some("IL"));
    }

    #[test]
    fn test_location_deserializes_from_geocode_fixture() {
        // Matches the shape of OpenWeatherMap's geo/1.0/direct response.
        let json = r#"[{
            "name": "Peoria",
            "lat": 40.6936,
            "lon": -89.589,
            "country": "US",
            "state": "Illinois"
        }]"#;
        let locations: Vec<Location> = serde_json::from_str(json).unwrap();
        assert_eq!(locations.len(), 1);
        assert_eq!(locations[0].name, "Peoria");
        assert_eq!(locations[0].state.as_deref(), Some("Illinois"));
    }

    #[test]
    fn test_location_deserializes_empty_geocode_fixture() {
        // OpenWeatherMap returns an empty array (not an error) when nothing matches.
        let locations: Vec<Location> = serde_json::from_str("[]").unwrap();
        assert!(locations.is_empty());
    }

    #[test]
    fn test_api_response_deserializes_from_fixture() {
        // Matches the shape of OpenWeatherMap's data/2.5/weather response.
        let json = r#"{
            "weather": [{"main": "Clear", "description": "clear sky"}],
            "main": {
                "temp": 22.5,
                "feels_like": 22.0,
                "temp_min": 19.0,
                "temp_max": 25.0,
                "pressure": 1015,
                "humidity": 65
            },
            "wind": {"speed": 3.6, "deg": 210},
            "visibility": 10000,
            "sys": {"sunrise": 1700000000, "sunset": 1700040000},
            "timezone": -18000,
            "name": "Peoria"
        }"#;
        let response: ApiResponse = serde_json::from_str(json).unwrap();
        assert_eq!(response.name, "Peoria");
        assert_eq!(response.weather[0].main, "Clear");
        assert_eq!(response.main.temp, 22.5);
        assert_eq!(response.wind.deg, 210);
        assert_eq!(response.sys.sunrise, 1_700_000_000);
        assert_eq!(response.timezone, -18000);
    }
}
