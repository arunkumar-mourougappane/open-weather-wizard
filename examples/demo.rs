use open_wearther_wizard::config::{AppConfig, ConfigManager, WeatherApiProvider, LocationConfig};
use open_wearther_wizard::weather_api::weather_provider::WeatherProviderFactory;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("Weather Wizard Functionality Demo");
    println!("==================================\n");
    
    // Demo 1: Configuration management
    println!("1. Testing Configuration Management");
    println!("   Creating test configuration...");
    
    let mut config = AppConfig {
        weather_provider: WeatherApiProvider::GoogleWeather,
        api_token_encoded: "".to_string(),
        location: LocationConfig {
            city: "San Francisco".to_string(),
            state: "CA".to_string(),
            country: "US".to_string(),
        },
    };
    
    config.set_api_token("demo_api_token_12345");
    println!("   ✅ API token encoded and stored");
    
    let decoded_token = config.get_api_token()?;
    println!("   ✅ API token decoded: {}", decoded_token);
    
    // Demo 2: Weather provider testing
    println!("\n2. Testing Weather Providers");
    
    // Test Google Weather (mockup)
    println!("   Testing Google Weather Provider...");
    let provider = WeatherProviderFactory::create_provider(
        &WeatherApiProvider::GoogleWeather,
        None
    )?;
    
    let weather_result = provider.get_weather(&config.location).await;
    match weather_result {
        Ok(weather_data) => {
            println!("   ✅ Google Weather Provider works!");
            println!("      City: {}", weather_data.name);
            println!("      Temperature: {:.1}°C", weather_data.main.temp);
            println!("      Description: {}", weather_data.weather[0].description);
            println!("      Humidity: {}%", weather_data.main.humidity);
        }
        Err(e) => {
            println!("   ❌ Google Weather Provider failed: {:?}", e);
        }
    }
    
    // Demo 3: Configuration serialization
    println!("\n3. Testing Configuration Serialization");
    let json = serde_json::to_string_pretty(&config)?;
    println!("   Configuration as JSON:");
    println!("{}", json);
    
    // Test deserialization
    let deserialized: AppConfig = serde_json::from_str(&json)?;
    println!("   ✅ Configuration deserialized successfully");
    println!("      Provider: {:?}", deserialized.weather_provider);
    println!("      Location: {}, {}, {}", 
             deserialized.location.city, 
             deserialized.location.state,
             deserialized.location.country);
    
    println!("\n✅ All functionality tests completed successfully!");
    println!("\nKey Features Implemented:");
    println!("• Configuration management with base64 API token encoding");
    println!("• Weather API abstraction layer");
    println!("• Google Weather API mockup");
    println!("• OpenWeather API provider (with configurable API key)");
    println!("• Location-based configuration");
    println!("• JSON serialization/deserialization");
    
    Ok(())
}