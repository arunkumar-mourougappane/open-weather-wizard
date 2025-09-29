//! # Weather Provider Abstraction
//!
//! This module defines the core abstraction for interacting with different weather
//! data services. It decoules the main application logic from the specific
//! implementations of various weather APIs.
//!
//! ## Key Components
//!
//! - **`WeatherProvider` Trait**: A common interface that all weather providers must
//!   implement. It guarantees that any provider can fetch weather data in a
//!   standardized way.
//! - **`WeatherProviderFactory`**: A factory responsible for creating concrete
//!   instances of `WeatherProvider` (e.g., `OpenWeatherProvider`, `GoogleWeatherProvider`)
//!   based on the application's configuration.

use crate::config::{LocationConfig, WeatherApiProvider};
use crate::weather_api::openweather_api::{ApiError, ApiResponse, Location};
use async_trait::async_trait;

/// A trait for weather API providers.
///
/// This trait defines the common interface for all weather providers, allowing the application
/// to fetch weather data from different sources using a unified API.
#[async_trait]
pub trait WeatherProvider {
    /// Fetches weather data for a given location.
    ///
    /// # Arguments
    ///
    /// * `location` - A reference to the `LocationConfig` containing the location for which to fetch weather data.
    ///
    /// # Errors
    /// Returns an `ApiError` if the data cannot be fetched, for reasons such as network
    /// issues, an invalid API key, or the location not being found.
    async fn get_weather(&self, location: &LocationConfig) -> Result<ApiResponse, ApiError>;

    /// Returns the display name of the weather provider (e.g., "OpenWeather").
    #[allow(dead_code)]
    fn name(&self) -> &'static str;

    /// Returns `true` if the provider requires an API key to function.
    #[allow(dead_code)]
    fn requires_api_key(&self) -> bool;
}

/// A factory for creating weather providers.
///
/// This struct is responsible for creating instances of `WeatherProvider` based on the application's configuration.
pub struct WeatherProviderFactory;

impl WeatherProviderFactory {
    /// Creates a concrete `WeatherProvider` instance based on the specified type.
    ///
    /// # Arguments
    ///
    /// * `provider_type` - The type of weather provider to create.
    /// * `api_token` - An `Option` containing the API token, if required by the provider.
    ///
    /// # Errors
    /// Returns an error `String` if a required API token is missing for the selected provider.
    pub fn create_provider(
        provider_type: &WeatherApiProvider,
        api_token: Option<String>,
    ) -> Result<Box<dyn WeatherProvider + Send + Sync>, String> {
        match provider_type {
            WeatherApiProvider::OpenWeather => {
                let token = api_token.ok_or("OpenWeather API requires an API token")?;
                Ok(Box::new(super::openweather_api::OpenWeatherProvider::new(
                    token,
                )))
            }
            WeatherApiProvider::GoogleWeather => Ok(Box::new(
                super::google_weather_api::GoogleWeatherProvider::new(),
            )),
        }
    }
}

/// Converts an application-level `LocationConfig` to an API-level `Location` struct.
///
/// This helper function is used to prepare the location data for the `openweather_api`
/// functions. It initializes latitude and longitude to `0.0` as they will be
/// determined by the geocoding service within the API call.
///
/// # Arguments
///
/// * `config` - A reference to the `LocationConfig` to convert.
pub fn location_config_to_location(config: &LocationConfig) -> Location {
    Location {
        name: config.city.clone(),
        lat: 0.0,
        lon: 0.0,
        country: Some(config.country.clone()),
        state: Some(config.state.clone()),
    }
}
