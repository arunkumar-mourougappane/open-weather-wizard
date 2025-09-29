//! # UI Element Builders
//!
//! This module centralizes the creation and management of various GTK UI elements
//! used throughout the Weather Wizard application. It provides helper functions
//! for constructing common widgets like spinners and menus, and also handles the
//! logic for updating the UI with new weather data.
//!
//! A key responsibility of this module is managing the weather icons. It uses the
//! `rust-embed` crate to embed SVG icon assets directly into the binary, maps
//! weather conditions to specific icon files, and loads them as `Pixbuf`s for
//! display in the UI.

use glib::Bytes;
use gtk::Button;
use gtk::gdk_pixbuf::Pixbuf;
use gtk::gio;
use gtk::gio::MemoryInputStream;
use gtk::prelude::*;
// Import necessary traits for GTK widgets
use super::UIWidgets;
use gtk::Spinner;
use gtk::gio::{Menu, MenuItem}; // For glib::ExitCode and Image widget
use rust_embed::RustEmbed;

use crate::weather_api::openweather_api;

pub const DEFAULT_WINDOW_WIDTH: i32 = 720;
pub const DEFAULT_WINDOW_HEIGHT: i32 = 480;

/// Embeds the contents of the `assets/` directory into the application binary.
///
/// This allows weather icons to be bundled with the application, removing the need for separate installation.
#[derive(RustEmbed)]
#[folder = "assets/"]
struct WeatherIconsAsset;

/// Builds a GTK spinner with a specified diameter.
///
/// # Arguments
///
/// * `diameter` - The diameter of the spinner.
///
/// # Returns
///
/// A `gtk::Spinner` widget.
pub fn build_spinner(diameter: i32) -> gtk::Spinner {
    let spinner = Spinner::builder()
        .spinning(false) // Initially not spinning
        .visible(false) // Initially hidden
        .margin_top(10)
        .margin_bottom(10)
        .build();

    spinner.set_size_request(diameter, diameter);

    spinner
}

/// Builds the main menu for the application.
///
/// This creates a standard menu structure with "File" and "Help" submenus,
/// containing actions like "Preferences", "Exit", and "About".
///
/// # Returns
///
/// A `gtk::gio::Menu` widget.
pub fn build_main_menu() -> Menu {
    let file_menu = Menu::new();
    let preferences_item = MenuItem::new(Some("Preferences"), Some("app.preferences"));
    let exit_item = MenuItem::new(Some("Exit"), Some("app.quit"));
    file_menu.append_item(&preferences_item);
    file_menu.append_item(&exit_item);

    let about_help = Menu::new();
    let about_menu = MenuItem::new(Some("About"), Some("app.about"));
    let help_menu = MenuItem::new(Some("Help"), Some("app.help"));
    about_help.append_item(&about_menu);
    about_help.append_item(&help_menu);

    // Create root menu and add submenus
    let root_menu = Menu::new();
    root_menu.append_submenu(Some("File"), &file_menu);
    root_menu.append_submenu(Some("Help"), &about_help);

    root_menu
}

/// Builds a GTK button with a specified label.
///
/// # Arguments
///
/// * `label` - The text to display on the button.
///
/// # Returns
///
/// A `gtk::Button` widget.
#[allow(dead_code)]
pub fn build_button(label: String) -> Button {
    // Create a button with a label

    Button::builder()
        .label(label.as_str())
        .margin_top(12)
        .margin_bottom(12)
        .margin_start(12)
        .margin_end(12)
        .build()
}

/// Maps a `WeatherSymbol` enum to its corresponding SVG icon file path.
///
/// This function determines which animated icon to display based on the weather
/// condition received from the API. It provides a fallback for unhandled conditions.
///
/// # Arguments
///
/// * `weather` - A `openweather_api::WeatherSymbol` representing the current condition.
///
/// # Returns
///
/// A static string slice representing the path to the icon file within the embedded assets.
pub fn get_weather_symbol(weather: openweather_api::WeatherSymbol) -> &'static str {
    match weather {
        openweather_api::WeatherSymbol::Clear => "animated/clear-day.svg",
        openweather_api::WeatherSymbol::Clouds => "animated/cloudy-2-day.svg",
        openweather_api::WeatherSymbol::Rain => "animated/rainy-3.svg",
        openweather_api::WeatherSymbol::Drizzle => "animated/rainy-1.svg",
        openweather_api::WeatherSymbol::Thunderstorm => "animated/thunderstorms.svg",
        openweather_api::WeatherSymbol::Snow => "animated/snowy-2.svg",
        openweather_api::WeatherSymbol::Mist => "animated/fog.svg",
        openweather_api::WeatherSymbol::Smoke => "animated/fog.svg",
        openweather_api::WeatherSymbol::Haze => "animated/haze.svg",
        openweather_api::WeatherSymbol::Dust => "animated/dust.svg",
        openweather_api::WeatherSymbol::Fog => "animated/fog.svg",
        openweather_api::WeatherSymbol::Sand => "animated/dust.svg",
        openweather_api::WeatherSymbol::Ash => "animated/dust.svg",
        openweather_api::WeatherSymbol::Squall => "animated/wind.svg",
        openweather_api::WeatherSymbol::Tornado => "animated/tornado.svg",
        _ => "animated/cloudy.svg",
    }
}

/// Updates the UI with new weather data, including text and the weather icon.
///
/// This function takes the API response, loads the appropriate weather icon from
/// the embedded assets, converts it to a `Pixbuf`, and updates all relevant UI widgets.
///
/// # Arguments
///
/// * `weather_data` - A reference to the `openweather_api::ApiResponse` containing the weather data.
/// * `widgets` - A reference to the `UIWidgets` struct containing the UI elements to update.
///
/// # Errors
///
/// Returns an `anyhow::Error` if the icon asset cannot be found or if the SVG cannot be loaded into a `Pixbuf`.
pub fn update_ui_with_weather(
    weather_data: &openweather_api::ApiResponse,
    widgets: &UIWidgets,
) -> Result<(), anyhow::Error> {
    if let Some(weather) = weather_data.weather.first() {
        // Get the data like before
        let embedded_file = WeatherIconsAsset::get(get_weather_symbol(
            openweather_api::get_weather_symbol(&weather.main),
        ))
        .ok_or_else(|| anyhow::anyhow!("Asset not found"))?;
        // Get the SVG data
        let svg_data = embedded_file.data.as_ref();
        let bytes = Bytes::from(svg_data);

        // Load bytes into a stream, then into a Pixbuf
        let stream: MemoryInputStream = MemoryInputStream::from_bytes(&bytes);
        let pixbuf =
            Pixbuf::from_stream_at_scale(&stream, 256, 256, true, None::<&gio::Cancellable>)?;
        // Update labels with formatted data
        widgets.weather_symbol_image.set_from_pixbuf(Some(&pixbuf));
        widgets
            .temp_label
            .set_text(&format!("{:.1}°C", weather_data.main.temp));
        widgets.description_label.set_text(&weather.description);
        widgets
            .humidity_label
            .set_text(&format!("Humidity: {}%", weather_data.main.humidity));
    }
    Ok(())
}
