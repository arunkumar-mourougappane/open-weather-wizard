//! Weather provider abstraction layer
//!
//! This module provides a trait-based abstraction over different weather API providers,
//! allowing the application to seamlessly switch between different weather services.

use async_trait::async_trait;
use crate::config::{WeatherApiProvider, LocationConfig};
use crate::weather_api::openweather_api::{ApiResponse, ApiError, Location};

/// Trait for weather API providers
#[async_trait]
pub trait WeatherProvider {
    /// Fetch weather data for the given location
    async fn get_weather(&self, location: &LocationConfig) -> Result<ApiResponse, ApiError>;
    
    /// Get the name of this provider
    fn name(&self) -> &'static str;
    
    /// Check if this provider requires an API key
    fn requires_api_key(&self) -> bool;
}

/// Factory for creating weather providers
pub struct WeatherProviderFactory;

impl WeatherProviderFactory {
    /// Create a weather provider based on the configuration
    pub fn create_provider(
        provider_type: &WeatherApiProvider,
        api_token: Option<String>,
    ) -> Result<Box<dyn WeatherProvider + Send + Sync>, String> {
        match provider_type {
            WeatherApiProvider::OpenWeather => {
                let token = api_token.ok_or("OpenWeather API requires an API token")?;
                Ok(Box::new(super::openweather_api::OpenWeatherProvider::new(token)))
            }
            WeatherApiProvider::GoogleWeather => {
                Ok(Box::new(super::google_weather_api::GoogleWeatherProvider::new()))
            }
        }
    }
}

/// Convert LocationConfig to Location for API calls
pub fn location_config_to_location(config: &LocationConfig) -> Location {
    Location {
        name: config.city.clone(),
        lat: 0.0,
        lon: 0.0,
        country: Some(config.country.clone()),
        state: Some(config.state.clone()),
    }
}