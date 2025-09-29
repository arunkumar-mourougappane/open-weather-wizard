pub mod build_elements;
pub mod preferences;

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
use crate::weather_api::openweather_api::ApiError;
use crate::weather_api::weather_provider::WeatherProviderFactory;

/// A container for the UI widgets that need to be updated.
#[derive(Clone)]
pub struct UIWidgets {
    temp_label: Label,
    description_label: Label,
    humidity_label: Label,
    weather_symbol_image: Image,
    spinner: gtk::Spinner,
}

#[allow(clippy::await_holding_lock)]
fn fetch_and_update_weather(widgets: &UIWidgets, config: &Arc<Mutex<Config>>) {
    let widgets_clone = widgets.clone();
    let config_clone = Arc::clone(config);

    glib::spawn_future_local(async move {
        widgets_clone.spinner.start();
        widgets_clone.spinner.set_visible(true);
        widgets_clone
            .description_label
            .set_text("Fetching weather...");

        let current_config = config_clone.lock().expect("Failed to lock config");
        let location_config = current_config.location.clone();
        let provider_type = current_config.weather_provider.clone();
        let api_token = current_config.get_api_token().ok();
        drop(current_config);

        let provider_result = WeatherProviderFactory::create_provider(&provider_type, api_token);

        widgets_clone.spinner.stop();
        widgets_clone.spinner.set_visible(false);

        match provider_result {
            Ok(provider) => match provider.get_weather(&location_config).await {
                Ok(weather_data) => {
                    if let Err(e) = update_ui_with_weather(&weather_data, &widgets_clone) {
                        widgets_clone
                            .description_label
                            .set_text(&format!("Error: {}", e));
                        widgets_clone.weather_symbol_image.set_from_pixbuf(None);
                    }
                }
                Err(e) => {
                    let error_message = match e {
                        ApiError::CityNotFound => "City not found.",
                        ApiError::RequestFailed(_) => "Network request failed.",
                        ApiError::InvalidResponse => "Could not parse server response.", // Invalid API key can also cause this
                    };
                    widgets_clone.description_label.set_text(error_message);
                    widgets_clone.weather_symbol_image.set_from_pixbuf(None);
                }
            },
            Err(e) => {
                widgets_clone
                    .description_label
                    .set_text(&format!("Configuration error: {}", e));
                widgets_clone.weather_symbol_image.set_from_pixbuf(None);
            }
        }
    });
}

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
        let temp_label = Label::new(Some("--°C"));
        let description_label = Label::new(Some("Enter a city to begin"));
        let humidity_label = Label::new(Some("Humidity: --%"));

        // Add CSS classes for styling
        // weather_symbol_image.add_css_class("weather-symbol");
        description_label.add_css_class("weather-description");
        temp_label.add_css_class("weather-temp");
        humidity_label.add_css_class("weather-humidity");

        // Create and add a spinner
        let spinner: gtk::Spinner = build_spinner(40);
        spinner.set_visible(false);
        vbox.append(&spinner);

        let widgets = UIWidgets {
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
