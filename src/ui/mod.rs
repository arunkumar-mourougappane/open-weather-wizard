//! # Main UI Module
//!
//! This module orchestrates the entire user interface of the Weather Wizard application.
//! It is responsible for:
//!
//! - **Building the Application**: Creating the main `gtk::Application` instance.
//! - **Window and Widget Setup**: Constructing the main application window, menu bar,
//!   labels, and other widgets upon application activation.
//! - **State Management**: Managing the application's configuration (`AppConfig`)
//!   using an `Arc<Mutex<>>` for safe concurrent access.
//! - **Asynchronous Operations**: Spawning asynchronous tasks with `glib::spawn_future_local`
//!   to fetch weather data without blocking the UI thread.
//! - **UI Updates**: Handling the logic to update UI elements with new weather data or
//!   error messages.
//! - **Event Handling**: Connecting signals for menu actions (like "Preferences" and "Quit")
//!   and setting up a periodic timer for automatic weather updates.

pub mod build_elements;
pub mod preferences;
pub mod about;

use gtk::gio::MenuModel;
use gtk::prelude::*;
use gtk::{Application, ApplicationWindow, Image, Label, PopoverMenuBar, gio, glib};
use std::sync::{Arc, Mutex};

use crate::config::{AppConfig as Config, ConfigManager};
use crate::ui::build_elements::{
    DEFAULT_WINDOW_HEIGHT, DEFAULT_WINDOW_WIDTH, build_main_menu, build_spinner,
    update_ui_with_weather,
};
use crate::ui::preferences::show_preferences_window;
use crate::ui::about::show_about_dialog;
use crate::weather_api::openweather_api::ApiError;
use crate::weather_api::weather_provider::WeatherProviderFactory;

/// A container for UI widgets that need to be accessed and updated dynamically.
///
/// This struct is cloneable and holds references to the GTK widgets that display
/// weather information, allowing them to be easily passed between functions and closures.
#[derive(Clone)]
pub struct UIWidgets {
    location_label: Label,
    temp_label: Label,
    description_label: Label,
    humidity_label: Label,
    weather_symbol_image: Image,
    spinner: gtk::Spinner,
}

/// A unified error type for the application's UI and backend logic.
///
/// This enum consolidates errors from different parts of the application, such as
/// configuration issues, API failures, and UI rendering problems, into a single type.
enum AppError {
    Config(String),
    Api(ApiError),
    Ui(anyhow::Error),
}

impl From<ApiError> for AppError {
    fn from(e: ApiError) -> Self {
        AppError::Api(e)
    }
}

impl From<anyhow::Error> for AppError {
    fn from(e: anyhow::Error) -> Self {
        AppError::Ui(e)
    }
}

/// Asynchronously fetches weather data and triggers a UI update.
///
/// This function reads the current configuration, creates the appropriate weather provider,
/// fetches the weather data, and then calls `update_ui_with_weather` to refresh the
/// display. It handles the conversion of different error types into the unified `AppError`.
///
/// # Arguments
///
/// * `widgets` - A reference to the `UIWidgets` struct containing the UI elements to update.
/// * `config` - An `Arc<Mutex<Config>>` containing the application's configuration.
///
/// # Errors
/// Returns an `AppError` if configuration is invalid, the API call fails, or the UI update fails.
#[allow(clippy::await_holding_lock)]
async fn fetch_weather_data(
    widgets: &UIWidgets,
    config: &Arc<Mutex<Config>>,
) -> Result<(), AppError> {
    let current_config = config.lock().expect("Failed to lock config");
    let location_config = current_config.location.clone();
    let provider_type = current_config.weather_provider.clone();
    let api_token = current_config.get_api_token().ok();
    let config_clone = current_config.clone();
    drop(current_config);

    let provider = WeatherProviderFactory::create_provider(&provider_type, api_token)
        .map_err(|e| AppError::Config(e.to_string()))?;
    let weather_data = provider.get_weather(&location_config).await?;

    update_ui_with_weather(&weather_data, &config_clone, widgets)?;

    Ok(())
}

