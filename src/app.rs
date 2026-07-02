//! # Application State and Update Logic
//!
//! This is the iced root of the application: the `AppState`/`Message` pair, and the
//! `boot`/`update`/`view`/`subscription` functions wired together by `run()`.
//!
//! The app is a `daemon` (multi-window) rather than a single-window `application`,
//! since Preferences and About are rendered as separate OS windows, closer to the
//! transient-window feel of the previous GTK version than an in-app overlay.

use std::time::Duration;

use iced::widget::Space;
use iced::{Element, Size, Subscription, Task, Theme, window};

use crate::config::{AppConfig, ConfigManager};
use crate::ui::{about, main_screen, preferences};
use crate::weather_api::forecast::ForecastResponse;
use crate::weather_api::openweather_api::ApiResponse;
use crate::weather_api::weather_provider::WeatherProviderFactory;

pub const DEFAULT_WINDOW_WIDTH: f32 = 720.0;
pub const DEFAULT_WINDOW_HEIGHT: f32 = 480.0;
const AUTO_REFRESH_INTERVAL: Duration = Duration::from_secs(30);
/// Drives redraws for the animated Lottie icons (~30fps); `icons::view`
/// computes each frame from wall-clock time, so this tick carries no state of
/// its own -- it exists purely to make iced re-invoke `view()` regularly.
const ANIMATION_TICK_INTERVAL: Duration = Duration::from_millis(33);

/// The lifecycle of an async weather fetch, driving the main screen's display.
#[derive(Debug, Clone)]
pub enum WeatherStatus {
    Loading,
    Loaded(ApiResponse),
    Error(String),
}

/// The lifecycle of an async forecast fetch. Kept separate from `WeatherStatus`
/// so a forecast failure (or an empty result from a provider like Google Weather,
/// which has no real forecast integration) never blanks out current conditions.
///
/// Unlike `WeatherStatus::Error`, this carries no message: a forecast failure is
/// never surfaced in the UI (the row is simply omitted), so the failure reason is
/// only logged at the point of transition in `update()`.
#[derive(Debug, Clone)]
pub enum ForecastStatus {
    Loading,
    Loaded(ForecastResponse),
    Error,
}

/// Top-level application state, owned directly (no `Arc<Mutex<>>`): iced's Elm
/// architecture already serializes every mutation through `update()`, so the
/// shared-closure problem the GTK version solved with a mutex doesn't exist here.
pub struct AppState {
    pub config: AppConfig,
    config_manager: ConfigManager,
    pub weather: WeatherStatus,
    pub forecast: ForecastStatus,
    main_window: window::Id,
    prefs_window: Option<window::Id>,
    prefs_state: Option<preferences::State>,
    about_window: Option<window::Id>,
}

