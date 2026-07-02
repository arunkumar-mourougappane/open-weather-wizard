//! # Weather Wizard Library Crate
//!
//! This is the main library for the Weather Wizard application. It serves as the
//! root of the crate, organizing the application's core logic into distinct modules.
//!
//! ## Modules
//!
//! - **`config`**: Handles loading, saving, and managing application configuration.
//! - **`ui`**: Contains all logic for building and managing the GTK user interface.
//! - **`weather_api`**: Provides an abstraction layer for fetching data from various
//!   weather services.

pub mod app;
pub mod config;
pub mod ui;
pub mod weather_api;

/// Contains integration and unit tests for the library.
#[cfg(test)]
mod tests {
    use crate::config::{AppConfig, LocationConfig, WeatherApiProvider};
    use crate::weather_api::weather_provider::WeatherProviderFactory;
    use base64::{Engine as _, engine::general_purpose::STANDARD};

    /// Tests that the API token is correctly encoded to and decoded from base64.
    #[test]
    fn test_config_base64_encoding() {
        let mut config = AppConfig::default();
        let test_token = "test_api_key_12345";

        config.set_api_token(test_token);
        let decoded_token = config.get_api_token().unwrap();

        assert_eq!(test_token, decoded_token);
    }

    /// Verifies that the `AppConfig` struct can be serialized to and deserialized from JSON.
    #[test]
    fn test_config_serialization() {
        let config = AppConfig {
            weather_provider: WeatherApiProvider::GoogleWeather,
            api_token_encoded: STANDARD.encode("test_key"),
            location: LocationConfig {
                city: "Test City".to_string(),
                state: "TS".to_string(),
                country: "TC".to_string(),
            },
        };

        let json = serde_json::to_string(&config).unwrap();
        assert!(json.contains("GoogleWeather"));
        assert!(json.contains("Test City"));

        let deserialized: AppConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.location.city, "Test City");
    }

    /// Tests the `WeatherProviderFactory`'s ability to create providers.
    ///
    /// This test covers both successful creation and error handling for missing API keys.
    #[test]
    fn test_weather_provider_factory() {
        // Test OpenWeather provider creation
        let result = WeatherProviderFactory::create_provider(
            &WeatherApiProvider::OpenWeather,
            Some("test_key".to_string()),
        );
        assert!(result.is_ok());

        // Test missing API key for OpenWeather
        let result =
            WeatherProviderFactory::create_provider(&WeatherApiProvider::OpenWeather, None);
        assert!(result.is_err());

        // Test Google Weather provider (doesn't need API key)
        let result =
            WeatherProviderFactory::create_provider(&WeatherApiProvider::GoogleWeather, None);
        assert!(result.is_ok());
    }

    /// Verifies that the `AppConfig` can be safely shared and mutated across threads using `Arc<Mutex<>>`.
    #[test]
    fn test_arc_mutex_config_access() {
        use std::sync::{Arc, Mutex};

        let config = AppConfig {
            weather_provider: WeatherApiProvider::OpenWeather,
            api_token_encoded: STANDARD.encode("test_token"),
            location: LocationConfig {
                city: "Test City".to_string(),
                state: "TS".to_string(),
                country: "TC".to_string(),
            },
        };

        let shared_config = Arc::new(Mutex::new(config));

        // Test reading from the Arc<Mutex<AppConfig>>
        {
            let config_guard = shared_config.lock().unwrap();
            assert_eq!(config_guard.location.city, "Test City");
            assert_eq!(config_guard.get_api_token().unwrap(), "test_token");
        }

        // Test writing to the Arc<Mutex<AppConfig>>
        {
            let mut config_guard = shared_config.lock().unwrap();
            config_guard.location.city = "Updated City".to_string();
            config_guard.set_api_token("new_token");
        }

        // Verify the changes
        {
            let config_guard = shared_config.lock().unwrap();
            assert_eq!(config_guard.location.city, "Updated City");
            assert_eq!(config_guard.get_api_token().unwrap(), "new_token");
        }
    }

    /// An asynchronous test to verify that the mock `GoogleWeatherProvider` works as expected.
    #[tokio::test]
    async fn test_google_weather_provider() {
        use crate::weather_api::google_weather_api::GoogleWeatherProvider;
        use crate::weather_api::weather_provider::WeatherProvider;

        let provider = GoogleWeatherProvider::new();
        let location = LocationConfig {
            city: "Test City".to_string(),
            state: "TS".to_string(),
            country: "TC".to_string(),
        };

        let result = provider.get_weather(&location).await;
        assert!(result.is_ok());

        let weather_data = result.unwrap();
        assert_eq!(weather_data.name, "Test City");
        assert!(weather_data.weather[0].description.contains("mock"));
    }

    /// Verifies that `aggregate_daily` buckets 3-hourly entries by UTC calendar
    /// date, computes correct min/max temperatures per day, and picks the midday
    /// entry's condition as the day's dominant/representative condition.
    #[test]
    fn test_forecast_aggregation() {
        use crate::weather_api::forecast::{
            ForecastCity, ForecastListItem, RawForecastResponse, aggregate_daily,
        };
        use crate::weather_api::openweather_api::{Main, Weather};

        let item = |dt_txt: &str, temp: f64, main: &str| ForecastListItem {
            dt: 0,
            main: Main { temp, humidity: 50 },
            weather: vec![Weather {
                main: main.to_string(),
                description: format!("{main} description"),
            }],
            dt_txt: dt_txt.to_string(),
        };

        let raw = RawForecastResponse {
            city: ForecastCity {
                name: "Test City".to_string(),
            },
            list: vec![
                // Day 1: cold overnight, midday is Rain -- should be the dominant condition.
                item("2026-07-02 00:00:00", 10.0, "Clouds"),
                item("2026-07-02 03:00:00", 8.0, "Clouds"),
                item("2026-07-02 12:00:00", 15.0, "Rain"),
                item("2026-07-02 21:00:00", 12.0, "Clouds"),
                // Day 2: no midday entry -- falls back to the most frequent condition (Clear).
                item("2026-07-03 00:00:00", 18.0, "Clear"),
                item("2026-07-03 03:00:00", 16.0, "Clear"),
                item("2026-07-03 21:00:00", 20.0, "Clouds"),
            ],
        };

        let forecast = aggregate_daily(raw);

        assert_eq!(forecast.location_name, "Test City");
        assert_eq!(forecast.days.len(), 2);

        let day1 = &forecast.days[0];
        assert_eq!(day1.date, "2026-07-02");
        assert_eq!(day1.temp_min, 8.0);
        assert_eq!(day1.temp_max, 15.0);
        assert_eq!(day1.description, "Rain description");

        let day2 = &forecast.days[1];
        assert_eq!(day2.date, "2026-07-03");
        assert_eq!(day2.temp_min, 16.0);
        assert_eq!(day2.temp_max, 20.0);
        assert_eq!(day2.description, "Clear description");
    }

    /// Verifies that `GoogleWeatherProvider::get_forecast` returns an empty
    /// placeholder rather than fabricated forecast data.
    #[tokio::test]
    async fn test_google_weather_forecast_is_empty() {
        use crate::weather_api::google_weather_api::GoogleWeatherProvider;
        use crate::weather_api::weather_provider::WeatherProvider;

        let provider = GoogleWeatherProvider::new();
        let location = LocationConfig {
            city: "Test City".to_string(),
            state: "TS".to_string(),
            country: "TC".to_string(),
        };

        let result = provider.get_forecast(&location).await;
        assert!(result.is_ok());

        let forecast = result.unwrap();
        assert_eq!(forecast.location_name, "Test City");
        assert!(forecast.days.is_empty());
    }
}
