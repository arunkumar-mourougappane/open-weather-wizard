//! # Weather Wizard Application Entry Point
//!
//! This is the main executable for the Weather Wizard application, built on `iced`.
//!
//! All application logic, including UI construction, state management, and API
//! calls, is handled within the `open_weather_wizard` library crate, organized into
//! the `config`, `app`, `ui`, and `weather_api` modules.
use env_logger::{self, Builder};
use log::{self, LevelFilter};

mod app;
mod config;
mod ui;
mod weather_api;

/// The main entry point for the Weather Wizard application.
///
/// iced manages its own tokio runtime internally (via its `tokio` executor feature),
/// so this must stay a plain, synchronous `fn main` rather than `#[tokio::main]` --
/// nesting a second runtime around `app::run()` would conflict with it.
fn main() -> iced::Result {
    Builder::new()
        .filter_level(LevelFilter::Info)
        .filter_module("noisy_crate", LevelFilter::Warn)
        .init();

    log::info!("Starting Weather Wizard application");
    app::run()
}
