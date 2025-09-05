//! Configuration management for Weather Wizard
//!
//! This module handles loading and saving application configuration,
//! including API keys, preferred weather service, and default location.

use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

/// Supported weather API providers
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub enum WeatherProvider {
    #[default]
    OpenWeather,
    GoogleWeather,
}

/// Application configuration structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    /// Selected weather API provider
    pub weather_provider: WeatherProvider,
    /// OpenWeather API key
    pub openweather_api_key: String,
    /// Google Weather API key
    pub google_weather_api_key: String,
    /// Default location settings
    pub default_location: LocationConfig,
}

/// Default location configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LocationConfig {
    pub city: String,
    pub state: String,
    pub country: String,
}

impl Default for Config {
    fn default() -> Self {
        Config {
            weather_provider: WeatherProvider::default(),
            openweather_api_key: "a836db2d273c0b50a2376d6a31750064".to_string(), // Default key from existing code
            google_weather_api_key: String::new(),
            default_location: LocationConfig {
                city: "Peoria".to_string(),
                state: "IL".to_string(),
                country: "US".to_string(),
            },
        }
    }
}

impl Config {
    /// Get the path to the configuration file
    fn config_file_path() -> Result<PathBuf, Box<dyn std::error::Error>> {
        let mut path = dirs::home_dir().ok_or("Could not find home directory")?;
        path.push(".config");
        path.push("weather-wizard");

        // Create the directory if it doesn't exist
        if !path.exists() {
            fs::create_dir_all(&path)?;
        }

        path.push("config.json");
        Ok(path)
    }

    /// Load configuration from file, or create default if file doesn't exist
    pub fn load() -> Result<Config, Box<dyn std::error::Error>> {
        let config_path = Self::config_file_path()?;

        if config_path.exists() {
            let config_str = fs::read_to_string(&config_path)?;
            let config: Config = serde_json::from_str(&config_str)?;
            log::info!("Loaded configuration from {:?}", config_path);
            Ok(config)
        } else {
            log::info!("Configuration file not found, using defaults");
            let config = Config::default();
            // Save the default configuration
            config.save()?;
            Ok(config)
        }
    }

    /// Save configuration to file
    pub fn save(&self) -> Result<(), Box<dyn std::error::Error>> {
        let config_path = Self::config_file_path()?;
        let config_str = serde_json::to_string_pretty(self)?;
        fs::write(&config_path, config_str)?;
        log::info!("Saved configuration to {:?}", config_path);
        Ok(())
    }
}