/// Spawns a non-blocking task to fetch and update weather data in the UI.
///
/// This function wraps the asynchronous `fetch_weather_data` call in a `glib::spawn_future_local`.
/// It manages the UI state during the fetch operation by:
/// 1. Showing and starting a spinner.
/// 2. Displaying a "Fetching weather..." message.
/// 3. On success, the UI is updated with weather data.
/// 4. On failure, an appropriate error message is displayed to the user.
/// 5. Hiding and stopping the spinner when the operation is complete.
///
/// # Arguments
///
/// * `widgets` - A reference to the `UIWidgets` struct containing the UI elements to update.
/// * `config` - An `Arc<Mutex<Config>>` containing the application's configuration.
fn fetch_and_update_weather(widgets: &UIWidgets, config: &Arc<Mutex<Config>>) {
    let widgets_clone = widgets.clone();
    let config_clone = Arc::clone(config);

    glib::spawn_future_local(async move {
        widgets_clone.spinner.start();
        widgets_clone.spinner.set_visible(true);
        widgets_clone
            .description_label
            .set_text("Fetching weather...");

        if let Err(e) = fetch_weather_data(&widgets_clone, &config_clone).await {
            let error_message = match e {
                AppError::Config(msg) => format!("Configuration error: {}", msg),
                AppError::Api(api_error) => match api_error {
                    ApiError::CityNotFound => "City not found.".to_string(),
                    ApiError::RequestFailed(_) => "Network request failed.".to_string(),
                    ApiError::InvalidResponse => "Could not parse server response.".to_string(),
                },
                AppError::Ui(ui_error) => format!("UI error: {}", ui_error),
            };
            widgets_clone.description_label.set_text(&error_message);
            widgets_clone.weather_symbol_image.set_from_pixbuf(None);
        }

        widgets_clone.spinner.stop();
        widgets_clone.spinner.set_visible(false);
    });
}

/// Builds and configures the main GTK application.
///
/// This function is the entry point for the UI. It initializes the `gtk::Application`,
/// loads the configuration, and connects the `activate` signal to a closure that
/// builds the main window, its widgets, and sets up all event handlers and timers.
///
/// # Returns
///
/// A `gtk::Application` instance.
pub fn build_main_ui() -> Application {
    // Load configuration
    let config_manager = ConfigManager::new().expect("Failed to create config manager");
    let config = Arc::new(Mutex::new(config_manager.load_config()));

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
        let location_label = Label::new(None);
        let temp_label = Label::new(Some("--°C"));
        let description_label = Label::new(Some("Enter a city to begin"));
        let humidity_label = Label::new(Some("Humidity: --%"));

        // Add CSS classes for styling
        // weather_symbol_image.add_css_class("weather-symbol");
        description_label.add_css_class("weather-description");
        location_label.add_css_class("location-label");
        temp_label.add_css_class("weather-temp");
        humidity_label.add_css_class("weather-humidity");

        // Create and add a spinner
        let spinner: gtk::Spinner = build_spinner(40);
        spinner.set_visible(false);
        vbox.append(&spinner);

        let widgets = UIWidgets {
            location_label,
            temp_label,
            description_label,
            humidity_label,
            weather_symbol_image,
            spinner,
        };

        // Arrange widgets vertically in a Box container
        let main_box = gtk::Box::builder()
            .orientation(gtk::Orientation::Vertical)
            .spacing(6)
            .margin_top(12)
            .margin_bottom(12)
            .margin_start(12)
            .margin_end(12)
            .build();

        main_box.append(&widgets.weather_symbol_image);
        main_box.append(&widgets.location_label);
        main_box.append(&widgets.temp_label);
        main_box.append(&widgets.description_label);
        main_box.append(&widgets.humidity_label);

        // Add menu actions
        let preferences_action = gio::SimpleAction::new("preferences", None);
        let config_clone_for_prefs = config_clone.clone();
        let window_clone = window.clone();
        let widgets_for_prefs = widgets.clone();

        preferences_action.connect_activate(move |_, _| {
            let widgets_clone = widgets_for_prefs.clone();
            let config_clone = config_clone_for_prefs.clone();

            show_preferences_window(&window_clone, config_clone_for_prefs.clone(), move || {
                fetch_and_update_weather(&widgets_clone, &config_clone);
            });
        });
        app.add_action(&preferences_action);

        let about_action = gio::SimpleAction::new("about", None);
        let window_clone = window.clone();
        about_action.connect_activate(move |_, _| {
            show_about_dialog(&window_clone);
        });
        app.add_action(&about_action);

        let quit_action = gio::SimpleAction::new("quit", None);
        let app_clone = app.clone();
        quit_action.connect_activate(move |_, _| {
            app_clone.quit();
        });
        app.add_action(&quit_action);

        vbox.append(&main_box);
        window.set_child(Some(&vbox));
        // Present the window to the user
        window.present();

        // Initial weather fetch
        fetch_and_update_weather(&widgets, &config_clone);

        // Set up auto-update timer (e.g., every 15 minutes)
        let widgets_timer = widgets.clone();
        let config_timer = config_clone.clone();

        glib::timeout_add_local(std::time::Duration::from_secs(30), move || {
            fetch_and_update_weather(&widgets_timer, &config_timer);
            glib::ControlFlow::Continue
        });
    });
    application
}
