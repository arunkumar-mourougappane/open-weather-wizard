//! # Weather Wizard Library Crate
//!
//! This is the main library for the Weather Wizard application. It serves as the
//! root of the crate, organizing the application's core logic into distinct modules.
//!
//! ## Modules
//!
//! - **`app`**: The iced application root -- state, messages, `update()`/`view()`.
//! - **`config`**: Handles loading, saving, and managing application configuration.
//! - **`ui`**: Per-screen views for the [iced](https://github.com/iced-rs/iced) user interface.
//! - **`weather_api`**: Provides an abstraction layer for fetching data from various
//!   weather services.

pub mod app;
pub mod config;
pub mod ui;
pub mod weather_api;

/// Contains integration and unit tests for the library.
#[cfg(test)]
mod tests {
    use crate::config::{AppConfig, ConfigManager, LocationConfig, WeatherApiProvider};
    use crate::weather_api::weather_provider::WeatherProviderFactory;

    /// The API token lives in a single OS-keyring entry shared by the whole
    /// process (see `config`'s `API_TOKEN_ENTRY`), and the mock credential
    /// backend used here doesn't key entries by service/user at all -- every
    /// test that reads or writes a token must run exclusive of every other
    /// one, or they'll observe each other's writes. Rust's default test
    /// harness runs tests in parallel threads within one process, so every
    /// such test acquires this lock (and switches to the mock backend)
    /// before touching a token, and holds it for its entire body via the
    /// returned guard.
    static TOKEN_TEST_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

