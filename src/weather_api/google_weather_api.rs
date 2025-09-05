//! Google Weather API implementation (mockup)
//!
//! This module provides a mockup implementation of Google Weather API
//! for demonstration purposes. It returns sample weather data instead
//! of making actual API calls.

use crate::config::LocationConfig;
use crate::weather_api::openweather_api::{ApiError, ApiResponse, Main, Weather};
use crate::weather_api::weather_provider::WeatherProvider;
use async_trait::async_trait;

/// Google Weather API provider (mockup implementation)
pub struct GoogleWeatherProvider;

impl GoogleWeatherProvider {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl WeatherProvider for GoogleWeatherProvider {
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

    fn name(&self) -> &'static str {
        "Google Weather (Mockup)"
    }

    fn requires_api_key(&self) -> bool {
        false
    }
}
