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
        // iced's internals log full window/compositor structs at Info level
        // on every launch (window attributes, GPU adapter info, etc.) --
        // useful when debugging iced itself, just noise otherwise.
        .filter_module("iced_winit", LevelFilter::Warn)
        .filter_module("iced_wgpu", LevelFilter::Warn)
        .init();

    log::info!("Starting Weather Wizard application");
    app::run()
}
