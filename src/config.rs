//! Configuration management for the Weather Wizard application.
//!
//! This module handles loading, saving, and managing application configuration,
//! including weather API settings, location preferences, and API tokens.
//! API tokens are base64 encoded for basic obfuscation when stored.

use base64::{Engine as _, engine::general_purpose::STANDARD};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

/// Supported weather API providers
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

/// Location configuration
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

/// Application configuration
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
    /// Set the API token (automatically base64 encodes it)
    pub fn set_api_token(&mut self, token: &str) {
        self.api_token_encoded = STANDARD.encode(token);
    }

    /// Get the decoded API token
    pub fn get_api_token(&self) -> Result<String, String> {
        STANDARD
            .decode(&self.api_token_encoded)
            .map_err(|e| format!("Failed to decode API token: {}", e))
            .and_then(|bytes| {
                String::from_utf8(bytes).map_err(|e| format!("Invalid UTF-8 in API token: {}", e))
            })
    }
}

/// Configuration manager for loading and saving application settings
pub struct ConfigManager {
    config_path: PathBuf,
}

impl ConfigManager {
    /// Create a new configuration manager
    pub fn new() -> Result<Self, Box<dyn std::error::Error>> {
        let config_dir = dirs::config_dir()
            .ok_or("Could not determine config directory")?
            .join("open-weather-wizard");

        // Create config directory if it doesn't exist
        fs::create_dir_all(&config_dir)?;

        let config_path = config_dir.join("config.json");

        Ok(Self { config_path })
    }

    /// Load configuration from file, or return default if file doesn't exist
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

    /// Save configuration to file
    pub fn save_config(&self, config: &AppConfig) -> Result<(), Box<dyn std::error::Error>> {
        let json = serde_json::to_string_pretty(config)?;
        fs::write(&self.config_path, json)?;
        log::info!("Saved configuration to {:?}", self.config_path);
        Ok(())
    }
}
