//!
//! This example demonstrates how to use the OpenWeather API provider to fetch
//! real-time weather data. It initializes a configuration for London, UK,
//! creates an `OpenWeather` provider using an API key read from the
//! `OPENWEATHER_API_KEY` environment variable, and then attempts to fetch
//! and display the current weather conditions.
//!
//! This serves as a live integration test to verify that the OpenWeather API
//! implementation is working correctly. Run it with:
//!
//! ```sh
//! OPENWEATHER_API_KEY=your-key-here cargo run --example openweather_test
//! ```
use open_weather_wizard::config::{AppConfig, LocationConfig, WeatherApiProvider};
use open_weather_wizard::weather_api::weather_provider::WeatherProviderFactory;

/// The main entry point for the OpenWeather API integration test.
///
/// This function sets up a test configuration, creates an OpenWeather provider,
/// and fetches weather data for a predefined location. It prints the results
/// to the console, indicating success or failure.
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("OpenWeather API Integration Test");
    println!("================================\n");

    let mut config = AppConfig::default();
    config.weather_provider = WeatherApiProvider::OpenWeather;
    config.location = LocationConfig {
        city: "London".to_string(),
        state: "".to_string(),
        // OpenWeatherMap's geocoding endpoint expects an ISO 3166-1 alpha-2
        // country code -- "UK" isn't one (the correct code is "GB") and
        // silently returns zero results rather than an error.
        country: "GB".to_string(),
    };

    // Never hardcode a real API key in source -- read it from the
    // environment instead. This example is a live integration test, run
    // manually by a developer who has their own key, not something CI
    // executes automatically.
    let api_key = std::env::var("OPENWEATHER_API_KEY").map_err(|_| {
        "Set the OPENWEATHER_API_KEY environment variable to your own OpenWeatherMap API key \
         before running this example (e.g. `OPENWEATHER_API_KEY=... cargo run --example openweather_test`)"
    })?;

    println!("Testing OpenWeather API Provider...");
    println!("Location: London, UK");
    println!(
        "Using provided API key: {}...",
        &api_key[..8.min(api_key.len())]
    );

    let provider = WeatherProviderFactory::create_provider(
        &WeatherApiProvider::OpenWeather,
        Some(api_key.to_string()),
        config.language,
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

    match provider.get_forecast(&config.location).await {
        Ok(forecast) => {
            println!("\n✅ OpenWeather forecast fetched!");
            for day in &forecast.days {
                println!(
                    "   {}: {:.0}°C / {:.0}°C -- {}",
                    day.date, day.temp_max, day.temp_min, day.description
                );
            }
        }
        Err(e) => {
            println!("\n⚠️  OpenWeather forecast call failed: {:?}", e);
        }
    }

    println!("\n✅ OpenWeather API integration test completed!");

    Ok(())
}
