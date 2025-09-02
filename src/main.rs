//! # Weather Wizard GTK Application
//!
//! This file contains the main entry point and UI setup for the Weather Wizard application,
//! built using GTK4 and Rust. The application demonstrates how to create a GTK application
//! window, set up a menu bar, and add interactive widgets such as buttons and spinners.
//!
//! ## Modules and Functions
//!
//! - Imports necessary GTK4 and GLib traits and types.
//! - Uses custom UI builder functions from the `build_elements` module.
//!
//! ### `build_main_ui`
//! Creates and configures the main GTK application, sets up the window, menu bar, and widgets.
//!
//! ### `main`
//! Entry point of the application. Runs the GTK application and returns the exit code.
//!
//! ## Widgets
//!
//! - **Menu Bar:** Built using `PopoverMenuBar` and a custom menu model.
//! - **Button:** Created with a custom builder function.
//! - **Spinner:** Created and started to indicate loading or processing.
//!
//! ## Usage
//!
//! Run the application to launch the Weather Wizard UI window.
use env_logger::{self, Builder};
use gtk::PopoverMenuBar;
use gtk::gio::MenuModel;
use gtk::{Application, ApplicationWindow};
use gtk::{Image, Label, prelude::*};
use log::{self, LevelFilter}; // Import necessary traits for GTK widgets
mod config;
mod ui;
mod weather_api;
use ui::build_elements::{
    DEFAULT_WINDOW_HEIGHT, DEFAULT_WINDOW_WIDTH, build_button, build_entry, build_main_menu,
    build_spinner,
};

use crate::ui::build_elements::update_ui_with_weather;
use crate::weather_api::openweather_api::{self, ApiError, Location}; // For glib::ExitCode
use crate::config::{Config, WeatherProvider};
use std::cell::RefCell;
use std::rc::Rc;

