pub mod config;
pub mod ui;
pub mod weather_api;

#[cfg(test)]
mod tests {
    use crate::config::{AppConfig, LocationConfig, WeatherApiProvider};
    use crate::weather_api::weather_provider::WeatherProviderFactory;
    use base64::{Engine as _, engine::general_purpose::STANDARD};

    #[test]
    fn test_config_base64_encoding() {
        let mut config = AppConfig::default();
        let test_token = "test_api_key_12345";

        config.set_api_token(test_token);
        let decoded_token = config.get_api_token().unwrap();

        assert_eq!(test_token, decoded_token);
    }

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
}
