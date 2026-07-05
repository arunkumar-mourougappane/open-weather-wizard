//! # Headless CLI Mode
//!
//! `--headless` fetches weather once and prints it to stdout, without ever
//! opening an `iced` window -- useful for scripting, status-bar widgets, or
//! a genuinely headless machine (see issue #40). Deliberately a *bin-only*
//! module (declared via `mod cli;` in `main.rs`, not re-exported from
//! `lib.rs`): CLI parsing is a concern of this specific executable, not the
//! library crate other code links against.
//!
//! This never touches `app::run()`/`iced::daemon`, so it's free to spin up
//! its own `tokio::runtime::Runtime` -- the "don't nest a runtime around
//! `app::run()`" constraint noted in `main.rs`'s doc comment only applies to
//! the GUI path.
//!
//! A one-shot fetch-and-print, not a daemon/watch mode -- recurring headless
//! refresh is explicitly out of scope (see issue #40).

use clap::Parser;

use crate::config::{ConfigManager, LocationConfig, WeatherApiProvider};
use crate::ui::temperature::{
    celsius_to_display, compass_direction, distance_to_display, distance_unit, format_local_time,
    speed_to_display, speed_unit, unit_symbol,
};
use crate::weather_api::forecast::ForecastResponse;
use crate::weather_api::openweather_api::ApiResponse;
use crate::weather_api::weather_provider::WeatherProviderFactory;

/// Overrides whatever token is in the OS keychain -- the keychain (via the
/// `keyring` crate's Secret Service backend on Linux) isn't guaranteed to be
/// available on a genuinely headless machine with no D-Bus session, so
/// scripted/server use needs a way to supply a token that doesn't depend on
/// it.
const TOKEN_ENV_VAR: &str = "OPEN_WEATHER_WIZARD_API_TOKEN";

#[derive(Parser, Debug)]
#[command(
    name = "open-weather-wizard",
    about = "A desktop weather app. Pass --headless to fetch and print weather without opening the GUI."
)]
pub struct Cli {
    /// Fetch weather once and print to stdout, without opening the GUI.
    #[arg(long)]
    pub headless: bool,

    /// Output machine-readable JSON instead of human-readable text.
    #[arg(long, requires = "headless")]
    pub json: bool,

    /// Override the configured city for this one query.
    #[arg(long, requires = "headless")]
    pub city: Option<String>,

    /// Override the configured state/province for this one query.
    #[arg(long, requires = "headless")]
    pub state: Option<String>,

    /// Override the configured country for this one query.
    #[arg(long, requires = "headless")]
    pub country: Option<String>,

    /// Override the configured weather provider for this one query:
    /// "openweather" or "google".
    #[arg(long, requires = "headless")]
    pub provider: Option<String>,
}

fn parse_provider(value: &str) -> Result<WeatherApiProvider, String> {
    match value.to_ascii_lowercase().as_str() {
        "openweather" | "open-weather" | "owm" => Ok(WeatherApiProvider::OpenWeather),
        "google" | "google-weather" | "googleweather" => Ok(WeatherApiProvider::GoogleWeather),
        other => Err(format!(
            "Unknown provider '{other}' -- expected \"openweather\" or \"google\""
        )),
    }
}

/// Runs headless mode and exits the process directly -- `main()`'s fixed
/// `iced::Result` return type has no good way to represent a CLI success/
/// failure/exit-code, and this path never returns control to it anyway.
pub fn run(cli: &Cli) -> ! {
    let exit_code = match run_inner(cli) {
        Ok(()) => 0,
        Err(message) => {
            eprintln!("Error: {message}");
            1
        }
    };
    std::process::exit(exit_code);
}

