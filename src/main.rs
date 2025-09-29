//! # Weather Wizard Application Entry Point
//!
//! This is the main executable for the Weather Wizard GTK application.
//!
//! Its primary responsibilities are:
//! 1.  Initializing the logging framework (`env_logger`).
//! 2.  Calling into the `ui` module to build the `gtk::Application`.
//! 3.  Running the GTK application's main event loop.
//!
//! All application logic, including UI construction, state management, and API
//! calls, is handled within the `meteo_wizard` library crate, which is organized
//! into the `config`, `ui`, and `weather_api` modules.
use env_logger::{self, Builder};

use gtk::prelude::*;
use gtk::{Application, glib};
use log::{self, LevelFilter}; // Import necessary traits for GTK widgets

// These modules are part of the library crate, but are declared here to be
// included in the binary build.
mod config;
mod ui;
mod weather_api;

/// The main entry point for the Weather Wizard application.
///
/// This function initializes the logger, builds the main UI, and runs the GTK application.
/// It serves as the asynchronous entry point required by `tokio`.
///
/// # Returns
///
/// A `glib::ExitCode` indicating the application's exit status.
#[tokio::main]
async fn main() -> glib::ExitCode {
    Builder::new()
        // Set the default log level to `info` if RUST_LOG is not set
        .filter_level(LevelFilter::Info)
        // You can also specifically filter certain modules
        .filter_module("noisy_crate", LevelFilter::Warn)
        .init();

    log::info!("Starting Weather Wizard application");
    let application: Application = ui::build_main_ui();
    // Connect the "activate" signal to a closure that builds the UI

    // Run the application
    application.run()
}
