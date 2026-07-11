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

use crate::config::{
    AppConfig, ConfigManager, LocationConfig, ThemePreference, WeatherApiProvider,
};
use crate::ui::temperature::{
    celsius_to_display, compass_direction, distance_to_display, distance_unit, format_local_time,
    pressure_to_display, pressure_unit, speed_to_display, speed_unit, unit_symbol,
};
use crate::ui::{about, main_screen, preferences, transition};
use crate::weather_api::alerts::WeatherAlert;
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
/// The height covers all three sections (Weather Provider -- including the
/// Verify API button/status row -- Home, and Appearance) plus the
/// Save/Cancel row at once, so the common case (no validation errors, no
/// first-run banner) never needs `view()`'s `scrollable` wrapper to kick in.
const PREFERENCES_WINDOW_MIN_SIZE: Size = Size::new(460.0, 640.0);
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
    pub config_manager: ConfigManager,
    pub weather: WeatherStatus,
    pub forecast: ForecastStatus,
    pub alerts: Vec<WeatherAlert>,
    /// The OS's current light/dark preference, as of the last
    /// `detect_system_theme_task` poll -- only consulted by `theme()` when
    /// `config.theme_preference` (or the live Preferences draft) is
    /// `ThemePreference::System`. Defaults to `Theme::Light` until the
    /// first detection (fired at boot) resolves.
    pub system_theme: Theme,
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
    /// Whether the Preferences window currently open (if any) was opened
    /// automatically because no config file existed at boot -- read by
    /// `title()` to swap in a welcome message, and cleared once that
    /// window's Save/Cancel resolves it (see `update`) so later manual
    /// reopens via the toolbar never show first-run copy.
    is_first_run: bool,
}

