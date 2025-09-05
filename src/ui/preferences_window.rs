//! Preferences window for configuring Weather Wizard settings
//!
//! This module provides a GTK4 window for editing application configuration,
//! including API selection, API keys, and default location settings.

use gtk::prelude::*;
use gtk::{ApplicationWindow, Box, Button, ComboBoxText, Entry, Grid, Label, Window};
use std::cell::RefCell;
use std::rc::Rc;

use crate::config::{Config, WeatherProvider};

/// Create and show the preferences window
pub fn show_preferences_window(parent: &ApplicationWindow, current_config: Rc<RefCell<Config>>) {
    let window = Window::builder()
        .title("Weather Wizard Preferences")
        .transient_for(parent)
        .modal(true)
        .default_width(500)
        .default_height(400)
        .build();

    // Main container
    let main_box = Box::builder()
        .orientation(gtk::Orientation::Vertical)
        .spacing(12)
        .margin_top(12)
        .margin_bottom(12)
        .margin_start(12)
        .margin_end(12)
        .build();

    // Weather API Selection
    let api_section = create_api_selection_section(&current_config);
    main_box.append(&api_section);

    // API Keys Section
    let keys_section = create_api_keys_section(&current_config);
    main_box.append(&keys_section);

    // Default Location Section
    let location_section = create_location_section(&current_config);
    main_box.append(&location_section);

    // Buttons
    let button_box = Box::builder()
        .orientation(gtk::Orientation::Horizontal)
        .spacing(6)
        .halign(gtk::Align::End)
        .build();

    let cancel_button = Button::builder().label("Cancel").build();

    let save_button = Button::builder().label("Save").build();

    button_box.append(&cancel_button);
    button_box.append(&save_button);
    main_box.append(&button_box);

    // Button actions
    let window_clone = window.clone();
    cancel_button.connect_clicked(move |_| {
        window_clone.close();
    });

    let window_clone = window.clone();
    let config_clone = current_config.clone();
    save_button.connect_clicked(move |_| {
        // Save configuration
        let config = config_clone.borrow();
        if let Err(e) = config.save() {
            log::error!("Failed to save configuration: {}", e);
        } else {
            log::info!("Configuration saved successfully");
        }
        window_clone.close();
    });

    window.set_child(Some(&main_box));
    window.present();
}

/// Create the Weather API selection section
fn create_api_selection_section(config: &Rc<RefCell<Config>>) -> Box {
    let section_box = Box::builder()
        .orientation(gtk::Orientation::Vertical)
        .spacing(6)
        .build();

    let title = Label::builder()
        .label("<b>Weather API Provider</b>")
        .use_markup(true)
        .halign(gtk::Align::Start)
        .build();

    let api_combo = ComboBoxText::new();
    api_combo.append_text("OpenWeather API");
    api_combo.append_text("Google Weather API");

    // Set current selection
    let current_provider = &config.borrow().weather_provider;
    match current_provider {
        WeatherProvider::OpenWeather => api_combo.set_active(Some(0)),
        WeatherProvider::GoogleWeather => api_combo.set_active(Some(1)),
    }

    // Handle selection changes
    let config_clone = config.clone();
    api_combo.connect_changed(move |combo| {
        if let Some(active) = combo.active() {
            let mut config = config_clone.borrow_mut();
            config.weather_provider = match active {
                0 => WeatherProvider::OpenWeather,
                1 => WeatherProvider::GoogleWeather,
                _ => WeatherProvider::OpenWeather,
            };
        }
    });

    section_box.append(&title);
    section_box.append(&api_combo);
    section_box
}

