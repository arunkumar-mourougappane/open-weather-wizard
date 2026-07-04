//!
//! This example demonstrates how to use the Google Weather API provider to
//! fetch real-time weather data. It initializes a configuration for Peoria,
//! IL, creates a `GoogleWeather` provider using an API key read from the
//! `GOOGLE_WEATHER_API_KEY` environment variable, and then attempts to fetch
//! and display the current weather conditions and forecast.
//!
//! This serves as a live integration test to verify that the Google Weather
//! API implementation is working correctly. Run it with:
//!
//! ```sh
//! GOOGLE_WEATHER_API_KEY=your-key-here cargo run --example google_weather_test
//! ```
use open_weather_wizard::config::{LocationConfig, WeatherApiProvider};
use open_weather_wizard::weather_api::weather_provider::WeatherProviderFactory;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("Google Weather API Integration Test");
    println!("====================================\n");

    let location = LocationConfig {
        city: "Peoria".to_string(),
        state: "IL".to_string(),
        country: "US".to_string(),
    };

    // Never hardcode a real API key in source -- read it from the
    // environment instead. This example is a live integration test, run
    // manually by a developer who has their own key, not something CI
    // executes automatically.
    let api_key = std::env::var("GOOGLE_WEATHER_API_KEY").map_err(|_| {
        "Set the GOOGLE_WEATHER_API_KEY environment variable to your own Google Cloud API key \
         (with the Weather API enabled) before running this example \
         (e.g. `GOOGLE_WEATHER_API_KEY=... cargo run --example google_weather_test`)"
    })?;

    println!("Testing Google Weather API Provider...");
    println!("Location: Peoria, IL, US");
    println!(
        "Using provided API key: {}...",
        &api_key[..8.min(api_key.len())]
    );

    let provider =
        WeatherProviderFactory::create_provider(&WeatherApiProvider::GoogleWeather, Some(api_key))?;

    match provider.get_weather(&location).await {
        Ok(weather_data) => {
            println!("\n✅ Google Weather API Provider works!");
            println!("   City: {}", weather_data.name);
            println!("   Temperature: {:.1}°C", weather_data.main.temp);
            println!("   Description: {}", weather_data.weather[0].description);
            println!("   Humidity: {}%", weather_data.main.humidity);
            println!("   Weather condition: {}", weather_data.weather[0].main);
            println!(
                "   Sunrise/sunset (unix): {} / {}",
                weather_data.sys.sunrise, weather_data.sys.sunset
            );
            println!("   Timezone offset (s): {}", weather_data.timezone);
        }
        Err(e) => {
            println!("\n⚠️  Google Weather API call failed: {:?}", e);
            println!("   This might be due to network issues or API key limitations.");
        }
    }

    match provider.get_forecast(&location).await {
        Ok(forecast) => {
            println!("\n✅ Google Weather forecast fetched!");
            for day in &forecast.days {
                println!(
                    "   {}: {:.0}°C / {:.0}°C -- {}",
                    day.date, day.temp_max, day.temp_min, day.description
                );
            }
        }
        Err(e) => {
            println!("\n⚠️  Google Weather forecast call failed: {:?}", e);
        }
    }

    println!("\n✅ Google Weather API integration test completed!");

    Ok(())
}
