//! Preferences window for the Weather Wizard application.
//!
//! This module provides a preferences window that allows users to configure:
//! - Weather API provider selection
//! - API tokens for weather services
//! - Location settings (city, state, country)

use gtk::prelude::*;
use gtk::{ApplicationWindow, Box, Button, ComboBoxText, Entry, Grid, HeaderBar, Label, Window};
use std::sync::{Arc, Mutex};

use crate::config::{AppConfig, ConfigManager, WeatherApiProvider};

/// Creates and shows the preferences window
pub fn show_preferences_window(parent: &ApplicationWindow, config: Arc<Mutex<AppConfig>>) {
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
            }
        }

        window_clone.close();
    });

    window.present();
}
