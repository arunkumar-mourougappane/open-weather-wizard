//! # Weather API Abstraction Layer
//!
//! This module provides a unified interface for interacting with various weather
//! data providers. It defines a common `WeatherProvider` trait and includes
//! implementations for different services.
//!
//! ## Sub-modules
//! - `weather_provider`: Defines the core `WeatherProvider` trait and a factory for creating provider instances.
//! - `openweather_api`: Contains the implementation for the real OpenWeatherMap API.
//! - `google_weather_api`: Contains a mock implementation for a "Google Weather" API, used for testing and demonstration.
pub mod google_weather_api;
pub mod openweather_api;
pub mod weather_provider;
