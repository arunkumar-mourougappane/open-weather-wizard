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

use gtk::prelude::*;
use gtk::{Application, glib};
use log::{self, LevelFilter}; // Import necessary traits for GTK widgets

mod config;
mod ui;
mod weather_api;

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