#[derive(Debug, Clone)]
pub enum Message {
    RefreshRequested,
    Tick(#[allow(dead_code)] std::time::Instant),
    WeatherFetched(Result<ApiResponse, String>),
    ForecastFetched(Result<ForecastResponse, String>),

    OpenPreferences,
    OpenAbout,
    WindowCloseRequested(window::Id),
    AnimationTick,

    Preferences(preferences::Message),
}

/// Builds a `Task` that fetches current weather for the active provider/location.
fn fetch_weather_task(config: &AppConfig) -> Task<Message> {
    let provider_type = config.weather_provider.clone();
    let location = config.location.clone();
    let token = config.get_api_token().ok();

    Task::perform(
        async move {
            let provider = WeatherProviderFactory::create_provider(&provider_type, token)?;
            provider
                .get_weather(&location)
                .await
                .map_err(|e| format!("{:?}", e))
        },
        Message::WeatherFetched,
    )
}

/// Builds a `Task` that fetches a forecast for the active provider/location.
fn fetch_forecast_task(config: &AppConfig) -> Task<Message> {
    let provider_type = config.weather_provider.clone();
    let location = config.location.clone();
    let token = config.get_api_token().ok();

    Task::perform(
        async move {
            let provider = WeatherProviderFactory::create_provider(&provider_type, token)?;
            provider
                .get_forecast(&location)
                .await
                .map_err(|e| format!("{:?}", e))
        },
        Message::ForecastFetched,
    )
}

/// Boots the application: loads config, opens the main window, and kicks off the
/// first weather + forecast fetch.
pub fn boot() -> (AppState, Task<Message>) {
    let config_manager = ConfigManager::new().expect("Failed to create config manager");
    let config = config_manager.load_config();

    let (main_window, open_task) = window::open(window::Settings {
        size: Size::new(DEFAULT_WINDOW_WIDTH, DEFAULT_WINDOW_HEIGHT),
        ..window::Settings::default()
    });

    let fetch_tasks = Task::batch([fetch_weather_task(&config), fetch_forecast_task(&config)]);

    let state = AppState {
        weather: WeatherStatus::Loading,
        forecast: ForecastStatus::Loading,
        main_window,
        prefs_window: None,
        prefs_state: None,
        about_window: None,
        config,
        config_manager,
    };

    (state, Task::batch([open_task.discard(), fetch_tasks]))
}

pub fn update(state: &mut AppState, message: Message) -> Task<Message> {
    match message {
        Message::RefreshRequested | Message::Tick(_) => {
            state.weather = WeatherStatus::Loading;
            state.forecast = ForecastStatus::Loading;
            Task::batch([
                fetch_weather_task(&state.config),
                fetch_forecast_task(&state.config),
            ])
        }
        Message::WeatherFetched(Ok(response)) => {
            state.weather = WeatherStatus::Loaded(response);
            Task::none()
        }
        Message::WeatherFetched(Err(error)) => {
            state.weather = WeatherStatus::Error(error);
            Task::none()
        }
        Message::ForecastFetched(Ok(response)) => {
            log::info!(
                "Forecast loaded for {}: {} day(s)",
                response.location_name,
                response.days.len()
            );
            state.forecast = ForecastStatus::Loaded(response);
            Task::none()
        }
        Message::ForecastFetched(Err(error)) => {
            log::warn!("Forecast fetch failed: {}", error);
            state.forecast = ForecastStatus::Error;
            Task::none()
        }
        Message::OpenPreferences => {
            if state.prefs_window.is_some() {
                return Task::none();
            }
            state.prefs_state = Some(preferences::State::from_config(&state.config));
            let (id, open_task) = window::open(window::Settings {
                size: Size::new(500.0, 420.0),
                ..window::Settings::default()
            });
            state.prefs_window = Some(id);
            open_task.discard()
        }
        Message::OpenAbout => {
            if state.about_window.is_some() {
                return Task::none();
            }
            let (id, open_task) = window::open(window::Settings {
                size: Size::new(420.0, 360.0),
                resizable: false,
                ..window::Settings::default()
            });
            state.about_window = Some(id);
            open_task.discard()
        }
        Message::AnimationTick => Task::none(),
        Message::WindowCloseRequested(id) => {
            if id == state.main_window {
                return iced::exit();
            }
            if state.prefs_window == Some(id) {
                state.prefs_window = None;
                state.prefs_state = None;
                return window::close(id);
            }
            if state.about_window == Some(id) {
                state.about_window = None;
                return window::close(id);
            }
            Task::none()
        }
        Message::Preferences(preferences::Message::Save) => {
            let Some(prefs_state) = state.prefs_state.take() else {
                return Task::none();
            };
            prefs_state.apply_to(&mut state.config);
            if let Err(e) = state.config_manager.save_config(&state.config) {
                log::error!("Failed to save configuration: {}", e);
            } else {
                log::info!("Configuration saved successfully");
            }
            let close_task = state
                .prefs_window
                .take()
                .map(window::close)
                .unwrap_or_else(Task::none);
            Task::batch([close_task, Task::done(Message::RefreshRequested)])
        }
        Message::Preferences(preferences::Message::Cancel) => {
            state.prefs_state = None;
            state
                .prefs_window
                .take()
                .map(window::close)
                .unwrap_or_else(Task::none)
        }
        Message::Preferences(sub_message) => {
            if let Some(prefs_state) = state.prefs_state.as_mut() {
                preferences::update(prefs_state, sub_message);
            }
            Task::none()
        }
    }
}

pub fn view(state: &AppState, window_id: window::Id) -> Element<'_, Message> {
    if window_id == state.main_window {
        return main_screen::view(state);
    }
    if Some(window_id) == state.prefs_window
        && let Some(prefs_state) = state.prefs_state.as_ref()
    {
        return preferences::view(prefs_state).map(Message::Preferences);
    }
    if Some(window_id) == state.about_window {
        return about::view();
    }
    Space::new().into()
}

pub fn subscription(_state: &AppState) -> Subscription<Message> {
    Subscription::batch([
        iced::time::every(AUTO_REFRESH_INTERVAL).map(Message::Tick),
        iced::time::every(ANIMATION_TICK_INTERVAL).map(|_| Message::AnimationTick),
        window::close_requests().map(Message::WindowCloseRequested),
    ])
}

pub fn theme(_state: &AppState, _window: window::Id) -> Theme {
    Theme::Light
}

pub fn title(_state: &AppState, _window_id: window::Id) -> String {
    "Weather Wizard".to_string()
}

/// Runs the application. The single entry point called from `main.rs`.
pub fn run() -> iced::Result {
    iced::daemon(boot, update, view)
        .title(title)
        .theme(theme)
        .subscription(subscription)
        .run()
}
