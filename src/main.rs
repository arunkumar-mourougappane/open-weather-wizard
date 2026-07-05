//! # Weather Wizard Application Entry Point
//!
//! This is the main executable for the Weather Wizard application, built on `iced`.
//!
//! All application logic, including UI construction, state management, and API
//! calls, is handled within the `open_weather_wizard` library crate, organized into
//! the `config`, `app`, `ui`, and `weather_api` modules. `cli` (this bin only,
//! not part of the library) adds a `--headless` mode -- see its own doc comment.
use clap::Parser;
use env_logger::{self, Builder};
use log::{self, LevelFilter};

mod app;
mod cli;
mod config;
mod geolocation;
mod ui;
mod weather_api;

/// The main entry point for the Weather Wizard application.
///
/// iced manages its own tokio runtime internally (via its `tokio` executor feature),
/// so this must stay a plain, synchronous `fn main` rather than `#[tokio::main]` --
/// nesting a second runtime around `app::run()` would conflict with it. `cli::run`
/// (the `--headless` path) never touches `app::run()`, so it's free to spin up its
/// own runtime instead.
fn main() -> iced::Result {
    let cli = cli::Cli::parse();

    // Info-level logging (config loads, fetch lifecycle, etc.) is useful
    // during development but just noise -- and clutters --headless's stdout
    // output -- in a release build; `cargo build --release` disables it in
    // favor of Warn/Error only. `RUST_LOG` still overrides this if a user
    // explicitly wants Info (or more) out of a release binary.
    let default_level = if cfg!(debug_assertions) {
        LevelFilter::Info
    } else {
        LevelFilter::Warn
    };

    Builder::new()
        .filter_level(default_level)
        // iced's internals log full window/compositor structs at Info level
        // on every launch (window attributes, GPU adapter info, etc.) --
        // useful when debugging iced itself, just noise otherwise.
        .filter_module("iced_winit", LevelFilter::Warn)
        .filter_module("iced_wgpu", LevelFilter::Warn)
        .parse_default_env()
        .init();

    if cli.headless {
        cli::run(&cli);
    }

    log::info!("Starting Weather Wizard application");
    app::run()
}