fn build_main_ui() -> Application {
    // Create a new GTK application
    let application = Application::builder()
        .application_id("com.example.FirstGtkApp") // Unique application ID
        .build();
    
    application.connect_activate(|app| {
        // Load configuration
        let config = match Config::load() {
            Ok(config) => config,
            Err(e) => {
                log::error!("Failed to load configuration: {}", e);
                Config::default()
            }
        };
        let config = Rc::new(RefCell::new(config));
        
        // Create a new application window
        let window = ApplicationWindow::builder()
            .application(app) // Associate the window with the application
            .title("Weather Wizard") // Set the window title
            .default_width(DEFAULT_WINDOW_WIDTH)
            .default_height(DEFAULT_WINDOW_HEIGHT)
            .build();

        // Set up menu actions
        setup_menu_actions(app, &window, config.clone());

        // Create root menu and add submenus
        let root_menu = build_main_menu();

        // Convert to MenuModel
        let menu_model: MenuModel = root_menu.into();

        // Create PopoverMenuBar
        let menubar = PopoverMenuBar::from_model(Some(&menu_model));

        // Add menubar to the window (e.g., within a Box)
        let vbox = gtk::Box::builder()
            .orientation(gtk::Orientation::Vertical)
            .build();

        let location_box = gtk::Box::builder()
            .orientation(gtk::Orientation::Horizontal)
            .build();

        vbox.append(&menubar);
        vbox.append(&location_box);
        
        // Create and add entry fields with default values from config
        let default_location = &config.borrow().default_location;
        let city_entry = build_entry("City".to_string());
        city_entry.set_text(&default_location.city);
        location_box.append(&city_entry);
        
        // State and Country entries
        let state_entry = build_entry("State".to_string());
        state_entry.set_text(&default_location.state);
        location_box.append(&state_entry);
        
        let country_entry = build_entry("Country".to_string());
        country_entry.set_text(&default_location.country);
        location_box.append(&country_entry);

        // Weather symbol image
        let weather_symbol_image = Image::from_pixbuf(None);
        weather_symbol_image.set_pixel_size(128);
        // Labels for displaying weather data
        let temp_label = Label::new(Some("--°C"));
        let description_label = Label::new(Some("Enter a city to begin"));
        let humidity_label = Label::new(Some("Humidity: --%"));

        // Add CSS classes for styling
        // weather_symbol_image.add_css_class("weather-symbol");
        description_label.add_css_class("weather-description");
        temp_label.add_css_class("weather-temp");
        humidity_label.add_css_class("weather-humidity");

        // Create a button
        let weather_button = build_button("Get Weather".to_string());
        // Add the button to the window
        vbox.append(&weather_button);

        // Create and add a spinner
        let spinner: gtk::Spinner = build_spinner(40);
        spinner.set_visible(false);
        vbox.append(&spinner);

        // Arrange widgets vertically in a Box container
        let main_box = gtk::Box::builder()
            .orientation(gtk::Orientation::Vertical)
            .spacing(6)
            .margin_top(12)
            .margin_bottom(12)
            .margin_start(12)
            .margin_end(12)
            .build();

        main_box.append(&weather_symbol_image);
        main_box.append(&temp_label);
        main_box.append(&description_label);
        main_box.append(&humidity_label);
        main_box.append(&city_entry);

        // Clone config for the button handler
        let config_for_button = config.clone();
        weather_button.connect_clicked(move |_| {
            // Clone the widgets that we need to modify inside the button's click handler
            let city_entry_clone = city_entry.clone();
            let state_entry_clone = state_entry.clone();
            let country_entry_clone = country_entry.clone();
            let temp_label_clone = temp_label.clone();
            let description_label_clone = description_label.clone();
            let humidity_label_clone = humidity_label.clone();
            let weather_symbol_image_clone = weather_symbol_image.clone();
            let spinner = spinner.clone();
            let config = config_for_button.clone();
            
            // Get the city name from the entry field
            let city = city_entry_clone.text().to_string();
            if city.is_empty() {
                return;
            }
            let state = state_entry_clone.text().to_string();
            if state.is_empty() {
                return;
            }
            let country = country_entry_clone.text().to_string();
            if country.is_empty() {
                return;
            }

            // Use glib::spawn_future_local to run our async API call without blocking the UI
            glib::spawn_future_local(async move {
                spinner.start(); // Start the spinner animation
                spinner.set_visible(true); // Make the spinner visible
                description_label_clone.set_text("Fetching weather...");
                
                let location = Location {
                    state: Some(state.clone()),
                    country: Some(country.clone()),
                    name: city.clone(),
                    lat: 0.0,
                    lon: 0.0,
                };
                
                // Get the weather data based on selected API
                let result = match config.borrow().weather_provider {
                    WeatherProvider::OpenWeather => {
                        openweather_api::get_weather(&location).await
                    },
                    WeatherProvider::GoogleWeather => {
                        crate::weather_api::google_weather_api::get_weather(
                            &location, 
                            &config.borrow().google_weather_api_key
                        ).await
                    },
                };
                
                spinner.stop(); // Stop the spinner animation
                spinner.set_visible(false); // Hide the spinner
                match result {
                    Ok(weather_data) => {
                        match update_ui_with_weather(
                            &weather_data,
                            &weather_symbol_image_clone,
                            &temp_label_clone,
                            &description_label_clone,
                            &humidity_label_clone,
                        ) {
                            Ok(()) => {}
                            Err(e) => {
                                description_label_clone.set_text(&format!("Error: {}", e));
                                weather_symbol_image_clone.set_from_pixbuf(None);
                            }
                        }
                    }
                    Err(e) => {
                        let error_message = match e {
                            ApiError::CityNotFound => "City not found.",
                            ApiError::RequestFailed(_) => "Network request failed.",
                            ApiError::InvalidResponse => "Could not parse server response.",
                        };
                        description_label_clone.set_text(error_message);
                        weather_symbol_image_clone.set_from_pixbuf(None);
                    }
                }
            });
        });

        vbox.append(&main_box);
        window.set_child(Some(&vbox));
        // Present the window to the user
        window.present();
    });
    application
}

fn setup_menu_actions(app: &Application, window: &ApplicationWindow, config: Rc<RefCell<Config>>) {
    // Preferences action
    let preferences_action = gtk::gio::SimpleAction::new("preferences", None);
    let window_weak = window.downgrade();
    let config_clone = config.clone();
    preferences_action.connect_activate(move |_, _| {
        if let Some(window) = window_weak.upgrade() {
            crate::ui::preferences_window::show_preferences_window(&window, config_clone.clone());
        }
    });
    app.add_action(&preferences_action);

    // Exit action
    let exit_action = gtk::gio::SimpleAction::new("exit", None);
    let app_weak = app.downgrade();
    exit_action.connect_activate(move |_, _| {
        if let Some(app) = app_weak.upgrade() {
            app.quit();
        }
    });
    app.add_action(&exit_action);

    // About action (placeholder)
    let about_action = gtk::gio::SimpleAction::new("about", None);
    about_action.connect_activate(move |_, _| {
        log::info!("About menu clicked - not implemented yet");
    });
    app.add_action(&about_action);

    // Help action (placeholder)
    let help_action = gtk::gio::SimpleAction::new("help", None);
    help_action.connect_activate(move |_, _| {
        log::info!("Help menu clicked - not implemented yet");
    });
    app.add_action(&help_action);
}

#[tokio::main]
async fn main() -> glib::ExitCode {
    Builder::new()
        // Set the default log level to `info` if RUST_LOG is not set
        .filter_level(LevelFilter::Info)
        // You can also specifically filter certain modules
        .filter_module("noisy_crate", LevelFilter::Warn)
        .init();

    log::info!("Starting Weather Wizard application");
    let application: Application = build_main_ui();
    // Connect the "activate" signal to a closure that builds the UI

    // Run the application
    application.run()
}
