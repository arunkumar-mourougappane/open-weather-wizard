//! # Weather Wizard Library Crate
//!
//! This is the main library for the Weather Wizard application. It serves as the
//! root of the crate, organizing the application's core logic into distinct modules.
//!
//! ## Modules
//!
//! - **`app`**: The iced application root -- state, messages, `update()`/`view()`.
//! - **`config`**: Handles loading, saving, and managing application configuration.
//! - **`geolocation`**: Best-effort IP-based location detection, used to prefill
//!   the "Home" location during first-run setup.
//! - **`ui`**: Per-screen views for the [iced](https://github.com/iced-rs/iced) user interface.
//! - **`weather_api`**: Provides an abstraction layer for fetching data from various
//!   weather services.

pub mod app;
pub mod config;
pub mod geolocation;
pub mod ui;
pub mod weather_api;

/// Contains integration and unit tests for the library.
#[cfg(test)]
mod tests {
    use crate::config::{
        AppConfig, ConfigManager, Language, LocationConfig, SavedLocation, ThemePreference,
        WeatherApiProvider,
    };
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
        config.locations = vec![SavedLocation {
            name: "Home".to_string(),
            location: LocationConfig {
                city: "Test City".to_string(),
                state: "TS".to_string(),
                country: "TC".to_string(),
            },
        }];
        config.refresh_interval_secs = Some(900);
        config.launch_at_login = true;
        config.theme_preference = ThemePreference::Dark;
        config.language = Language::Spanish;

        let json = serde_json::to_string(&config).unwrap();
        assert!(json.contains("GoogleWeather"));
        assert!(json.contains("Test City"));
        assert!(json.contains("refresh_interval_secs"));
        assert!(json.contains("900"));
        assert!(json.contains("launch_at_login"));
        assert!(json.contains("theme_preference"));
        assert!(json.contains("\"language\":\"Spanish\""));
        assert!(json.contains("\"locations\""));
        assert!(!json.contains("dark_mode"));
        assert!(!json.contains("api_token"));

