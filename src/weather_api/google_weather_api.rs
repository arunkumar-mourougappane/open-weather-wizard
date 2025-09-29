//! # Google Weather API Provider (Mockup)
//!
//! This module provides a mock implementation of a `WeatherProvider` for the
//! "Google Weather" API. It is designed for development, testing, and
//! demonstration purposes.
//!
//! Instead of making a real network request to a Google Weather service, this
//! provider simulates an API call by introducing a short delay and then returning
//! hardcoded, sample weather data. This allows for UI development and testing
//! of the provider abstraction layer without needing a network connection or a
//! valid API key.

use crate::config::LocationConfig;
use crate::weather_api::openweather_api::{ApiError, ApiResponse, Main, Weather};
use crate::weather_api::weather_provider::WeatherProvider;
use async_trait::async_trait;

/// A mock implementation of the `WeatherProvider` trait for Google Weather.
///
/// This provider returns sample weather data and is intended for demonstration purposes.
pub struct GoogleWeatherProvider;

impl Default for GoogleWeatherProvider {
    fn default() -> Self {
        Self::new()
    }
}

impl GoogleWeatherProvider {
    /// Creates a new instance of the `GoogleWeatherProvider`.
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl WeatherProvider for GoogleWeatherProvider {
    /// Simulates fetching weather data for a given location.
    ///
    /// This is a mock implementation that introduces a 500ms delay to simulate
    /// network latency and then returns a hardcoded `ApiResponse` with sample
    /// weather data. The city name in the response is taken from the input `location`.
    ///
    /// # Arguments
    ///
    /// * `location` - A reference to the `LocationConfig` for which to generate mock weather data.
    ///
    /// # Returns
    ///
    /// A `Result` containing a mock `ApiResponse` on success. This implementation never returns an `ApiError`.
    async fn get_weather(&self, location: &LocationConfig) -> Result<ApiResponse, ApiError> {
        log::info!(
            "Google Weather API (mockup) called for location: {}, {}, {}",
            location.city,
            location.state,
            location.country
        );

        // Simulate API delay
        tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;

        // Return mock weather data
        let mock_response = ApiResponse {
            weather: vec![Weather {
                main: "Clear".to_string(),
                description: "clear sky (mock data)".to_string(),
            }],
            main: Main {
                temp: 22.5,
                humidity: 65,
            },
            name: location.city.clone(),
        };

        log::info!("Google Weather API (mockup) returning mock data");
        Ok(mock_response)
    }

    /// Returns the display name of the weather provider.
    fn name(&self) -> &'static str {
        "Google Weather (Mockup)"
    }

    /// Returns `false` as this mock provider does not require an API key.
    fn requires_api_key(&self) -> bool {
        false
    }
}
