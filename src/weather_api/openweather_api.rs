//! This module provides functionality to fetch and parse weather data from the OpenWeatherMap API.
//!
//! # Overview
//!
//! The module defines data structures to deserialize relevant fields from the OpenWeatherMap API JSON response,
//! as well as a custom error type for improved error handling. The main function, `get_weather`, asynchronously
//! retrieves weather information for a specified city using a provided API key.
//!
//! # Structs
//!
//! - `Weather`: Represents weather conditions, including the main type and description.
//! - `Main`: Contains temperature and humidity data.
//! - `ApiResponse`: Aggregates weather, main, and city name information from the API response.
//!
//! # Errors
//!
//! - `ApiError`: Enumerates possible errors, such as request failures, city not found, and invalid responses.
//!
//! # Functions
//!
//! - `get_weather`: Asynchronously fetches weather data for a given city and API key, returning either the parsed
//!   response or an error.
//!
//! # Example
//!
//! ```rust
//! use weather_api::openweather::get_weather;
//!
//! #[tokio::main]
//! async fn main() {
//!     let city = "London";
//!     let api_key = "your_api_key";
//!     match get_weather(city, api_key).await {
//!         Ok(response) => println!("{:?}", response),
//!         Err(e) => eprintln!("Error: {:?}", e),
//!     }
//! }
//! ```
use reqwest;
use serde::Deserialize;

const API_KEY: &str = "a836db2d273c0b50a2376d6a31750064"; // Replace with your actual OpenWeatherMap API key

/// Represents weather conditions returned by the OpenWeatherMap API.
///
/// This struct contains the main weather type (e.g., "Clouds", "Rain") and a more detailed description
/// (e.g., "scattered clouds", "light rain").
///
/// # Fields
/// - `main`: The primary weather condition.
/// - `description`: A textual description of the weather.
#[derive(Deserialize, Debug)]
pub struct Weather {
    pub main: String,
    pub description: String,
}

/// Contains the main meteorological data like temperature and humidity.
///
/// This struct is part of the `ApiResponse` and holds the primary weather metrics
/// returned by the OpenWeatherMap API.
///
/// # Fields
/// - `temp`: The current temperature in Celsius.
/// - `humidity`: The current humidity level in percent.
#[derive(Deserialize, Debug)]
pub struct Main {
    pub temp: f64,
    pub humidity: i64,
}

/// Represents the top-level structure of the JSON response from the OpenWeatherMap API.
///
/// This struct aggregates the most relevant weather information, including a list of weather
/// conditions, the main meteorological data like temperature and humidity, and the name of the city.
///
/// # Fields
/// - `weather`: A vector of `Weather` structs. Typically contains a single element describing the primary weather condition.
/// - `main`: A `Main` struct containing key meteorological data such as temperature and humidity.
/// - `name`: The name of the city corresponding to the weather data.
#[derive(Deserialize, Debug)]
#[allow(dead_code)]
pub struct ApiResponse {
    pub weather: Vec<Weather>,
    pub main: Main,
    pub name: String,
}

/// Represents possible errors that can occur when interacting with the OpenWeatherMap API.
///
/// This enum provides detailed error variants to help distinguish between different failure modes:
///
/// - `RequestFailed`: Indicates a network or HTTP error occurred during the API request.
/// - `CityNotFound`: Returned when the requested city does not exist or cannot be found by the API.
/// - `InvalidResponse`: Indicates that the response from the API could not be parsed or was malformed.
///
/// Use this type to handle errors gracefully and provide informative feedback to users.
#[derive(Debug)]
#[allow(dead_code)]
pub enum ApiError {
    RequestFailed(reqwest::Error),
    CityNotFound,
    InvalidResponse,
}

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

// This struct matches the structure of the JSON objects inside the array
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

// Custom error types for clearer error handling.
#[derive(Debug)]
pub enum GeocodeError {
    RequestFailed(reqwest::Error),
    LocationNotFound,
}

/// Fetches geographic coordinates for a given location.
///
/// # Arguments
/// * `city` - The name of the city.
/// * `state` - The state or region (can be empty).
/// * `country` - The country code (e.g., "US", "CA").
/// * `api_key` - Your OpenWeatherMap API key.
///
/// # Returns
/// A `Result` containing the first found `Location` or a `GeocodeError`.
async fn get_coords(city: &str, state: &str, country: &str) -> Result<Location, GeocodeError> {
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
        location_query, API_KEY
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

/// Asynchronously fetches weather data for a given city.
///
/// # Arguments
/// * `city` - The name of the city to fetch weather for.
/// * `api_key` - Your personal OpenWeatherMap API key.
///
/// # Returns
/// A `Result` containing the `ApiResponse` on success, or an `ApiError` on failure.
pub async fn get_weather(location: &Location) -> Result<ApiResponse, ApiError> {
    // Get coordinates for the location
    let weather_location = get_coords(
        &location.name,
        location.state.as_deref().unwrap_or(""),
        &location.country.clone().unwrap_or("".to_string()),
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
        weather_location.lat, weather_location.lon, API_KEY
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

/// Returns a `WeatherSymbol` enum variant based on the main weather condition string.
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
