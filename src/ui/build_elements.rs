use glib::Bytes;
use gtk::Button;
use gtk::Label;
use gtk::gdk_pixbuf::Pixbuf;
use gtk::gio;
use gtk::gio::MemoryInputStream;
use gtk::prelude::*;
// Import necessary traits for GTK widgets
use gtk::gio::{Menu, MenuItem};
use gtk::{Image, Spinner}; // For glib::ExitCode and Image widget

use crate::weather_api::openweather_api;

pub const DEFAULT_WINDOW_WIDTH: i32 = 720;
pub const DEFAULT_WINDOW_HEIGHT: i32 = 480;

use rust_embed::RustEmbed;

#[derive(RustEmbed)]
#[folder = "assets/"]
struct WeatherIconsAsset;

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

pub fn build_main_menu() -> Menu {
    let file_menu = Menu::new();
    let preferences_item = MenuItem::new(Some("Preferences"), None);
    let exit_item = MenuItem::new(Some("Exit"), None);
    file_menu.append_item(&preferences_item);
    file_menu.append_item(&exit_item);

    let about_help = Menu::new();
    let about_menu = MenuItem::new(Some("About"), None);
    let help_menu = MenuItem::new(Some("Help"), None);
    about_help.append_item(&about_menu);
    about_help.append_item(&help_menu);

    // Create root menu and add submenus
    let root_menu = Menu::new();
    root_menu.append_submenu(Some("File"), &file_menu);
    root_menu.append_submenu(Some("Help"), &about_help);

    root_menu
}

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

pub fn build_entry(label: String) -> gtk::Entry {
    gtk::Entry::builder()
        .placeholder_text(label.as_str())
        .margin_top(12)
        .margin_bottom(12)
        .margin_start(12)
        .margin_end(12)
        .build()
}

fn get_weather_symbol(weather: openweather_api::WeatherSymbol) -> &'static str {
    match weather {
        openweather_api::WeatherSymbol::Clear => "static/day.svg",
        openweather_api::WeatherSymbol::Clouds => "animated/cloudy-day-1.svg",
        openweather_api::WeatherSymbol::Rain => "animated/rainy-6.svg",
        openweather_api::WeatherSymbol::Drizzle => "animated/rainy-2.svg",
        openweather_api::WeatherSymbol::Thunderstorm => "animated/thunder.svg",
        openweather_api::WeatherSymbol::Snow => "animated/snowy-3.svg",
        openweather_api::WeatherSymbol::Mist => "static/mist.png",
        _ => "animated/weather.svg",
    }
}

/// Updates the UI labels with the fetched weather data.
pub fn update_ui_with_weather(
    weather_data: &openweather_api::ApiResponse,
    symbol_image: &Image,
    temp_label: &Label,
    desc_label: &Label,
    humidity_label: &Label,
) {
    if let Some(weather) = weather_data.weather.first() {
        // Get the data like before
        let embedded_file = WeatherIconsAsset::get(get_weather_symbol(
            openweather_api::get_weather_symbol(&weather.main),
        ))
        .unwrap();
        let svg_data: std::borrow::Cow<'static, [u8]> = embedded_file.data;
        let bytes = Bytes::from_owned(svg_data.clone());

        // Load bytes into a stream, then into a Pixbuf
        let stream: MemoryInputStream = MemoryInputStream::from_bytes(&bytes);
        let pixbuf =
            Pixbuf::from_stream_at_scale(&stream, 256, 256, true, None::<&gio::Cancellable>)
                .expect("Failed to create Pixbuf from SVG stream.");
        // Update labels with formatted data
        symbol_image.set_from_pixbuf(Some(&pixbuf));
        temp_label.set_text(&format!("{:.1}°C", weather_data.main.temp));
        desc_label.set_text(&weather.description);
        humidity_label.set_text(&format!("Humidity: {}%", weather_data.main.humidity));
    }
}
