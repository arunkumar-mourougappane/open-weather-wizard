//! # Application State and Update Logic
//!
//! This is the iced root of the application: the `AppState`/`Message` pair, and the
//! `boot`/`update`/`view`/`subscription` functions wired together by `run()`.
//!
//! The app is a `daemon` (multi-window) rather than a single-window `application`,
//! since Preferences and About are rendered as separate OS windows, closer to the
//! transient-window feel of the previous GTK version than an in-app overlay.

use std::time::{Duration, Instant};

use iced::widget::Space;
use iced::{Element, Size, Subscription, Task, Theme, window};

use crate::config::{AppConfig, ConfigManager, WeatherApiProvider};
use crate::ui::temperature::{
    celsius_to_display, compass_direction, distance_to_display, distance_unit, format_local_time,
    speed_to_display, speed_unit, unit_symbol,
};
use crate::ui::{about, main_screen, preferences, transition};
use crate::weather_api::forecast::ForecastResponse;
use crate::weather_api::openweather_api::ApiResponse;
use crate::weather_api::weather_provider::WeatherProviderFactory;

pub const DEFAULT_WINDOW_WIDTH: f32 = 720.0;
/// Tall enough that the toolbar + current-conditions panel (icon, location,
/// temp, description, humidity) + forecast row all fit without the main
/// screen's `scrollable` wrapper needing to kick in, at the default size.
pub const DEFAULT_WINDOW_HEIGHT: f32 = 620.0;
/// Below this the content no longer fits without scrolling -- which
/// `main_screen::view`'s `scrollable` wrapper now handles gracefully, so
/// this floor exists for comfort (avoid *always* needing to scroll at the
/// minimum size) rather than to prevent distortion, since fixed-size
/// widgets like the animated icons can no longer get silently squeezed.
const MAIN_WINDOW_MIN_SIZE: Size = Size::new(480.0, 420.0);
/// The preferences form's fixed 160px label column plus a usable input
/// width needs at least this much room before fields start getting crushed.
const PREFERENCES_WINDOW_MIN_SIZE: Size = Size::new(440.0, 400.0);
const AUTO_REFRESH_INTERVAL: Duration = Duration::from_secs(30);
/// A full refresh against the real Google Weather API costs 3 billable
/// calls (`currentConditions:lookup` + two `forecast/days:lookup` calls --
/// see `docs/GOOGLE_WEATHER_API.md`). At 15 minutes that's ~8,640 calls/month
/// for one always-open instance, comfortably under Google's 10,000/month
/// free tier; `AUTO_REFRESH_INTERVAL`'s 30s would blow through it in about a
/// day.
const GOOGLE_WEATHER_REFRESH_INTERVAL: Duration = Duration::from_secs(15 * 60);
/// Drives redraws for the animated Lottie icons (~30fps); `icons::view`
/// computes each frame from wall-clock time, so this tick carries no state of
/// its own -- it exists purely to make iced re-invoke `view()` regularly.
const ANIMATION_TICK_INTERVAL: Duration = Duration::from_millis(33);

/// The lifecycle of an async weather fetch, driving the main screen's display.
///
/// `Refreshing` carries the last-known-good data forward while a background
/// fetch (auto-refresh or the manual Refresh button) is in flight, so the UI
/// never blanks back to a loading state once it has real data to show --
/// `Loading` is reachable only before the very first successful fetch (or
/// after a first-load `Error`, on retry). `view()` treats `Loaded` and
/// `Refreshing` identically; the distinction exists purely for `update()`.
#[derive(Debug, Clone)]
pub enum WeatherStatus {
    Loading,
    Loaded(ApiResponse),
    Refreshing(ApiResponse),
    Error(String),
}

impl WeatherStatus {
    /// The data to render, whether idle or mid-refresh -- `None` while
    /// loading for the first time or after a first-load error.
    pub fn data(&self) -> Option<&ApiResponse> {
        match self {
            WeatherStatus::Loaded(data) | WeatherStatus::Refreshing(data) => Some(data),
            WeatherStatus::Loading | WeatherStatus::Error(_) => None,
        }
    }
}