fn run_inner(cli: &Cli) -> Result<(), String> {
    let config_manager =
        ConfigManager::new().map_err(|e| format!("Could not access config directory: {e}"))?;

    if !config_manager.config_exists() {
        return Err(
            "No configuration found yet. Run the app normally once to complete first-run setup \
             (provider, API key, Home location) before using --headless."
                .to_string(),
        );
    }

    let config = config_manager.load_config();

    let provider_type = match &cli.provider {
        Some(value) => parse_provider(value)?,
        None => config.weather_provider.clone(),
    };

    let location = LocationConfig {
        city: cli
            .city
            .clone()
            .unwrap_or_else(|| config.location.city.clone()),
        state: cli
            .state
            .clone()
            .unwrap_or_else(|| config.location.state.clone()),
        country: cli
            .country
            .clone()
            .unwrap_or_else(|| config.location.country.clone()),
    };

    let token = std::env::var(TOKEN_ENV_VAR)
        .ok()
        .filter(|t| !t.is_empty())
        .or_else(|| config.get_api_token().ok().filter(|t| !t.is_empty()));

    let provider = WeatherProviderFactory::create_provider(&provider_type, token).map_err(|e| {
        format!(
            "{e} (set {TOKEN_ENV_VAR} or configure a token via the GUI's Preferences window first)"
        )
    })?;

    let runtime = tokio::runtime::Runtime::new()
        .map_err(|e| format!("Failed to start async runtime: {e}"))?;
    let (weather_result, forecast_result) = runtime.block_on(async {
        let weather = provider.get_weather(&location).await;
        let forecast = provider.get_forecast(&location).await;
        (weather, forecast)
    });

    let weather = weather_result.map_err(|e| format!("Failed to fetch weather: {e:?}"))?;
    // A forecast failure shouldn't sink the whole command -- current
    // conditions are still useful on their own, same philosophy as the GUI's
    // ForecastStatus being independent of WeatherStatus (see src/app.rs).
    let forecast = forecast_result.ok();

    if cli.json {
        print_json(&weather, forecast.as_ref())
    } else {
        print_text(&weather, forecast.as_ref(), config.use_fahrenheit);
        Ok(())
    }
}

#[derive(serde::Serialize)]
struct HeadlessOutput<'a> {
    weather: &'a ApiResponse,
    forecast: Option<&'a ForecastResponse>,
}

fn print_json(weather: &ApiResponse, forecast: Option<&ForecastResponse>) -> Result<(), String> {
    let output = HeadlessOutput { weather, forecast };
    let json = serde_json::to_string_pretty(&output)
        .map_err(|e| format!("Failed to serialize output as JSON: {e}"))?;
    println!("{json}");
    Ok(())
}

fn print_text(weather: &ApiResponse, forecast: Option<&ForecastResponse>, use_fahrenheit: bool) {
    let unit = unit_symbol(use_fahrenheit);
    let temp = celsius_to_display(weather.main.temp, use_fahrenheit);
    let feels_like = celsius_to_display(weather.main.feels_like, use_fahrenheit);
    let wind_speed = speed_to_display(weather.wind.speed, use_fahrenheit);
    let wind_unit = speed_unit(use_fahrenheit);
    let compass = compass_direction(weather.wind.deg);
    let visibility = distance_to_display(weather.visibility as f64, use_fahrenheit);
    let visibility_unit = distance_unit(use_fahrenheit);
    let sunrise = format_local_time(weather.sys.sunrise, weather.timezone);
    let sunset = format_local_time(weather.sys.sunset, weather.timezone);

    println!("{}", weather.name);
    if let Some(condition) = weather.weather.first() {
        println!("{}", condition.description);
    }
    println!("Temperature:  {temp:.1}{unit} (feels like {feels_like:.0}{unit})");
    println!("Humidity:     {}%", weather.main.humidity);
    println!("Wind:         {wind_speed:.0} {wind_unit} {compass}");
    println!("Pressure:     {} hPa", weather.main.pressure);
    println!("Visibility:   {visibility:.1} {visibility_unit}");
    println!("Sunrise:      {sunrise}");
    println!("Sunset:       {sunset}");

    if let Some(forecast) = forecast
        && !forecast.days.is_empty()
    {
        println!("\nForecast:");
        for day in &forecast.days {
            let hi = celsius_to_display(day.temp_max, use_fahrenheit);
            let lo = celsius_to_display(day.temp_min, use_fahrenheit);
            println!(
                "  {}: {hi:.0}{unit} / {lo:.0}{unit} -- {}",
                day.date, day.description
            );
        }
    }
}
