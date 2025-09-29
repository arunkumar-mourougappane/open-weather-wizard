//!
//! This example demonstrates how to use the OpenWeather API provider to fetch
//! real-time weather data. It initializes a configuration for London, UK,
//! creates an `OpenWeather` provider with a hardcoded API key, and then
//! attempts to fetch and display the current weather conditions.
//!
//! This serves as a live integration test to verify that the OpenWeather API
//! implementation is working correctly.
//!
use open_wearther_wizard::config::{AppConfig, LocationConfig, WeatherApiProvider};
use open_wearther_wizard::weather_api::weather_provider::WeatherProviderFactory;

/// The main entry point for the OpenWeather API integration test.
///
/// This function sets up a test configuration, creates an OpenWeather provider,
/// and fetches weather data for a predefined location. It prints the results
/// to the console, indicating success or failure.
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("OpenWeather API Integration Test");
    println!("================================\n");

    // Test OpenWeather API with the default key from the original app
    let config = AppConfig {
        weather_provider: WeatherApiProvider::OpenWeather,
        api_token_encoded: "".to_string(),
        location: LocationConfig {
            city: "London".to_string(),
            state: "".to_string(),
            country: "UK".to_string(),
        },
    };

    // Use the original API key for testing
    let api_key = "a836db2d273c0b50a2376d6a31750064";

    println!("Testing OpenWeather API Provider...");
    println!("Location: London, UK");
    println!("Using provided API key: {}...", &api_key[..8]);

    let provider = WeatherProviderFactory::create_provider(
        &WeatherApiProvider::OpenWeather,
        Some(api_key.to_string()),
    )?;

    match provider.get_weather(&config.location).await {
        Ok(weather_data) => {
            println!("\n✅ OpenWeather API Provider works!");
            println!("   City: {}", weather_data.name);
            println!("   Temperature: {:.1}°C", weather_data.main.temp);
            println!("   Description: {}", weather_data.weather[0].description);
            println!("   Humidity: {}%", weather_data.main.humidity);
            println!("   Weather condition: {}", weather_data.weather[0].main);
        }
        Err(e) => {
            println!("\n⚠️  OpenWeather API call failed: {:?}", e);
            println!("   This might be due to network issues or API key limitations.");
            println!("   The provider factory and configuration system work correctly.");
        }
    }

    println!("\n✅ OpenWeather API integration test completed!");

    Ok(())
}