/// Create the API keys configuration section
fn create_api_keys_section(config: &Rc<RefCell<Config>>) -> Box {
    let section_box = Box::builder()
        .orientation(gtk::Orientation::Vertical)
        .spacing(6)
        .build();

    let title = Label::builder()
        .label("<b>API Keys</b>")
        .use_markup(true)
        .halign(gtk::Align::Start)
        .build();

    let grid = Grid::builder().row_spacing(6).column_spacing(12).build();

    // OpenWeather API Key
    let openweather_label = Label::builder()
        .label("OpenWeather API Key:")
        .halign(gtk::Align::Start)
        .build();

    let openweather_entry = Entry::builder()
        .placeholder_text("Enter OpenWeather API key")
        .hexpand(true)
        .build();

    openweather_entry.set_text(&config.borrow().openweather_api_key);

    // Google Weather API Key
    let google_label = Label::builder()
        .label("Google Weather API Key:")
        .halign(gtk::Align::Start)
        .build();

    let google_entry = Entry::builder()
        .placeholder_text("Enter Google Weather API key")
        .hexpand(true)
        .build();

    google_entry.set_text(&config.borrow().google_weather_api_key);

    // Connect entry changes to config
    let config_clone = config.clone();
    openweather_entry.connect_changed(move |entry| {
        config_clone.borrow_mut().openweather_api_key = entry.text().to_string();
    });

    let config_clone = config.clone();
    google_entry.connect_changed(move |entry| {
        config_clone.borrow_mut().google_weather_api_key = entry.text().to_string();
    });

    grid.attach(&openweather_label, 0, 0, 1, 1);
    grid.attach(&openweather_entry, 1, 0, 1, 1);
    grid.attach(&google_label, 0, 1, 1, 1);
    grid.attach(&google_entry, 1, 1, 1, 1);

    section_box.append(&title);
    section_box.append(&grid);
    section_box
}

/// Create the default location configuration section
fn create_location_section(config: &Rc<RefCell<Config>>) -> Box {
    let section_box = Box::builder()
        .orientation(gtk::Orientation::Vertical)
        .spacing(6)
        .build();

    let title = Label::builder()
        .label("<b>Default Location</b>")
        .use_markup(true)
        .halign(gtk::Align::Start)
        .build();

    let grid = Grid::builder().row_spacing(6).column_spacing(12).build();

    // City
    let city_label = Label::builder()
        .label("City:")
        .halign(gtk::Align::Start)
        .build();

    let city_entry = Entry::builder()
        .placeholder_text("Enter city name")
        .hexpand(true)
        .build();

    city_entry.set_text(&config.borrow().default_location.city);

    // State
    let state_label = Label::builder()
        .label("State/Province:")
        .halign(gtk::Align::Start)
        .build();

    let state_entry = Entry::builder()
        .placeholder_text("Enter state or province")
        .hexpand(true)
        .build();

    state_entry.set_text(&config.borrow().default_location.state);

    // Country
    let country_label = Label::builder()
        .label("Country:")
        .halign(gtk::Align::Start)
        .build();

    let country_entry = Entry::builder()
        .placeholder_text("Enter country code (e.g., US, CA)")
        .hexpand(true)
        .build();

    country_entry.set_text(&config.borrow().default_location.country);

    // Connect entry changes to config
    let config_clone = config.clone();
    city_entry.connect_changed(move |entry| {
        config_clone.borrow_mut().default_location.city = entry.text().to_string();
    });

    let config_clone = config.clone();
    state_entry.connect_changed(move |entry| {
        config_clone.borrow_mut().default_location.state = entry.text().to_string();
    });

    let config_clone = config.clone();
    country_entry.connect_changed(move |entry| {
        config_clone.borrow_mut().default_location.country = entry.text().to_string();
    });

    grid.attach(&city_label, 0, 0, 1, 1);
    grid.attach(&city_entry, 1, 0, 1, 1);
    grid.attach(&state_label, 0, 1, 1, 1);
    grid.attach(&state_entry, 1, 1, 1, 1);
    grid.attach(&country_label, 0, 2, 1, 1);
    grid.attach(&country_entry, 1, 2, 1, 1);

    section_box.append(&title);
    section_box.append(&grid);
    section_box
}