        let deserialized: AppConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.locations[0].location.city, "Test City");
        assert_eq!(deserialized.refresh_interval_secs, Some(900));
        assert!(deserialized.launch_at_login);
        assert_eq!(deserialized.theme_preference, ThemePreference::Dark);
        assert_eq!(deserialized.language, Language::Spanish);

        // Test migration-safe default where refresh_interval_secs and
        // launch_at_login are missing, and theme_preference has never been
        // written (no "dark_mode" key at all either) -- should default to
        // System, not fall back to Light. language should default to
        // English, matching both providers' own API default.
        let missing_interval_json = r#"{"weather_provider":"OpenWeather","location":{"city":"Peoria","state":"IL","country":"US"},"use_fahrenheit":false}"#;
        let deserialized_default: AppConfig = serde_json::from_str(missing_interval_json).unwrap();
        assert_eq!(deserialized_default.refresh_interval_secs, None);
        assert!(!deserialized_default.launch_at_login);
        assert_eq!(
            deserialized_default.theme_preference,
            ThemePreference::System
        );
        assert_eq!(deserialized_default.language, Language::English);
    }

    /// Verifies the one deliberate divergence between the two providers'
    /// language codes: OpenWeatherMap's `lang` parameter uses its own `"kr"`
    /// for Korean rather than ISO 639-1/BCP-47's `"ko"`, which Google
    /// Weather's `languageCode` does use. Every other curated language
    /// shares an identical code across both providers -- a regression here
    /// (e.g. someone "simplifying" the two match arms into one) would
    /// silently request the wrong provider's descriptions in the wrong
    /// language for Korean users specifically, with no error to surface it.
    #[test]
    fn test_language_codes_diverge_only_for_korean() {
        use crate::config::Language;

        let all = [
            Language::English,
            Language::Spanish,
            Language::French,
            Language::German,
            Language::Italian,
            Language::Portuguese,
            Language::Russian,
            Language::Japanese,
            Language::Korean,
            Language::Arabic,
            Language::Hindi,
            Language::Dutch,
        ];

        for language in all {
            if language == Language::Korean {
                assert_eq!(language.openweather_code(), "kr");
                assert_eq!(language.google_code(), "ko");
            } else {
                assert_eq!(
                    language.openweather_code(),
                    language.google_code(),
                    "{language} should share the same code across both providers"
                );
            }
        }
    }

    /// Verifies that the refresh interval validation logic enforces the
    /// Google Weather rate limits while allowing fast intervals for OpenWeather.
    #[test]
    fn test_refresh_interval_validation() {
        use crate::ui::preferences::{RefreshIntervalPreset, State as PrefsState};

        let mut config = AppConfig::default();
        config.weather_provider = WeatherApiProvider::OpenWeather;
        config.refresh_interval_secs = Some(30);

        // OpenWeather with 30 seconds is valid. `AppConfig::default()`
        // already seeds a valid Peoria/IL/US "Home" location, so only the
        // token needs filling in for the other validators not to trigger.
        let mut prefs_state = PrefsState::from_config(&config);
        prefs_state.token_input = "dummy_token".to_string();

        let errors = prefs_state.validation_errors();
        assert!(
            errors.is_empty(),
            "OpenWeather with 30s should be valid: {:?}",
            errors
        );

        // Switching provider to Google Weather with 30s preset is invalid.
        prefs_state.provider = WeatherApiProvider::GoogleWeather;
        prefs_state.refresh_interval = RefreshIntervalPreset::ThirtySeconds;
        let errors = prefs_state.validation_errors();
        assert!(
            !errors.is_empty(),
            "Google Weather with 30s should be invalid"
        );
        assert!(errors.iter().any(|e| {
            e.contains("Google Weather requires a refresh interval of at least 15 minutes")
        }));

        // Google Weather with 15 minutes is valid.
        prefs_state.refresh_interval = RefreshIntervalPreset::FifteenMinutes;
        let errors = prefs_state.validation_errors();
        assert!(
            errors.is_empty(),
            "Google Weather with 15m should be valid: {:?}",
            errors
        );
    }

    /// Verifies the Preferences "Locations" list's add/remove/reorder
    /// messages (issue #55): a new entry is appended and selected, removal
    /// is refused once only one location remains (rather than emptying the
    /// list), and Move Up/Down swap the selected entry with its neighbor
    /// while following the selection.
    #[test]
    fn test_location_list_management() {
        use crate::ui::preferences::{self, State as PrefsState};

        let config = AppConfig::default();
        let mut prefs_state = PrefsState::from_config(&config);
        assert_eq!(prefs_state.locations.len(), 1);

        // Adding selects the new (blank) entry.
        preferences::update(&mut prefs_state, preferences::Message::AddLocationRequested);
        assert_eq!(prefs_state.locations.len(), 2);
        assert_eq!(prefs_state.selected_location_index, 1);

        preferences::update(
            &mut prefs_state,
            preferences::Message::LocationNameChanged("Work".to_string()),
        );
        assert_eq!(prefs_state.locations[1].name, "Work");

        // Move the new entry up to the front, following the selection.
        preferences::update(&mut prefs_state, preferences::Message::MoveLocationUp);
        assert_eq!(prefs_state.selected_location_index, 0);
        assert_eq!(prefs_state.locations[0].name, "Work");
        assert_eq!(prefs_state.locations[1].name, "Home");

        // Removing down to one entry works...
        preferences::update(
            &mut prefs_state,
            preferences::Message::RemoveLocationRequested,
        );
        assert_eq!(prefs_state.locations.len(), 1);
        assert_eq!(prefs_state.locations[0].name, "Home");

        // ...but removing the last one is refused.
        preferences::update(
            &mut prefs_state,
            preferences::Message::RemoveLocationRequested,
        );
        assert_eq!(
            prefs_state.locations.len(),
            1,
            "the last saved location must never be removable"
        );
    }

    /// Verifies that every saved location needs a name, city, and country
    /// before Save is allowed -- multiple entries are each validated, not
    /// just whichever one is currently selected in the form.
    #[test]
    fn test_location_list_validation() {
        use crate::ui::preferences::{LocationEntry, State as PrefsState};

        let config = AppConfig::default();
        let mut prefs_state = PrefsState::from_config(&config);
        prefs_state.token_input = "dummy_token".to_string();
        assert!(prefs_state.validation_errors().is_empty());

        prefs_state.locations.push(LocationEntry {
            name: String::new(),
            city: String::new(),
            state: String::new(),
            country: "US".to_string(),
        });

        let errors = prefs_state.validation_errors();
        assert!(errors.iter().any(|e| e.contains("needs a name")));
        assert!(errors.iter().any(|e| e.contains("needs a city")));
    }

    /// Verifies that `apply_to` keeps `AppConfig.current_location_index`
    /// pointing at the location the main window was actually showing,
    /// through operations that move entries around underneath it --
    /// renaming the active entry (its position doesn't change, so tracking
    /// it by name would break here but tracking it by position doesn't),
    /// and reordering it to a different position. See `State::
    /// current_location_index`'s docs for why this is tracked as a
    /// position rather than re-derived from a name after the fact.
    #[test]
    fn test_apply_to_preserves_current_location_through_rename_and_reorder() {
        use crate::ui::preferences::{self, State as PrefsState};

        // apply_to writes token_input via set_api_token, an OS keychain
        // call -- switch to the mock backend like every other test that
        // exercises apply_to/set_api_token.
        let _guard = lock_mock_keyring();

        let mut config = AppConfig::default();
        config.locations.push(SavedLocation {
            name: "Work".to_string(),
            location: LocationConfig {
                city: "Chicago".to_string(),
                state: "IL".to_string(),
                country: "US".to_string(),
            },
        });
        // "Home" (index 0) is current.
        assert_eq!(config.current_location_index, 0);

        let mut prefs_state = PrefsState::from_config(&config);
        prefs_state.token_input = "dummy_token".to_string();

        // Rename the currently-active entry -- a name-based lookup would
        // lose track of it here, since "Home" no longer exists afterward.
        preferences::update(&mut prefs_state, preferences::Message::LocationSelected(0));
        preferences::update(
            &mut prefs_state,
            preferences::Message::LocationNameChanged("Apartment".to_string()),
        );
        prefs_state.apply_to(&mut config).unwrap();
        assert_eq!(config.current_location_index, 0);
        assert_eq!(config.current_location_name(), "Apartment");
        assert_eq!(config.current_location().city, "Peoria");

        // Now move it down a position -- the tracked index should follow.
        preferences::update(&mut prefs_state, preferences::Message::MoveLocationDown);
        prefs_state.apply_to(&mut config).unwrap();
        assert_eq!(config.current_location_index, 1);
        assert_eq!(config.current_location_name(), "Apartment");
        assert_eq!(config.current_location().city, "Peoria");
    }

    /// Verifies that removing the currently-active location falls back to
    /// index 0, the same "don't crash, degrade to something sane" behavior
    /// as an out-of-range index.
    #[test]
    fn test_apply_to_falls_back_when_current_location_removed() {
        use crate::ui::preferences::{self, State as PrefsState};

        let _guard = lock_mock_keyring();

        let mut config = AppConfig::default();
        config.locations.push(SavedLocation {
            name: "Work".to_string(),
            location: LocationConfig {
                city: "Chicago".to_string(),
                state: "IL".to_string(),
                country: "US".to_string(),
            },
        });

        let mut prefs_state = PrefsState::from_config(&config);
        prefs_state.token_input = "dummy_token".to_string();

        // Remove "Home" (the currently-active entry) while it's selected.
        preferences::update(&mut prefs_state, preferences::Message::LocationSelected(0));
        preferences::update(
            &mut prefs_state,
            preferences::Message::RemoveLocationRequested,
        );
        prefs_state.apply_to(&mut config).unwrap();

        assert_eq!(config.locations.len(), 1);
        assert_eq!(config.current_location_index, 0);
        assert_eq!(config.current_location_name(), "Work");
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
        // The token migration's forced re-save happens after
        // `migrate_legacy_dark_mode` has already updated `theme_preference`
        // in memory, so the rewritten file reflects the new field too even
        // though dark-mode migration itself never triggers its own save.
        assert!(saved.contains("theme_preference"));
        assert!(!saved.contains("dark_mode"));

        let _ = std::fs::remove_file(&config_path);
    }

    /// Verifies that a config file saved by a version of this app before
    /// `dark_mode: bool` became `theme_preference: ThemePreference` has its
    /// old explicit boolean mapped onto `Light`/`Dark` on load (never onto
    /// `System`, since a file that had `dark_mode` at all always reflected
    /// an explicit choice). Unlike the token migration, this one doesn't
    /// force an immediate re-save, so the legacy `dark_mode` key is still
    /// expected to linger in the file until the next real Save.
    #[test]
    fn test_legacy_dark_mode_migration() {
        let config_path = std::env::temp_dir().join(format!(
            "open-weather-wizard-dark-mode-migration-test-{:?}.json",
            std::thread::current().id()
        ));

        let legacy_json = r#"{"weather_provider":"OpenWeather","location":{"city":"Test City","state":"TS","country":"TC"},"dark_mode":true,"use_fahrenheit":false}"#;
        std::fs::write(&config_path, legacy_json).unwrap();

        let manager = ConfigManager::for_path(config_path.clone());
        let config = manager.load_config();

        assert_eq!(config.theme_preference, ThemePreference::Dark);

        let untouched = std::fs::read_to_string(&config_path).unwrap();
        assert!(
            untouched.contains("dark_mode"),
            "dark-mode migration should not force a re-save the way token migration does"
        );

        let _ = std::fs::remove_file(&config_path);
    }

    /// Verifies that a config file saved by a version of this app before
    /// the single `location: LocationConfig` field became
    /// `locations: Vec<SavedLocation>` (issue #55) has its old location
    /// wrapped into a one-entry list named "Home", with
    /// `current_location_index` pointing at it. Like
    /// `test_legacy_dark_mode_migration`, this doesn't force a re-save, so
    /// the legacy `location` key is expected to linger until the next real
    /// Save.
    #[test]
    fn test_legacy_location_migration() {
        let config_path = std::env::temp_dir().join(format!(
            "open-weather-wizard-location-migration-test-{:?}.json",
            std::thread::current().id()
        ));

        let legacy_json = r#"{"weather_provider":"OpenWeather","location":{"city":"Test City","state":"TS","country":"TC"},"dark_mode":false,"use_fahrenheit":false}"#;
        std::fs::write(&config_path, legacy_json).unwrap();

        let manager = ConfigManager::for_path(config_path.clone());
        let config = manager.load_config();

        assert_eq!(config.locations.len(), 1);
        assert_eq!(config.locations[0].name, "Home");
        assert_eq!(config.locations[0].location.city, "Test City");
        assert_eq!(config.current_location_index, 0);
        assert_eq!(config.current_location().city, "Test City");
        assert_eq!(config.current_location_name(), "Home");

        let untouched = std::fs::read_to_string(&config_path).unwrap();
        assert!(
            untouched.contains("\"location\""),
            "location migration should not force a re-save the way token migration does"
        );

        let _ = std::fs::remove_file(&config_path);
    }

    /// Verifies `AppConfig::current_location`/`current_location_name`
    /// degrade gracefully (fall back to the first entry, or a fresh
    /// default if the list is somehow empty) rather than panicking when
    /// `current_location_index` is out of range -- e.g. a hand-edited
    /// config file, since nothing else in this codebase can produce that
    /// state through normal use.
    #[test]
    fn test_current_location_out_of_range_falls_back() {
        let mut config = AppConfig::default();
        config.current_location_index = 5;
        assert_eq!(config.current_location(), config.locations[0].location);
        assert_eq!(config.current_location_name(), "Home");

        config.locations = vec![];
        assert_eq!(config.current_location(), LocationConfig::default());
        assert_eq!(config.current_location_name(), "Home");

        config.locations = vec![SavedLocation {
            name: "Only One".to_string(),
            location: LocationConfig::default(),
        }];
        config.current_location_index = 0;
        assert_eq!(config.current_location_name(), "Only One");
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
            Language::English,
        );
        assert!(result.is_ok());

        // Test missing API key for OpenWeather
        let result = WeatherProviderFactory::create_provider(
            &WeatherApiProvider::OpenWeather,
            None,
            Language::English,
        );
        assert!(result.is_err());

        // Google Weather now requires a real API token, same as OpenWeather.
        let result = WeatherProviderFactory::create_provider(
            &WeatherApiProvider::GoogleWeather,
            None,
            Language::English,
        );
        assert!(result.is_err());

        let result = WeatherProviderFactory::create_provider(
            &WeatherApiProvider::GoogleWeather,
            Some("test_key".to_string()),
            Language::English,
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
        config.locations = vec![SavedLocation {
            name: "Home".to_string(),
            location: LocationConfig {
                city: "Test City".to_string(),
                state: "TS".to_string(),
                country: "TC".to_string(),
            },
        }];
        config.set_api_token("test_token").unwrap();

        let shared_config = Arc::new(Mutex::new(config));

        // Test reading from the Arc<Mutex<AppConfig>>
        {
            let config_guard = shared_config.lock().unwrap();
            assert_eq!(config_guard.locations[0].location.city, "Test City");
            assert_eq!(config_guard.get_api_token().unwrap(), "test_token");
        }

        // Test writing to the Arc<Mutex<AppConfig>>
        {
            let mut config_guard = shared_config.lock().unwrap();
            config_guard.locations[0].location.city = "Updated City".to_string();
            config_guard.set_api_token("new_token").unwrap();
        }

        // Verify the changes
        {
            let config_guard = shared_config.lock().unwrap();
            assert_eq!(config_guard.locations[0].location.city, "Updated City");
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