    fn lock_mock_keyring() -> std::sync::MutexGuard<'static, ()> {
        let guard = TOKEN_TEST_LOCK
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        keyring::set_default_credential_builder(keyring::mock::default_credential_builder());
        guard
    }

    /// Tests that a token set via `set_api_token` reads back unchanged via
    /// `get_api_token` -- i.e. the OS keyring round-trip works, using the
    /// crate's mock backend so this never touches the real OS keychain.
    #[test]
    fn test_api_token_roundtrip() {
        let _guard = lock_mock_keyring();
        let mut config = AppConfig::default();
        let test_token = "test_api_key_12345";

        config.set_api_token(test_token).unwrap();
        let round_tripped = config.get_api_token().unwrap();

        assert_eq!(test_token, round_tripped);
    }

    /// Verifies that the `AppConfig` struct can be serialized to and
    /// deserialized from JSON. The API token is deliberately not part of
    /// this -- it never lives in the JSON at all anymore, only in the OS
    /// keyring (see `test_api_token_roundtrip` and
    /// `test_legacy_token_migration`).
    #[test]
    fn test_config_serialization() {
        let mut config = AppConfig::default();
        config.weather_provider = WeatherApiProvider::GoogleWeather;
        config.location = LocationConfig {
            city: "Test City".to_string(),
            state: "TS".to_string(),
            country: "TC".to_string(),
        };

        let json = serde_json::to_string(&config).unwrap();
        assert!(json.contains("GoogleWeather"));
        assert!(json.contains("Test City"));
        assert!(!json.contains("api_token"));

        let deserialized: AppConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.location.city, "Test City");
    }

    /// Verifies that a config file saved by an older version of this app
    /// (a base64 `api_token_encoded` field alongside the other settings)
    /// has its token transparently migrated into the OS keyring on load,
    /// and that the field is gone from the file after that migration's
    /// automatic re-save -- so it isn't re-attempted (or re-exposed) on
    /// every subsequent launch.
    #[test]
    fn test_legacy_token_migration() {
        use base64::{Engine as _, engine::general_purpose::STANDARD};

        let _guard = lock_mock_keyring();

        let config_path = std::env::temp_dir().join(format!(
            "open-weather-wizard-migration-test-{:?}.json",
            std::thread::current().id()
        ));
        let legacy_token = "legacy-secret-token";
        let legacy_json = format!(
            r#"{{"weather_provider":"OpenWeather","location":{{"city":"Test City","state":"TS","country":"TC"}},"dark_mode":false,"use_fahrenheit":false,"api_token_encoded":"{}"}}"#,
            STANDARD.encode(legacy_token)
        );
        std::fs::write(&config_path, legacy_json).unwrap();

        let manager = ConfigManager::for_path(config_path.clone());
        let config = manager.load_config();

        assert_eq!(config.get_api_token().unwrap(), legacy_token);

        let saved = std::fs::read_to_string(&config_path).unwrap();
        assert!(
            !saved.contains("api_token_encoded"),
            "migration should have rewritten the file without the legacy field"
        );

        let _ = std::fs::remove_file(&config_path);
    }

    /// Verifies `ConfigManager::config_exists` -- the signal `app::boot`
    /// uses to detect a fresh install (see issue #38) -- correctly reports
    /// `false` before any config has ever been saved and `true` immediately
    /// after `save_config` first writes the file.
    #[test]
    fn test_config_manager_detects_first_run() {
        let config_path = std::env::temp_dir().join(format!(
            "open-weather-wizard-first-run-test-{:?}.json",
            std::thread::current().id()
        ));
        let _ = std::fs::remove_file(&config_path);

        let manager = ConfigManager::for_path(config_path.clone());
        assert!(
            !manager.config_exists(),
            "no config file has been saved yet"
        );

        manager.save_config(&AppConfig::default()).unwrap();
        assert!(
            manager.config_exists(),
            "config_exists should be true immediately after save_config"
        );

        let _ = std::fs::remove_file(&config_path);
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

        // Google Weather now requires a real API token, same as OpenWeather.
        let result =
            WeatherProviderFactory::create_provider(&WeatherApiProvider::GoogleWeather, None);
        assert!(result.is_err());

        let result = WeatherProviderFactory::create_provider(
            &WeatherApiProvider::GoogleWeather,
            Some("test_key".to_string()),
        );
        assert!(result.is_ok());
    }

    /// Verifies that the `AppConfig` can be safely shared and mutated across threads using `Arc<Mutex<>>`.
    #[test]
    fn test_arc_mutex_config_access() {
        use std::sync::{Arc, Mutex};

        let _guard = lock_mock_keyring();

        let mut config = AppConfig::default();
        config.weather_provider = WeatherApiProvider::OpenWeather;
        config.location = LocationConfig {
            city: "Test City".to_string(),
            state: "TS".to_string(),
            country: "TC".to_string(),
        };
        config.set_api_token("test_token").unwrap();

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
            config_guard.set_api_token("new_token").unwrap();
        }

        // Verify the changes
        {
            let config_guard = shared_config.lock().unwrap();
            assert_eq!(config_guard.location.city, "Updated City");
            assert_eq!(config_guard.get_api_token().unwrap(), "new_token");
        }
    }

    /// Verifies that `aggregate_daily` buckets 3-hourly entries by UTC calendar
    /// date, computes correct min/max temperatures per day, picks the midday
    /// entry's condition as the day's dominant/representative condition, pulls
    /// feels-like/humidity/wind/pressure/visibility from that same
    /// representative entry, and takes the **max** `pop` across the whole day
    /// rather than just the representative entry's value.
    #[test]
    fn test_forecast_aggregation() {
        use crate::weather_api::forecast::{
            ForecastCity, ForecastListItem, RawForecastResponse, aggregate_daily,
        };
        use crate::weather_api::openweather_api::{Main, Weather, Wind};

        let item = |dt_txt: &str, temp: f64, main: &str, pop: f64| ForecastListItem {
            dt: 0,
            main: Main {
                temp,
                feels_like: temp,
                temp_min: temp,
                temp_max: temp,
                pressure: 1013,
                humidity: 50,
            },
            weather: vec![Weather {
                main: main.to_string(),
                description: format!("{main} description"),
            }],
            wind: Wind {
                speed: 5.0,
                deg: 180,
            },
            pop,
            visibility: 10_000,
            dt_txt: dt_txt.to_string(),
        };

        let raw = RawForecastResponse {
            city: ForecastCity {
                name: "Test City".to_string(),
            },
            list: vec![
                // Day 1: cold overnight, midday is Rain -- should be the dominant condition.
                // pop peaks well before midday, so the day's pop must be the
                // max across all entries, not just the representative one.
                item("2026-07-02 00:00:00", 10.0, "Clouds", 0.1),
                item("2026-07-02 03:00:00", 8.0, "Clouds", 0.9),
                item("2026-07-02 12:00:00", 15.0, "Rain", 0.3),
                item("2026-07-02 21:00:00", 12.0, "Clouds", 0.2),
                // Day 2: no midday entry -- falls back to the most frequent condition (Clear).
                item("2026-07-03 00:00:00", 18.0, "Clear", 0.0),
                item("2026-07-03 03:00:00", 16.0, "Clear", 0.4),
                item("2026-07-03 21:00:00", 20.0, "Clouds", 0.5),
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
        // Representative entry is the midday Rain reading (temp 15.0).
        assert_eq!(day1.feels_like, 15.0);
        assert_eq!(day1.humidity, 50);
        assert_eq!(day1.wind_speed, 5.0);
        assert_eq!(day1.wind_deg, 180);
        assert_eq!(day1.pressure, 1013);
        assert_eq!(day1.visibility, 10_000);
        // Max across the day (0.9 at 03:00), not the representative entry's 0.3.
        assert_eq!(day1.pop, 0.9);

        let day2 = &forecast.days[1];
        assert_eq!(day2.date, "2026-07-03");
        assert_eq!(day2.temp_min, 16.0);
        assert_eq!(day2.temp_max, 20.0);
        assert_eq!(day2.description, "Clear description");
        assert_eq!(day2.pop, 0.5);
    }

    /// Verifies the hand-authored Lottie assets under `assets/lottie/` are
    /// valid, parseable compositions with a non-empty, finite frame range --
    /// catches malformed JSON before it ever reaches the animated-icon widget.
    #[test]
    fn test_lottie_assets_parse() {
        let assets: [(&str, &str); 10] = [
            ("sun", include_str!("../assets/lottie/sun.json")),
            ("clouds", include_str!("../assets/lottie/clouds.json")),
            ("rain", include_str!("../assets/lottie/rain.json")),
            ("snow", include_str!("../assets/lottie/snow.json")),
            ("drizzle", include_str!("../assets/lottie/drizzle.json")),
            (
                "thunderstorm",
                include_str!("../assets/lottie/thunderstorm.json"),
            ),
            ("fog", include_str!("../assets/lottie/fog.json")),
            ("haze", include_str!("../assets/lottie/haze.json")),
            ("wind", include_str!("../assets/lottie/wind.json")),
            ("tornado", include_str!("../assets/lottie/tornado.json")),
        ];

        for (name, json) in assets {
            let composition = velato::Composition::from_slice(json.as_bytes())
                .unwrap_or_else(|e| panic!("{name}.json failed to parse: {e:?}"));
            assert!(
                composition.frames.end > composition.frames.start,
                "{name}.json must have a non-empty frame range"
            );
            assert!(
                !composition.layers.is_empty(),
                "{name}.json must have at least one layer"
            );
        }
    }
}