/// The lifecycle of an async forecast fetch. Kept separate from `WeatherStatus`
/// so a forecast failure (or an empty result from a provider like Google Weather,
/// which has no real forecast integration) never blanks out current conditions.
///
/// Unlike `WeatherStatus::Error`, this carries no message: a forecast failure is
/// never surfaced in the UI (the row is simply omitted), so the failure reason is
/// only logged at the point of transition in `update()`. `Refreshing` mirrors
/// `WeatherStatus::Refreshing` -- see its docs for why the split exists.
#[derive(Debug, Clone)]
pub enum ForecastStatus {
    Loading,
    Loaded(ForecastResponse),
    Refreshing(ForecastResponse),
    Error,
}

impl ForecastStatus {
    pub fn data(&self) -> Option<&ForecastResponse> {
        match self {
            ForecastStatus::Loaded(data) | ForecastStatus::Refreshing(data) => Some(data),
            ForecastStatus::Loading | ForecastStatus::Error => None,
        }
    }
}

/// Top-level application state, owned directly (no `Arc<Mutex<>>`): iced's Elm
/// architecture already serializes every mutation through `update()`, so the
/// shared-closure problem the GTK version solved with a mutex doesn't exist here.
pub struct AppState {
    pub config: AppConfig,
    config_manager: ConfigManager,
    pub weather: WeatherStatus,
    pub forecast: ForecastStatus,
    /// When `weather` last transitioned to `Loaded`, for the "Updated Xs ago"
    /// label. `main_screen` re-renders often enough (via `AnimationTick`,
    /// already needed for the animated icons) that this stays fresh without
    /// its own timer.
    pub last_updated: Option<Instant>,
    /// Drives the per-value cross-fade when a tracked field's freshly
    /// fetched value differs from what was last displayed -- see
    /// `ui::transition`. Noted in `update()` on fetch success, read (never
    /// mutated) by view code.
    pub value_tracker: transition::ValueTracker,
    /// Index into `forecast`'s days when the main card is showing that
    /// day's detail instead of live current conditions. `None` (the
    /// default) shows live conditions. See `Message::ForecastDaySelected`.
    pub selected_forecast_day: Option<usize>,
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
    OpenUrl(String),
    /// Tapping a forecast day card. Toggles: selecting the same index again,
    /// or index `0` ("Today", redundant with the live current-conditions
    /// view), clears the selection back to live conditions.
    ForecastDaySelected(usize),

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

/// Records the freshly-formatted display value for each cross-faded
/// current-conditions field -- `ui::main_screen`'s `hero_view`/`stats_view`
/// read these same keys back via `ValueTracker::cross_fade`. Must be called
/// with the *new* `response` before it overwrites `state.weather`, so the
/// tracker can diff against whatever was noted last time.
fn note_weather_transitions(
    tracker: &mut transition::ValueTracker,
    response: &ApiResponse,
    use_fahrenheit: bool,
) {
    let unit = unit_symbol(use_fahrenheit);
    let temp = celsius_to_display(response.main.temp, use_fahrenheit);
    let feels_like = celsius_to_display(response.main.feels_like, use_fahrenheit);
    let temp_min = celsius_to_display(response.main.temp_min, use_fahrenheit);
    let temp_max = celsius_to_display(response.main.temp_max, use_fahrenheit);
    let wind_speed = speed_to_display(response.wind.speed, use_fahrenheit);
    let wind_unit = speed_unit(use_fahrenheit);
    let compass = compass_direction(response.wind.deg);
    let visibility = distance_to_display(response.visibility as f64, use_fahrenheit);
    let visibility_unit = distance_unit(use_fahrenheit);
    let sunrise = format_local_time(response.sys.sunrise, response.timezone);
    let sunset = format_local_time(response.sys.sunset, response.timezone);

    tracker.note("temp", &format!("{:.1}{unit}", temp));
    tracker.note("feels_like", &format!("{:.0}{unit}", feels_like));
    tracker.note("humidity", &format!("{}%", response.main.humidity));
    tracker.note("wind", &format!("{:.0} {wind_unit} {compass}", wind_speed));
    tracker.note("pressure", &format!("{} hPa", response.main.pressure));
    tracker.note(
        "visibility",
        &format!("{:.1} {visibility_unit}", visibility),
    );
    tracker.note(
        "high_low",
        &format!("{:.0}{unit} / {:.0}{unit}", temp_max, temp_min),
    );
    tracker.note("sunrise", &sunrise);
    tracker.note("sunset", &sunset);
}

/// Same idea as `note_weather_transitions`, for each forecast day's hi/lo
/// and description -- `forecast_row::day_card` reads these back keyed by
/// the day's index.
fn note_forecast_transitions(
    tracker: &mut transition::ValueTracker,
    response: &ForecastResponse,
    use_fahrenheit: bool,
) {
    let unit = unit_symbol(use_fahrenheit);
    for (index, day) in response.days.iter().enumerate() {
        let temp_max = celsius_to_display(day.temp_max, use_fahrenheit);
        let temp_min = celsius_to_display(day.temp_min, use_fahrenheit);
        tracker.note(
            &format!("forecast_{index}_hilo"),
            &format!("{:.0}{unit} / {:.0}{unit}", temp_max, temp_min),
        );
        tracker.note(&format!("forecast_{index}_desc"), &day.description);
    }
}

/// Boots the application: loads config, opens the main window, and kicks off the
/// first weather + forecast fetch.
pub fn boot() -> (AppState, Task<Message>) {
    let config_manager = ConfigManager::new().expect("Failed to create config manager");
    let config = config_manager.load_config();

    let (main_window, open_task) = window::open(window::Settings {
        size: Size::new(DEFAULT_WINDOW_WIDTH, DEFAULT_WINDOW_HEIGHT),
        min_size: Some(MAIN_WINDOW_MIN_SIZE),
        icon: crate::ui::icons::load_window_icon("icon/icon.png"),
        ..window::Settings::default()
    });

    let fetch_tasks = Task::batch([fetch_weather_task(&config), fetch_forecast_task(&config)]);

    let state = AppState {
        weather: WeatherStatus::Loading,
        forecast: ForecastStatus::Loading,
        last_updated: None,
        value_tracker: transition::ValueTracker::default(),
        selected_forecast_day: None,
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
            // Keep showing last-known-good data during the fetch instead of
            // blanking to a loading state -- only the very first fetch (or a
            // retry after a first-load error) has nothing to show yet.
            state.weather = match std::mem::replace(&mut state.weather, WeatherStatus::Loading) {
                WeatherStatus::Loaded(data) | WeatherStatus::Refreshing(data) => {
                    WeatherStatus::Refreshing(data)
                }
                other => other,
            };
            state.forecast = match std::mem::replace(&mut state.forecast, ForecastStatus::Loading) {
                ForecastStatus::Loaded(data) | ForecastStatus::Refreshing(data) => {
                    ForecastStatus::Refreshing(data)
                }
                other => other,
            };
            Task::batch([
                fetch_weather_task(&state.config),
                fetch_forecast_task(&state.config),
            ])
        }
        Message::WeatherFetched(Ok(response)) => {
            note_weather_transitions(
                &mut state.value_tracker,
                &response,
                state.config.use_fahrenheit,
            );
            state.weather = WeatherStatus::Loaded(response);
            state.last_updated = Some(Instant::now());
            Task::none()
        }
        Message::WeatherFetched(Err(error)) => {
            // A failed background refresh shouldn't disrupt a screen that
            // already has good data -- only surface the error if we had
            // nothing to show in the first place.
            state.weather = match std::mem::replace(&mut state.weather, WeatherStatus::Loading) {
                WeatherStatus::Refreshing(data) => {
                    log::warn!("Background weather refresh failed, keeping last data: {error}");
                    WeatherStatus::Loaded(data)
                }
                _ => WeatherStatus::Error(error),
            };
            Task::none()
        }
        Message::ForecastFetched(Ok(response)) => {
            log::info!(
                "Forecast loaded for {}: {} day(s)",
                response.location_name,
                response.days.len()
            );
            note_forecast_transitions(
                &mut state.value_tracker,
                &response,
                state.config.use_fahrenheit,
            );
            // A stale selection (e.g. a provider that now returns fewer
            // days) would otherwise panic-free but silently index into
            // nothing meaningful -- clear it back to the live view instead.
            if let Some(index) = state.selected_forecast_day
                && index >= response.days.len()
            {
                state.selected_forecast_day = None;
            }
            state.forecast = ForecastStatus::Loaded(response);
            Task::none()
        }
        Message::ForecastFetched(Err(error)) => {
            state.forecast = match std::mem::replace(&mut state.forecast, ForecastStatus::Loading) {
                ForecastStatus::Refreshing(data) => {
                    log::warn!("Background forecast refresh failed, keeping last data: {error}");
                    ForecastStatus::Loaded(data)
                }
                _ => {
                    log::warn!("Forecast fetch failed: {error}");
                    ForecastStatus::Error
                }
            };
            Task::none()
        }
        Message::OpenPreferences => {
            if state.prefs_window.is_some() {
                return Task::none();
            }
            state.prefs_state = Some(preferences::State::from_config(&state.config));
            let (id, open_task) = window::open(window::Settings {
                size: Size::new(520.0, 560.0),
                min_size: Some(PREFERENCES_WINDOW_MIN_SIZE),
                icon: crate::ui::icons::load_window_icon("icon/icon.png"),
                ..window::Settings::default()
            });
            state.prefs_window = Some(id);
            open_task.discard()
        }
        Message::OpenAbout => {
            if state.about_window.is_some() {
                return Task::none();
            }
            const ABOUT_SIZE: Size = Size::new(420.0, 440.0);
            let (id, open_task) = window::open(window::Settings {
                size: ABOUT_SIZE,
                min_size: Some(ABOUT_SIZE),
                max_size: Some(ABOUT_SIZE),
                resizable: false,
                icon: crate::ui::icons::load_window_icon("icon/icon.png"),
                ..window::Settings::default()
            });
            state.about_window = Some(id);
            open_task.discard()
        }
        Message::AnimationTick => Task::none(),
        Message::ForecastDaySelected(index) => {
            state.selected_forecast_day =
                if index == 0 || state.selected_forecast_day == Some(index) {
                    None
                } else {
                    Some(index)
                };
            Task::none()
        }
        Message::OpenUrl(url) => {
            if let Err(e) = open::that(&url) {
                log::warn!("Failed to open URL {url}: {e}");
            }
            Task::none()
        }
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
            if let Err(e) = prefs_state.apply_to(&mut state.config) {
                // A keychain write failed (locked keychain, no Secret
                // Service running, etc.) -- put the form state back and
                // leave the window open rather than closing it and silently
                // discarding the token the user just typed in.
                log::error!("Failed to save API token: {e}");
                state.prefs_state = Some(prefs_state);
                return Task::none();
            }
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

pub fn subscription(state: &AppState) -> Subscription<Message> {
    let refresh_interval = match state.config.weather_provider {
        WeatherApiProvider::GoogleWeather => GOOGLE_WEATHER_REFRESH_INTERVAL,
        WeatherApiProvider::OpenWeather => AUTO_REFRESH_INTERVAL,
    };
    Subscription::batch([
        iced::time::every(refresh_interval).map(Message::Tick),
        iced::time::every(ANIMATION_TICK_INTERVAL).map(|_| Message::AnimationTick),
        window::close_requests().map(Message::WindowCloseRequested),
    ])
}

pub fn theme(state: &AppState, _window: window::Id) -> Theme {
    // Preview the toggle live, across every window, as soon as it's
    // flipped in Preferences -- not just after Save. `prefs_state` is a
    // draft; falling back to the persisted config when no Preferences
    // window is open (or once it's closed via Cancel) means an
    // unsaved/discarded toggle doesn't leave the theme changed behind it.
    let dark_mode = state
        .prefs_state
        .as_ref()
        .map_or(state.config.dark_mode, |prefs| prefs.dark_mode);

    if dark_mode { Theme::Dark } else { Theme::Light }
}

pub fn title(state: &AppState, window_id: window::Id) -> String {
    if Some(window_id) == state.prefs_window {
        "Preferences".to_string()
    } else if Some(window_id) == state.about_window {
        "About Weather Wizard".to_string()
    } else {
        "Weather Wizard".to_string()
    }
}

/// Runs the application. The single entry point called from `main.rs`.
pub fn run() -> iced::Result {
    iced::daemon(boot, update, view)
        .title(title)
        .theme(theme)
        .subscription(subscription)
        .run()
}
