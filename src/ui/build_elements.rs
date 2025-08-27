use gtk::Button;
use gtk::Label;
use gtk::prelude::*; // Import necessary traits for GTK widgets
use gtk4::gio::{Menu, MenuItem};
use gtk4::{self as gtk, Spinner}; // For glib::ExitCode

use crate::weather_api::openweather_api;

pub const DEFAULT_WINDOW_WIDTH: i32 = 720;
pub const DEFAULT_WINDOW_HEIGHT: i32 = 480;

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

/// Updates the UI labels with the fetched weather data.
pub fn update_ui_with_weather(
    weather_data: &openweather_api::ApiResponse,
    symbol_label: &Label,
    temp_label: &Label,
    desc_label: &Label,
    humidity_label: &Label,
) {
    if let Some(weather) = weather_data.weather.first() {
        // Update labels with formatted data
        symbol_label.set_text(openweather_api::get_weather_symbol(&weather.main));
        temp_label.set_text(&format!("{:.1}°C", weather_data.main.temp));
        desc_label.set_text(&weather.description);
        humidity_label.set_text(&format!("Humidity: {}%", weather_data.main.humidity));
    }
}
