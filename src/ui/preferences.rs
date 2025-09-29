//! # Preferences Window Module
//!
//! This module is responsible for creating and managing the application's
//! preferences window. The window is a modal dialog that allows users to
//! configure various settings, which are then persisted to a configuration file.
//!
//! Key features of the preferences window include:
//! - **API Provider Selection**: A dropdown to choose between different weather
//!   services (e.g., OpenWeather, Google Weather).
//! - **API Token Management**: A secure entry field for the user's API token.
//! - **Location Configuration**: Fields for setting the default city, state, and country.
//!
//! The window is built using GTK widgets and interacts with the main application's
//! shared configuration state (`Arc<Mutex<AppConfig>>`). When settings are saved,
//! it updates this shared state, writes the configuration to disk using `ConfigManager`,
//! and triggers a callback to notify the main UI to refresh its data.

use gtk::prelude::*;
use gtk::{ApplicationWindow, Box, Button, ComboBoxText, Entry, Grid, HeaderBar, Label, Window};
use std::sync::{Arc, Mutex};

use crate::config::{AppConfig, ConfigManager, WeatherApiProvider};

/// Creates and displays the modal preferences window.
///
/// This function constructs the entire preferences UI, including labels, text entries,
/// and dropdowns for all configurable options. It populates the fields with the
/// current values from the provided `AppConfig`. It connects signal handlers for the
/// "Save" and "Cancel" buttons.
///
/// When the "Save" button is clicked, it reads the new values from the UI widgets,
/// updates the shared `AppConfig` state, persists the changes to the configuration
/// file via `ConfigManager`, and finally executes the `on_save` closure to trigger
/// actions in the main UI, such as re-fetching weather data.
///
/// # Arguments
///
/// * `parent` - The parent `ApplicationWindow` to which this modal dialog is transient.
/// * `config` - A thread-safe, shared pointer to the application's `AppConfig`.
/// * `on_save` - A closure that is executed after the configuration is successfully saved.
///   This is typically used to refresh the main application view.
///
/// # Type Parameters
///
/// * `F` - The type of the `on_save` closure, which must be a `Fn()` with a `'static` lifetime.
pub fn show_preferences_window<F>(
    parent: &ApplicationWindow,
    config: Arc<Mutex<AppConfig>>,
    on_save: F,
) where
    F: Fn() + 'static,
{
    let window = Window::builder()
        .title("Preferences")
        .default_width(500)
        .default_height(400)
        .modal(true)
        .transient_for(parent)
        .build();

    // Create header bar
    let header_bar = HeaderBar::builder()
        .title_widget(&Label::new(Some("Preferences")))
        .show_title_buttons(true)
        .build();
    window.set_titlebar(Some(&header_bar));

    // Create main content
    let main_box = Box::builder()
        .orientation(gtk::Orientation::Vertical)
        .spacing(12)
        .margin_top(20)
        .margin_bottom(20)
        .margin_start(20)
        .margin_end(20)
        .build();

    // Create grid for form layout
    let grid = Grid::builder().row_spacing(12).column_spacing(12).build();

    // Weather Provider section
    let provider_label = Label::builder()
        .label("Weather Provider:")
        .halign(gtk::Align::Start)
        .build();
    grid.attach(&provider_label, 0, 0, 1, 1);

    let provider_combo = ComboBoxText::new();
    provider_combo.append_text("OpenWeather");
    provider_combo.append_text("Google Weather");

    // Set current provider
    {
        let current_config = config.lock().expect("Failed to lock config");
        match current_config.weather_provider {
            WeatherApiProvider::OpenWeather => provider_combo.set_active(Some(0)),
            WeatherApiProvider::GoogleWeather => provider_combo.set_active(Some(1)),
        }
    }
    grid.attach(&provider_combo, 1, 0, 1, 1);

    // API Token section
    let token_label = Label::builder()
        .label("API Token:")
        .halign(gtk::Align::Start)
        .build();
    grid.attach(&token_label, 0, 1, 1, 1);

    let token_entry = Entry::builder()
        .placeholder_text("Enter your API token")
        .visibility(false) // Hide token for security
        .build();

    // Set current token (if available)
    {
        let current_config = config.lock().expect("Failed to lock config");
        if let Ok(token) = current_config.get_api_token() {
            token_entry.set_text(&token);
        }
    }
    grid.attach(&token_entry, 1, 1, 1, 1);

    // Location section header
    let location_header = Label::builder()
        .label("<b>Default Location</b>")
        .use_markup(true)
        .halign(gtk::Align::Start)
        .margin_top(12)
        .build();
    grid.attach(&location_header, 0, 2, 2, 1);

    // City
    let city_label = Label::builder()
        .label("City:")
        .halign(gtk::Align::Start)
        .build();
    grid.attach(&city_label, 0, 3, 1, 1);

    let city_entry = Entry::builder().placeholder_text("Enter city name").build();
    {
        let current_config = config.lock().expect("Failed to lock config");
        city_entry.set_text(&current_config.location.city);
    }
    grid.attach(&city_entry, 1, 3, 1, 1);

    // State
    let state_label = Label::builder()
        .label("State/Province:")
        .halign(gtk::Align::Start)
        .build();
    grid.attach(&state_label, 0, 4, 1, 1);

    let state_entry = Entry::builder()
        .placeholder_text("Enter state or province")
        .build();
    {
        let current_config = config.lock().expect("Failed to lock config");
        state_entry.set_text(&current_config.location.state);
    }
    grid.attach(&state_entry, 1, 4, 1, 1);

    // Country
    let country_label = Label::builder()
        .label("Country:")
        .halign(gtk::Align::Start)
        .build();
    grid.attach(&country_label, 0, 5, 1, 1);

    let country_entry = Entry::builder()
        .placeholder_text("Enter country code (e.g., US, CA)")
        .build();
    {
        let current_config = config.lock().expect("Failed to lock config");
        country_entry.set_text(&current_config.location.country);
    }
    grid.attach(&country_entry, 1, 5, 1, 1);

    main_box.append(&grid);

    // Button box
    let button_box = Box::builder()
        .orientation(gtk::Orientation::Horizontal)
        .spacing(6)
        .halign(gtk::Align::End)
        .margin_top(20)
        .build();

    let cancel_button = Button::builder().label("Cancel").build();

    let save_button = Button::builder()
        .label("Save")
        .css_classes(vec!["suggested-action"])
        .build();

    button_box.append(&cancel_button);
    button_box.append(&save_button);
    main_box.append(&button_box);

    window.set_child(Some(&main_box));

    // Connect signals
    let window_clone = window.clone();
    cancel_button.connect_clicked(move |_| {
        window_clone.close();
    });

    let window_clone = window.clone();
    save_button.connect_clicked(move |_| {
        // Save configuration
        let mut current_config = config.lock().expect("Failed to lock config");

        // Update provider
        if let Some(active) = provider_combo.active() {
            current_config.weather_provider = match active {
                0 => WeatherApiProvider::OpenWeather,
                1 => WeatherApiProvider::GoogleWeather,
                _ => WeatherApiProvider::OpenWeather,
            };
        }

        // Update API token
        let token_text = token_entry.text();
        if !token_text.is_empty() {
            current_config.set_api_token(&token_text);
        }

        // Update location
        current_config.location.city = city_entry.text().to_string();
        current_config.location.state = state_entry.text().to_string();
        current_config.location.country = country_entry.text().to_string();

        // Save to file
        if let Ok(config_manager) = ConfigManager::new() {
            if let Err(e) = config_manager.save_config(&current_config) {
                log::error!("Failed to save configuration: {}", e);
                // TODO: Show error dialog
            } else {
                log::info!("Configuration saved successfully");
                on_save();
            }
        }

        window_clone.close();
    });

    window.present();
}