#[derive(Debug, Clone)]
pub enum Message {
    RefreshRequested,
    Tick(#[allow(dead_code)] std::time::Instant),
    WeatherFetched(Result<ApiResponse, String>),
    ForecastFetched(Result<ForecastResponse, String>),
    AlertsFetched(Result<Vec<WeatherAlert>, String>),
    /// Result of `detect_system_theme_task`, fired at boot and again on
    /// every `RefreshRequested`/`Tick` -- see that function's docs for why
    /// this is polled rather than pushed.
    SystemThemeDetected(Theme),

    OpenPreferences,
    OpenAbout,
    WindowCloseRequested(window::Id),
    AnimationTick,
    OpenUrl(String),
    /// Tapping a forecast day card. Toggles: selecting the same index again,
    /// or index `0` ("Today", redundant with the live current-conditions
    /// view), clears the selection back to live conditions.
    ForecastDaySelected(usize),
    /// Result of `crate::geolocation::detect_location`, fired by
    /// `Message::Preferences(preferences::Message::DetectLocationRequested)`.
    /// Applies to whatever Preferences window is currently open, if any --
    /// see `update`.
    LocationDetected(Result<LocationConfig, String>),
    /// Result of a single `get_weather()` call against the *currently-typed*
    /// provider/token/location, fired by
    /// `Message::Preferences(preferences::Message::TestConnectionRequested)`.
    /// Applies to whatever Preferences window is currently open, if any --
    /// see `update`. Purely informational -- never touches `state.weather`
    /// or `state.config`.
    ConnectionTested(Result<(), String>),
    /// Result of the async, off-UI-thread `AppConfig::get_api_token` read
    /// fired whenever a Preferences window opens (`OpenPreferences`, and
    /// `boot`'s first-run path) -- see `preferences::State::from_config`'s
    /// docs for why the read isn't done synchronously up front. Applies to
    /// whatever Preferences window is currently open, if any; a no-op if
    /// it's already been closed by the time this resolves.
    ApiTokenLoaded(String),
    /// A location switcher pill was clicked (`ui::location_switcher`) --
    /// switches `config.current_location_index` and persists it
    /// immediately, independent of Preferences' Save/Cancel, then
    /// re-fetches for the newly-current location. Unlike a same-location
    /// refresh, the previous location's data is discarded outright (back to
    /// `Loading`, not `Refreshing`) rather than shown while the new fetch
    /// is in flight -- it belongs to a different place, so carrying it
    /// forward would misleadingly look current.
    LocationSwitched(usize),

    Preferences(preferences::Message),
}

/// Builds a `Task` that fetches current weather for the active provider/location.
///
/// `AppConfig::get_api_token` is a blocking OS keychain read (and, on macOS,
/// can pop a permission prompt the *first* time a given build accesses it,
/// or every time if the user picked "Allow" over "Always Allow") -- calling
/// it before entering the `async move` block would run it synchronously on
/// `update()`'s own thread, freezing the whole UI (including button clicks)
/// until that prompt is dismissed. Reading it inside the async block instead
/// keeps it on iced's executor, off the UI thread.
fn fetch_weather_task(config: &AppConfig) -> Task<Message> {
    let provider_type = config.weather_provider.clone();
    let location = config.current_location();
    let config = config.clone();

    Task::perform(
        async move {
            let token = config.get_api_token().ok();
            let provider =
                WeatherProviderFactory::create_provider(&provider_type, token, config.language)?;
            provider
                .get_weather(&location)
                .await
                .map_err(|e| format!("{:?}", e))
        },
        Message::WeatherFetched,
    )
}

/// Builds a `Task` that fetches a forecast for the active provider/location.
/// See `fetch_weather_task`'s docs for why the token is read inside the
/// async block rather than before it.
fn fetch_forecast_task(config: &AppConfig) -> Task<Message> {
    let provider_type = config.weather_provider.clone();
    let location = config.current_location();
    let config = config.clone();

    Task::perform(
        async move {
            let token = config.get_api_token().ok();
            let provider =
                WeatherProviderFactory::create_provider(&provider_type, token, config.language)?;
            provider
                .get_forecast(&location)
                .await
                .map_err(|e| format!("{:?}", e))
        },
        Message::ForecastFetched,
    )
}

/// Builds a `Task` that fetches active weather alerts.
fn fetch_alerts_task(config: &AppConfig) -> Task<Message> {
    let provider_type = config.weather_provider.clone();
    let location = config.current_location();
    let config = config.clone();

    Task::perform(
        async move {
            let token = config.get_api_token().ok();
            let provider =
                WeatherProviderFactory::create_provider(&provider_type, token, config.language)?;
            provider
                .get_alerts(&location)
                .await
                .map_err(|e| format!("{:?}", e))
        },
        Message::AlertsFetched,
    )
}

/// Builds a `Task` that polls the OS's current light/dark preference off
/// the UI thread. `dark_light::detect()` is a blocking call (on Linux, a
/// D-Bus round trip to the XDG Desktop Portal, bounded by the crate's own
/// 25ms timeout) -- calling it directly from `theme()` would run it
/// synchronously every time iced redraws (including on every
/// `AnimationTick`, ~30 times a second), freezing rendering. Instead this
/// runs on the same cadence as the weather refresh (`RefreshRequested`/
/// `Tick`, plus once at boot) and caches the result in
/// `AppState::system_theme`, which `theme()` only reads. This means an OS
/// theme change while the app is running is picked up on the next refresh
/// tick, not instantly -- `dark-light` has no event/subscription API to
/// react to it immediately.
fn detect_system_theme_task() -> Task<Message> {
    Task::perform(
        async {
            match dark_light::detect() {
                Ok(dark_light::Mode::Dark) => Theme::Dark,
                Ok(dark_light::Mode::Light | dark_light::Mode::Unspecified) | Err(_) => {
                    Theme::Light
                }
            }
        },
        Message::SystemThemeDetected,
    )
}

/// Builds a `Task` that reads the API token off the UI thread and reports
/// it back via `Message::ApiTokenLoaded` -- see
/// `preferences::State::from_config`'s docs for why Preferences opens with
/// an empty token field rather than reading it synchronously up front.
fn fetch_api_token_task(config: &AppConfig) -> Task<Message> {
    let config = config.clone();
    Task::perform(
        async move { config.get_api_token().unwrap_or_default() },
        Message::ApiTokenLoaded,
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
    let pressure = pressure_to_display(response.main.pressure, use_fahrenheit);
    let pressure_unit_str = pressure_unit(use_fahrenheit);
    let pressure_precision = if use_fahrenheit { 2 } else { 0 };
    let sunrise = format_local_time(response.sys.sunrise, response.timezone);
    let sunset = format_local_time(response.sys.sunset, response.timezone);

    tracker.note("temp", &format!("{:.1}{unit}", temp));
    tracker.note("feels_like", &format!("{:.0}{unit}", feels_like));
    tracker.note("humidity", &format!("{}%", response.main.humidity));
    tracker.note("wind", &format!("{:.0} {wind_unit} {compass}", wind_speed));
    tracker.note(
        "pressure",
        &format!("{:.*} {pressure_unit_str}", pressure_precision, pressure),
    );
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

/// The Preferences window's fixed size/icon -- shared by `boot`'s first-run
/// auto-open and `Message::OpenPreferences`'s manual one, so the two open
/// paths can never drift apart.
fn preferences_window_settings() -> window::Settings {
    window::Settings {
        size: Size::new(540.0, 700.0),
        min_size: Some(PREFERENCES_WINDOW_MIN_SIZE),
        icon: crate::ui::icons::load_window_icon("icon/icon.png"),
        ..window::Settings::default()
    }
}

/// Boots the application: loads config, opens the main window, and kicks off the
/// first weather + forecast fetch -- unless no config file existed yet (a
/// fresh install), in which case Preferences opens automatically instead of
/// firing a fetch that's guaranteed to fail against `AppConfig::default()`'s
/// unset API token (see `docs`/issue #38 for why: `WeatherProviderFactory`
/// requires a token for every provider now, and the default config has
/// none).
pub fn boot() -> (AppState, Task<Message>) {
    let config_manager = ConfigManager::new().expect("Failed to create config manager");
    let is_first_run = !config_manager.config_exists();
    let config = config_manager.load_config();

    let (main_window, main_open_task) = window::open(window::Settings {
        size: Size::new(DEFAULT_WINDOW_WIDTH, DEFAULT_WINDOW_HEIGHT),
        min_size: Some(MAIN_WINDOW_MIN_SIZE),
        icon: crate::ui::icons::load_window_icon("icon/icon.png"),
        ..window::Settings::default()
    });

    let (prefs_window, prefs_state, second_task) = if is_first_run {
        let mut prefs_state = preferences::State::from_config(&config);
        prefs_state.is_first_run = true;
        let (id, prefs_open_task) = window::open(preferences_window_settings());
        (
            Some(id),
            Some(prefs_state),
            Task::batch([prefs_open_task.discard(), fetch_api_token_task(&config)]),
        )
    } else {
        (
            None,
            None,
            Task::batch([
                fetch_weather_task(&config),
                fetch_forecast_task(&config),
                fetch_alerts_task(&config),
            ]),
        )
    };

    let state = AppState {
        weather: WeatherStatus::Loading,
        forecast: ForecastStatus::Loading,
        alerts: vec![],
        system_theme: Theme::Light,
        last_updated: None,
        value_tracker: transition::ValueTracker::default(),
        selected_forecast_day: None,
        main_window,
        prefs_window,
        prefs_state,
        about_window: None,
        is_first_run,
        config,
        config_manager,
    };

    (
        state,
        Task::batch([
            main_open_task.discard(),
            second_task,
            detect_system_theme_task(),
        ]),
    )
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
                fetch_alerts_task(&state.config),
                detect_system_theme_task(),
            ])
        }
        Message::LocationSwitched(index) => {
            if index >= state.config.locations.len() || index == state.config.current_location_index
            {
                return Task::none();
            }
            state.config.current_location_index = index;
            if let Err(e) = state.config_manager.save_config(&state.config) {
                log::warn!("Failed to persist location switch: {}", e);
            }
            // Unlike `RefreshRequested`/`Tick`, drop straight to `Loading`
            // rather than `Refreshing` -- the previous location's data
            // belongs to a different place entirely, not a stale copy of
            // the same one, so carrying it forward would look current when
            // it isn't.
            state.weather = WeatherStatus::Loading;
            state.forecast = ForecastStatus::Loading;
            state.alerts = vec![];
            state.selected_forecast_day = None;
            state.last_updated = None;
            Task::batch([
                fetch_weather_task(&state.config),
                fetch_forecast_task(&state.config),
                fetch_alerts_task(&state.config),
            ])
        }
        Message::SystemThemeDetected(theme) => {
            state.system_theme = theme;
            Task::none()
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
        Message::AlertsFetched(Ok(alerts)) => {
            state.alerts = alerts;
            Task::none()
        }
        Message::AlertsFetched(Err(error)) => {
            log::warn!("Alerts fetch failed: {error}");
            // We retain existing alerts on failure, or could clear them. Keeping them for now.
            Task::none()
        }
        Message::OpenPreferences => {
            if state.prefs_window.is_some() {
                return Task::none();
            }
            state.prefs_state = Some(preferences::State::from_config(&state.config));
            let (id, open_task) = window::open(preferences_window_settings());
            state.prefs_window = Some(id);
            Task::batch([open_task.discard(), fetch_api_token_task(&state.config)])
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
                state.is_first_run = false;
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
            // Whatever brought up first-run setup is now resolved -- a
            // later manual reopen (toolbar gear icon) should show the
            // ordinary "Preferences" copy, not the welcome banner again.
            state.is_first_run = false;
            let close_task = state
                .prefs_window
                .take()
                .map(window::close)
                .unwrap_or_else(Task::none);
            Task::batch([close_task, Task::done(Message::RefreshRequested)])
        }
        Message::Preferences(preferences::Message::Cancel) => {
            state.prefs_state = None;
            state.is_first_run = false;
            state
                .prefs_window
                .take()
                .map(window::close)
                .unwrap_or_else(Task::none)
        }
        Message::Preferences(preferences::Message::OpenUrl(url)) => {
            Task::done(Message::OpenUrl(url))
        }
        Message::Preferences(preferences::Message::DetectLocationRequested) => {
            let Some(prefs_state) = state.prefs_state.as_mut() else {
                return Task::none();
            };
            prefs_state.is_detecting_location = true;
            prefs_state.location_detection_error = None;
            Task::perform(
                crate::geolocation::detect_location(),
                Message::LocationDetected,
            )
        }
        Message::LocationDetected(result) => {
            // The user may have already closed Preferences (or it was never
            // open outside first-run) by the time this async lookup
            // resolves -- nothing to apply the result to in that case.
            let Some(prefs_state) = state.prefs_state.as_mut() else {
                return Task::none();
            };
            prefs_state.is_detecting_location = false;
            match result {
                Ok(location) => {
                    // Fills in the entry currently selected in the
                    // Locations tab strip, not necessarily index 0 --
                    // detection is meant to prefill whichever saved
                    // location the user is editing.
                    if let Some(entry) = prefs_state
                        .locations
                        .get_mut(prefs_state.selected_location_index)
                    {
                        entry.city = location.city;
                        entry.state = location.state;
                        entry.country = location.country;
                    }
                }
                Err(e) => {
                    log::warn!("Location detection failed: {e}");
                    prefs_state.location_detection_error = Some(
                        "Couldn't detect your location automatically -- enter it manually."
                            .to_string(),
                    );
                }
            }
            Task::none()
        }
        Message::ApiTokenLoaded(token) => {
            // Same reasoning as `LocationDetected`: Preferences may already
            // be closed by the time this async keychain read resolves.
            if let Some(prefs_state) = state.prefs_state.as_mut() {
                prefs_state.token_input = token;
            }
            Task::none()
        }
        Message::Preferences(preferences::Message::TestConnectionRequested) => {
            let Some(prefs_state) = state.prefs_state.as_mut() else {
                return Task::none();
            };
            prefs_state.is_testing_connection = true;
            prefs_state.connection_test_result = None;

            let provider_type = prefs_state.provider.clone();
            let token =
                (!prefs_state.token_input.is_empty()).then(|| prefs_state.token_input.clone());
            // Tests against whichever entry is currently selected in the
            // Locations tab strip -- "currently-typed", same philosophy as
            // provider/token above, just per-entry now.
            let selected_location = &prefs_state.locations[prefs_state.selected_location_index];
            let location = LocationConfig {
                city: selected_location.city.clone(),
                state: selected_location.state.clone(),
                country: selected_location.country.clone(),
            };
            let language = prefs_state.language;

            Task::perform(
                async move {
                    let provider =
                        WeatherProviderFactory::create_provider(&provider_type, token, language)?;
                    provider
                        .get_weather(&location)
                        .await
                        .map(|_| ())
                        .map_err(|e| format!("{:?}", e))
                },
                Message::ConnectionTested,
            )
        }
        Message::ConnectionTested(result) => {
            // Same reasoning as `LocationDetected`: Preferences may already
            // be closed by the time this async call resolves.
            let Some(prefs_state) = state.prefs_state.as_mut() else {
                return Task::none();
            };
            prefs_state.is_testing_connection = false;
            if let Err(e) = &result {
                log::warn!("Connection test failed: {e}");
            }
            prefs_state.connection_test_result = Some(result);
            Task::none()
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
    let refresh_interval = match state.config.refresh_interval_secs {
        Some(secs) => {
            let duration = Duration::from_secs(secs);
            if state.config.weather_provider == WeatherApiProvider::GoogleWeather {
                duration.max(GOOGLE_WEATHER_REFRESH_INTERVAL)
            } else {
                duration
            }
        }
        None => match state.config.weather_provider {
            WeatherApiProvider::GoogleWeather => GOOGLE_WEATHER_REFRESH_INTERVAL,
            WeatherApiProvider::OpenWeather => AUTO_REFRESH_INTERVAL,
        },
    };
    Subscription::batch([
        iced::time::every(refresh_interval).map(Message::Tick),
        iced::time::every(ANIMATION_TICK_INTERVAL).map(|_| Message::AnimationTick),
        window::close_requests().map(Message::WindowCloseRequested),
    ])
}

pub fn theme(state: &AppState, _window: window::Id) -> Theme {
    // Preview the choice live, across every window, as soon as it's changed
    // in Preferences -- not just after Save. `prefs_state` is a draft;
    // falling back to the persisted config when no Preferences window is
    // open (or once it's closed via Cancel) means an unsaved/discarded
    // change doesn't leave the theme changed behind it.
    let preference = state
        .prefs_state
        .as_ref()
        .map_or(state.config.theme_preference, |prefs| {
            prefs.theme_preference
        });

    match preference {
        ThemePreference::Light => Theme::Light,
        ThemePreference::Dark => Theme::Dark,
        ThemePreference::System => state.system_theme.clone(),
    }
}

pub fn title(state: &AppState, window_id: window::Id) -> String {
    if Some(window_id) == state.prefs_window {
        if state.is_first_run {
            "Welcome to Weather Wizard".to_string()
        } else {
            "Preferences".to_string()
        }
    } else if Some(window_id) == state.about_window {
        "About Weather Wizard".to_string()
    } else if state.config.locations.len() > 1 {
        // Only worth naming which location once there's more than one --
        // otherwise it's just noise repeating what the single "Home" entry
        // already says everywhere else.
        format!("Weather Wizard — {}", state.config.current_location_name())
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
