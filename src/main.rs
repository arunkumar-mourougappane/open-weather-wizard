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
    DEFAULT_WINDOW_HEIGHT, DEFAULT_WINDOW_WIDTH, build_button, build_main_menu,
    build_spinner,
};

use crate::ui::build_elements::update_ui_with_weather;
use crate::weather_api::openweather_api::ApiError;
use crate::config::ConfigManager;
use crate::weather_api::weather_provider::WeatherProviderFactory;
use crate::ui::preferences::show_preferences_window;
use std::rc::Rc;
use std::cell::RefCell;

fn build_main_ui() -> Application {
    // Load configuration
    let config_manager = ConfigManager::new().expect("Failed to create config manager");
    let config = Rc::new(RefCell::new(config_manager.load_config()));
    
    // Create a new GTK application
    let application = Application::builder()
        .application_id("com.example.FirstGtkApp") // Unique application ID
        .build();
    let config_clone = config.clone();
    application.connect_activate(move |app| {
        // Create a new application window
        let window = ApplicationWindow::builder()
            .application(app) // Associate the window with the application
            .title("Weather Wizard") // Set the window title
            .default_width(DEFAULT_WINDOW_WIDTH)
            .default_height(DEFAULT_WINDOW_HEIGHT)
            .build();

        // Add menu actions
        let preferences_action = gio::SimpleAction::new("preferences", None);
        let config_clone_for_prefs = config_clone.clone();
        let window_clone = window.clone();
        preferences_action.connect_activate(move |_, _| {
            show_preferences_window(&window_clone, config_clone_for_prefs.clone());
        });
        app.add_action(&preferences_action);

        let quit_action = gio::SimpleAction::new("quit", None);
        let app_clone = app.clone();
        quit_action.connect_activate(move |_, _| {
            app_clone.quit();
        });
        app.add_action(&quit_action);

        window.present();

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

        vbox.append(&menubar);

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

        let config_for_button = config_clone.clone();
        weather_button.connect_clicked(move |_| {
            // Clone the widgets that we need to modify inside the button's click handler
            let temp_label_clone = temp_label.clone();
            let description_label_clone = description_label.clone();
            let humidity_label_clone = humidity_label.clone();
            let weather_symbol_image_clone = weather_symbol_image.clone();
            let spinner = spinner.clone();
            let config_clone = config_for_button.clone();
            
            // Use glib::spawn_future_local to run our async API call without blocking the UI
            glib::spawn_future_local(async move {
                spinner.start(); // Start the spinner animation
                spinner.set_visible(true); // Make the spinner visible
                description_label_clone.set_text("Fetching weather...");
                
                let current_config = config_clone.borrow();
                let location_config = current_config.location.clone();
                let provider_type = current_config.weather_provider.clone();
                let api_token = current_config.get_api_token().ok();
                drop(current_config); // Release the borrow
                
                // Create weather provider
                let provider_result = WeatherProviderFactory::create_provider(&provider_type, api_token);
                
                spinner.stop(); // Stop the spinner animation
                spinner.set_visible(false); // Hide the spinner
                
                match provider_result {
                    Ok(provider) => {
                        // Call the weather API through the provider
                        match provider.get_weather(&location_config).await {
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
                    }
                    Err(e) => {
                        description_label_clone.set_text(&format!("Configuration error: {}", e));
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
