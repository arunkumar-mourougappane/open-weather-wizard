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
use serde::Deserialize;

/// Represents weather conditions returned by the OpenWeatherMap API.
///
/// This struct contains the main weather type (e.g., "Clouds", "Rain") and a more detailed description
#[derive(Deserialize, Debug)]
pub struct Weather {
    pub main: String,
    pub description: String,
}

/// Contains the main meteorological data like temperature and humidity.
///
/// This struct is part of the `ApiResponse` and holds the primary weather metrics
#[derive(Deserialize, Debug)]
pub struct Main {
    pub temp: f64,
    pub humidity: i64,
}

/// Represents the top-level structure of the JSON response from the OpenWeatherMap API.
///
/// This struct aggregates the most relevant weather information, including a list of weather
/// conditions, the main meteorological data like temperature and humidity, and the name of the city.
#[derive(Deserialize, Debug)]
#[allow(dead_code)]
pub struct ApiResponse {
    pub weather: Vec<Weather>,
    pub main: Main,
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
#[derive(Debug)]
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
    log::info!("Geocoding response: {:?}", locations);
    // The API returns an empty array `[]` if the location isn't found.
    // We take the first element if it exists, otherwise return a `LocationNotFound` error.
    locations
        .into_iter()
        .next()
        .ok_or(GeocodeError::LocationNotFound)
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
    let weather_location = get_coords(
        &location.name,
        location.state.as_deref().unwrap_or(""),
        &location.country.clone().unwrap_or("".to_string()),
        api_key,
    )
    .await
    .map_err(|e| match e {
        GeocodeError::RequestFailed(err) => {
            println!("Error fetching coordinates: {:?}", err);
            ApiError::RequestFailed(err)
        }
        GeocodeError::LocationNotFound => {
            println!("Location not found");
            ApiError::CityNotFound
        }
    })?;

    // Construct the API URL. We use metric units for Celsius.
    let url = format!(
        "https://api.openweathermap.org/data/2.5/weather?lat={}&lon={}&appid={}&units=metric",
        weather_location.lat, weather_location.lon, api_key
    );

    // Make the asynchronous GET request
    let response = reqwest::get(&url).await.map_err(ApiError::RequestFailed)?;
    log::info!("Weather API response: {}", response.status());
    // Check if the request was successful (e.g., status 200 OK)
    if response.status().is_success() {
        // Try to parse the JSON response into our ApiResponse struct
        response.json::<ApiResponse>().await.map_err(|_| {
            log::error!("Failed to parse API response");
            ApiError::InvalidResponse
        })
    } else {
        // If the city is not found, the API returns a 404 status
        log::error!("City not found: {}", location.name);
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
    log::info!(
        "Mapping weather condition '{}' to symbol",
        weather_condition
    );
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

    /// Returns the display name of the provider.
    fn name(&self) -> &'static str {
        "OpenWeather"
    }

    /// Returns `true` as this provider requires an API key.
    fn requires_api_key(&self) -> bool {
        true
    }
}
