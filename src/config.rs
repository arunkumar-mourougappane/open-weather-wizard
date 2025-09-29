//! # Application Configuration Management
//!
//! This module handles loading, saving, and managing application configuration,
//! which is persisted to a JSON file in the user's config directory.
//!
//! ## Key Components
//!
//! - **`AppConfig`**: The main struct representing all user-configurable settings,
//!   including the chosen weather provider, location, and API token.
//! - **`ConfigManager`**: A utility struct responsible for handling the file I/O
//!   for loading from and saving to `config.json`.
//! - **API Token Handling**: The `AppConfig` struct includes methods to set and get
//!   the API token, which is stored in a base64-encoded format for basic obfuscation.

use base64::{Engine as _, engine::general_purpose::STANDARD};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

/// An enum representing the supported weather API providers.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum WeatherApiProvider {
    OpenWeather,
    GoogleWeather,
}

impl Default for WeatherApiProvider {
    fn default() -> Self {
        Self::OpenWeather
    }
}

impl std::fmt::Display for WeatherApiProvider {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            WeatherApiProvider::OpenWeather => write!(f, "OpenWeather"),
            WeatherApiProvider::GoogleWeather => write!(f, "Google Weather"),
        }
    }
}

/// A struct representing the user's configured location.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LocationConfig {
    pub city: String,
    pub state: String,
    pub country: String,
}

impl Default for LocationConfig {
    fn default() -> Self {
        Self {
            city: "Peoria".to_string(),
            state: "IL".to_string(),
            country: "US".to_string(),
        }
    }
}

/// The main struct for the application's configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    pub weather_provider: WeatherApiProvider,
    pub api_token_encoded: String, // Base64 encoded API token
    pub location: LocationConfig,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            weather_provider: WeatherApiProvider::OpenWeather,
            // Default OpenWeather API key (base64 encoded)
            api_token_encoded: STANDARD.encode("a836db2d273c0b50a2376d6a31750064"),
            location: LocationConfig::default(),
        }
    }
}

impl AppConfig {
    /// Sets the API token, automatically encoding it in base64.
    ///
    /// # Arguments
    ///
    /// * `token` - The API token to set.
    pub fn set_api_token(&mut self, token: &str) {
        self.api_token_encoded = STANDARD.encode(token);
    }

    /// Decodes and returns the API token from its base64 representation.
    ///
    /// # Returns
    ///
    /// A `Result` containing the decoded API token `String` on success, or an error `String` on failure.
    pub fn get_api_token(&self) -> Result<String, String> {
        STANDARD
            .decode(&self.api_token_encoded)
            .map_err(|e| format!("Failed to decode API token: {}", e))
            .and_then(|bytes| {
                String::from_utf8(bytes).map_err(|e| format!("Invalid UTF-8 in API token: {}", e))
            })
    }
}

/// Manages the loading and saving of the application's configuration.
///
/// This struct handles the logic for finding the correct configuration file path
/// and performing file I/O operations for serialization and deserialization.
pub struct ConfigManager {
    config_path: PathBuf,
}

impl ConfigManager {
    /// Creates a new `ConfigManager`.
    ///
    /// This function also ensures that the configuration directory exists.
    ///
    /// # Returns
    ///
    /// A `Result` containing the `ConfigManager` on success, or a boxed error on failure.
    pub fn new() -> Result<Self, Box<dyn std::error::Error>> {
        let config_dir = dirs::config_dir()
            .ok_or("Could not determine config directory")?
            .join("open-weather-wizard");

        // Create config directory if it doesn't exist
        fs::create_dir_all(&config_dir)?;

        let config_path = config_dir.join("config.json");

        Ok(Self { config_path })
    }

    /// Loads the application's configuration from a file.
    ///
    /// If the configuration file does not exist, cannot be read, or contains invalid
    /// JSON, this function logs a warning and returns the default configuration.
    ///
    /// # Returns
    ///
    /// The loaded `AppConfig`.
    pub fn load_config(&self) -> AppConfig {
        match fs::read_to_string(&self.config_path) {
            Ok(contents) => match serde_json::from_str::<AppConfig>(&contents) {
                Ok(config) => {
                    log::info!("Loaded configuration from {:?}", self.config_path);
                    config
                }
                Err(e) => {
                    log::warn!("Failed to parse config file, using defaults: {}", e);
                    AppConfig::default()
                }
            },
            Err(_) => {
                log::info!("Config file not found, using defaults");
                AppConfig::default()
            }
        }
    }

    /// Saves the application's configuration to a file.
    ///
    /// # Arguments
    ///
    /// * `config` - A reference to the `AppConfig` to save.
    ///
    /// # Returns
    ///
    /// A `Result` which is `Ok` on success or a boxed error on failure.
    pub fn save_config(&self, config: &AppConfig) -> Result<(), Box<dyn std::error::Error>> {
        let json = serde_json::to_string_pretty(config)?;
        fs::write(&self.config_path, json)?;
        log::info!("Saved configuration to {:?}", self.config_path);
        Ok(())
    }
}
