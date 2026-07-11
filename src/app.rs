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
use crate::ui::{about, icons, main_screen, preferences, transition};
use crate::weather_api::alerts::WeatherAlert;
use crate::weather_api::forecast::ForecastResponse;
use crate::weather_api::openweather_api::ApiResponse;
use crate::weather_api::weather_provider::WeatherProviderFactory;
use tray::{MouseButton, MouseButtonState, TrayIcon, TrayIconBuilder, TrayIconEvent};

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
    /// The persistent tray/menu bar icon (issue #56) -- `None` if creation
    /// failed (logged as a warning in `boot()`), so a platform hiccup here
    /// degrades to "no tray icon" rather than crashing the whole app.
    /// Kept alive for the app's lifetime purely by staying a field here --
    /// the underlying OS resource is torn down when this drops, so nothing
    /// ever reads it back out, just holds onto it and refreshes its
    /// tooltip. See `sync_tray_display` and `Message::AnimationTick`'s
    /// handler (which drains `TrayIconEvent::receiver()`).
    tray_icon: Option<TrayIcon>,
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

/// Restores an existing window to the foreground -- un-minimizing it first
/// if necessary. `gain_focus` alone documents itself as a no-op if the
/// window is minimized, which is exactly the state a user re-clicking a
/// toolbar button (Preferences, About) or the tray icon for an
/// already-open window is most likely trying to recover from. Naively
/// batching `minimize(id, false)` and `gain_focus(id)` together doesn't
/// work either: winit's own `focus_window` (what `gain_focus` calls on
/// macOS) checks `isMiniaturized()` and no-ops if still true, and
/// `deminiaturize`'s un-minimize animation hasn't necessarily finished by
/// the time both actions dispatch in the same tick, so `gain_focus` would
/// silently do nothing almost every time. Querying the actual state first
/// and issuing only the one relevant action avoids the race outright:
/// un-minimizing alone already restores focus (same as clicking a
/// minimized window's Dock icon), so `gain_focus` is only needed when the
/// window was merely unfocused (behind other windows), never minimized.
fn bring_window_to_front(id: window::Id) -> Task<Message> {
    window::is_minimized(id).then(move |minimized| {
        if minimized == Some(true) {
            window::minimize(id, false)
        } else {
            window::gain_focus(id)
        }
    })
}

/// Drops any last-known-good weather/forecast/alerts data rather than
/// letting it carry forward through the next fetch's `Refreshing` state --
/// for use whenever what's about to be fetched is for a *different place*
/// than what's currently displayed (a location switch, or a Preferences
/// Save that changed which location is current), where showing the old
/// data while the new fetch is in flight would misleadingly look like a
/// same-place refresh instead of a different place's page still loading.
/// Ordinary same-place refreshes (`Message::RefreshRequested`/`Tick`) don't
/// call this -- they're the one case `Refreshing` exists for.
fn discard_stale_location_data(state: &mut AppState) {
    state.weather = WeatherStatus::Loading;
    state.forecast = ForecastStatus::Loading;
    state.alerts = vec![];
    state.selected_forecast_day = None;
    state.last_updated = None;
    sync_tray_display(state);
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
fn build_tray_icon() -> Option<TrayIcon> {
    let icon = icons::load_tray_icon("icon/iconset/icon-32.png")?;
    match TrayIconBuilder::new()
        .with_tooltip("Weather Wizard")
        // A "template" image lets macOS recolor it to match the current
        // menu bar appearance (light/dark), the same way every other
        // monochrome menu bar glyph behaves -- without this, our icon
        // stays a fixed color regardless of the system's appearance, which
        // can read as low-contrast or just visually inconsistent with
        // everything else up there. No effect on Windows/Linux.
        .with_icon_as_template(true)
        .with_icon(icon)
        .build()
    {
        Ok(tray_icon) => Some(tray_icon),
        Err(e) => {
            log::warn!("Failed to create tray icon: {e}");
            None
        }
    }
}

/// Builds the tray icon's tooltip text from the current weather status --
/// a pure function (unlike `sync_tray_display`, which also has to reach
/// into `state.tray_icon` and make the actual OS call) purely so the text
/// itself is unit-testable without needing a real `TrayIcon`.
fn tray_tooltip_text(weather: &WeatherStatus, use_fahrenheit: bool) -> String {
    match weather {
        WeatherStatus::Loaded(response) | WeatherStatus::Refreshing(response) => {
            match response.weather.first() {
                Some(condition) => format!(
                    "Weather Wizard — {:.0}{} {}",
                    celsius_to_display(response.main.temp, use_fahrenheit),
                    unit_symbol(use_fahrenheit),
                    condition.description
                ),
                None => "Weather Wizard".to_string(),
            }
        }
        WeatherStatus::Loading => "Weather Wizard — Loading…".to_string(),
        WeatherStatus::Error(_) => "Weather Wizard — couldn't fetch weather".to_string(),
    }
}

/// Builds the short text shown next to the tray icon itself (macOS only,
/// via `TrayIcon::set_title`) -- unlike the tooltip (a full sentence, only
/// visible on hover), this is always on display, so it stays as compact as
/// the system's own menu bar weather widget ("78°F"). `None` while loading
/// or after a fetch error, rather than some placeholder text, so a blank
/// title doesn't crowd the icon with nothing useful to say.
fn tray_title_text(weather: &WeatherStatus, use_fahrenheit: bool) -> Option<String> {
    match weather {
        WeatherStatus::Loaded(response) | WeatherStatus::Refreshing(response) => Some(format!(
            "{:.0}{}",
            celsius_to_display(response.main.temp, use_fahrenheit),
            unit_symbol(use_fahrenheit)
        )),
        WeatherStatus::Loading | WeatherStatus::Error(_) => None,
    }
}

/// Refreshes the tray icon's title and tooltip from `state.weather`/
/// `state.config.use_fahrenheit` -- called wherever either one changes:
/// `WeatherFetched`, `discard_stale_location_data` (a location switch or a
/// Preferences Save that changed the current location), and Preferences
/// Save generally (`use_fahrenheit` might have changed with no location
/// change at all).
fn sync_tray_display(state: &AppState) {
    let Some(tray_icon) = &state.tray_icon else {
        return;
    };
    let tooltip = tray_tooltip_text(&state.weather, state.config.use_fahrenheit);
    if let Err(e) = tray_icon.set_tooltip(Some(tooltip)) {
        log::warn!("Failed to update tray icon tooltip: {e}");
    }
    let title = tray_title_text(&state.weather, state.config.use_fahrenheit);
    tray_icon.set_title(title.as_deref());
}

pub fn boot() -> (AppState, Task<Message>) {
    // `window::Settings::icon` below is a no-op on macOS (see
    // `icons::set_dock_icon_macos`'s docs) -- this is what actually gets a
    // correct Dock icon for a bare `cargo run`/`cargo build` dev binary
    // rather than a generic executable icon. Packaged release builds get
    // theirs from `packaging/macos/Info.plist`'s `.icns` regardless.
    #[cfg(target_os = "macos")]
    icons::set_dock_icon_macos("icon/icon.png");

    let config_manager = ConfigManager::new().expect("Failed to create config manager");
    let is_first_run = !config_manager.config_exists();
    let config = config_manager.load_config();

    let (main_window, main_open_task) = window::open(window::Settings {
        size: Size::new(DEFAULT_WINDOW_WIDTH, DEFAULT_WINDOW_HEIGHT),
        min_size: Some(MAIN_WINDOW_MIN_SIZE),
        icon: crate::ui::icons::load_window_icon("icon/icon.png"),
        // `window::Settings::default()`'s `exit_on_close_request: true`
        // would make iced_winit destroy the window itself the instant the
        // native close button is clicked (`WindowEvent::CloseRequested` ->
        // an automatic `window::close`), racing ahead of and completely
        // bypassing whatever `Message::WindowCloseRequested`'s handler
        // decides to do -- which is exactly why minimizing-to-tray on
        // close never actually worked: the window was already gone by the
        // time that handler ran. `false` here hands control entirely to
        // that handler instead, which is required for "close hides to
        // tray" to do anything at all.
        exit_on_close_request: false,
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
        tray_icon: build_tray_icon(),
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
            discard_stale_location_data(state);
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
            sync_tray_display(state);
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
            sync_tray_display(state);
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
            // Already open -- bring it back to front instead of silently
            // doing nothing, in case it's minimized or just sitting behind
            // other windows (same reasoning as the tray icon's left-click
            // handler; see `bring_window_to_front`'s docs).
            if let Some(id) = state.prefs_window {
                return bring_window_to_front(id);
            }
            state.prefs_state = Some(preferences::State::from_config(&state.config));
            let (id, open_task) = window::open(preferences_window_settings());
            state.prefs_window = Some(id);
            Task::batch([open_task.discard(), fetch_api_token_task(&state.config)])
        }
        Message::OpenAbout => {
            if let Some(id) = state.about_window {
                return bring_window_to_front(id);
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
        Message::AnimationTick => {
            // Piggybacks on the animation timer to drain the tray icon's
            // event channel, rather than adding a second timer just for
            // this -- see `build_tray_icon`'s docs and
            // `examples/tray_spike.rs` for why a polling receiver (as
            // opposed to a push-based `winit::event_loop::EventLoopProxy`
            // integration) works here at all.
            while let Ok(event) = TrayIconEvent::receiver().try_recv() {
                let TrayIconEvent::Click {
                    button,
                    button_state: MouseButtonState::Up,
                    ..
                } = event
                else {
                    continue;
                };
                match button {
                    MouseButton::Left => {
                        return bring_window_to_front(state.main_window);
                    }
                    MouseButton::Right => {
                        // The only way to quit once closing the main
                        // window no longer does (see `WindowCloseRequested`)
                        // -- the `tray` crate has no context-menu API to
                        // offer a proper "Quit" item instead, so right-click
                        // is it. Not `Message::WindowCloseRequested`
                        // (would just re-minimize) or `window::close` (this
                        // isn't a specific window's concern) -- an outright
                        // `iced::exit()` is what a menu's "Quit" item would
                        // ultimately have done anyway.
                        return iced::exit();
                    }
                    MouseButton::Middle => {}
                }
            }
            Task::none()
        }
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
                // With a tray icon present, closing the main window tucks
                // it away into the tray rather than quitting outright --
                // the whole point of "a lightweight, persistent way to see
                // current conditions without the full main window open"
                // (issue #56) is that closing the window is a normal thing
                // to do, not the same as quitting. `window::close` would
                // destroy the window outright (no way to reopen it under
                // the same `window::Id`), so this minimizes instead; the
                // tray icon's left-click handler already knows how to
                // un-minimize and focus it back. Falls back to actually
                // quitting if the tray icon failed to create (`build_tray_icon`)
                // -- with no tray, there'd be no way to ever get the window
                // back otherwise.
                return if state.tray_icon.is_some() {
                    window::minimize(state.main_window, true)
                } else {
                    iced::exit()
                };
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
            let previous_location = state.config.current_location();
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
            // Editing the active location's own fields (or removing it,
            // which falls back to a different entry -- see
            // `preferences::State::apply_to`) changes what "current
            // location" resolves to just as much as the main window's
            // switcher does, and needs the same treatment: don't let the
            // upcoming `RefreshRequested` show the old place's data as
            // `Refreshing` while the new place's fetch is in flight.
            if state.config.current_location() != previous_location {
                discard_stale_location_data(state);
            }
            // Independent of location: `use_fahrenheit` might have just
            // changed with no location change at all, and the tooltip
            // needs to reflect that too. A harmless no-op re-sync in the
            // branch above, which already called this.
            sync_tray_display(state);
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::SavedLocation;
    use crate::weather_api::forecast::ForecastDay;
    use crate::weather_api::openweather_api::{Main, Sys, Weather, WeatherSymbol, Wind};
    use std::sync::atomic::{AtomicU64, Ordering};

    /// Builds an `AppState` for testing without going through `boot()` --
    /// `boot()` opens real OS windows via `window::open`, which needs a
    /// live iced runtime to actually do anything. Neither `window::Id::
    /// unique()` nor `ConfigManager::for_path` (pointed at a scratch file)
    /// need one, so this builds a fully usable `AppState` directly.
    /// `update()`'s handlers don't need a runtime either -- `window::
    /// open()`/`Task::perform(...)` just build inert `Task` *descriptions*;
    /// nothing here ever executes one, so no window actually opens and no
    /// network/keychain call actually fires.
    ///
    /// Returns the scratch config path alongside the state so tests that
    /// exercise `config_manager.save_config` (`LocationSwitched`,
    /// `Preferences::Save`) can clean it up afterward -- harmless to ignore
    /// for tests that never write to it.
    fn test_state(config: AppConfig) -> (AppState, std::path::PathBuf) {
        static COUNTER: AtomicU64 = AtomicU64::new(0);
        let n = COUNTER.fetch_add(1, Ordering::Relaxed);
        let path = std::env::temp_dir().join(format!("open-weather-wizard-app-test-{n}.json"));
        let config_manager = ConfigManager::for_path(path.clone());

        (
            AppState {
                config,
                config_manager,
                weather: WeatherStatus::Loading,
                forecast: ForecastStatus::Loading,
                alerts: vec![],
                system_theme: Theme::Light,
                last_updated: None,
                value_tracker: transition::ValueTracker::default(),
                selected_forecast_day: None,
                main_window: window::Id::unique(),
                prefs_window: None,
                prefs_state: None,
                about_window: None,
                is_first_run: false,
                // Deliberately `None` -- tests shouldn't create a real OS
                // tray icon, and `sync_tray_display` is a no-op without one.
                tray_icon: None,
            },
            path,
        )
    }

    fn sample_weather(name: &str) -> ApiResponse {
        ApiResponse {
            weather: vec![Weather {
                main: "Clear".to_string(),
                description: "clear sky".to_string(),
            }],
            main: Main {
                temp: 20.0,
                feels_like: 19.0,
                temp_min: 15.0,
                temp_max: 25.0,
                pressure: 1013,
                humidity: 50,
            },
            wind: Wind {
                speed: 3.0,
                deg: 180,
            },
            visibility: 10_000,
            sys: Sys {
                sunrise: 0,
                sunset: 0,
            },
            timezone: 0,
            name: name.to_string(),
        }
    }

    fn sample_forecast(days: usize) -> ForecastResponse {
        ForecastResponse {
            location_name: "Test City".to_string(),
            days: (0..days)
                .map(|i| ForecastDay {
                    date: format!("2026-01-{:02}", i + 1),
                    temp_min: 10.0,
                    temp_max: 20.0,
                    description: "clear sky".to_string(),
                    symbol: WeatherSymbol::Clear,
                    feels_like: 15.0,
                    humidity: 50,
                    wind_speed: 2.0,
                    wind_deg: 90,
                    pressure: 1013,
                    visibility: 10_000,
                    pop: 0.1,
                })
                .collect(),
        }
    }

    /// A config with two saved locations ("Home" at index 0, from
    /// `AppConfig::default()`, plus "Work" appended at index 1).
    fn two_location_config() -> AppConfig {
        let mut config = AppConfig::default();
        config.locations.push(SavedLocation {
            name: "Work".to_string(),
            location: LocationConfig {
                city: "Chicago".to_string(),
                state: "IL".to_string(),
                country: "US".to_string(),
            },
        });
        config
    }

    #[test]
    fn test_forecast_day_selected_toggles_selection() {
        let (mut state, path) = test_state(AppConfig::default());

        let _ = update(&mut state, Message::ForecastDaySelected(2));
        assert_eq!(state.selected_forecast_day, Some(2));

        // Re-selecting the same day clears back to live conditions.
        let _ = update(&mut state, Message::ForecastDaySelected(2));
        assert_eq!(state.selected_forecast_day, None);

        let _ = update(&mut state, Message::ForecastDaySelected(3));
        assert_eq!(state.selected_forecast_day, Some(3));

        // Index 0 ("Today") always clears, regardless of what was selected.
        let _ = update(&mut state, Message::ForecastDaySelected(0));
        assert_eq!(state.selected_forecast_day, None);

        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn test_location_switched_updates_current_and_discards_stale_data() {
        let (mut state, path) = test_state(two_location_config());
        state.weather = WeatherStatus::Loaded(sample_weather("Peoria"));
        state.forecast = ForecastStatus::Loaded(sample_forecast(3));
        state.alerts = vec![WeatherAlert {
            id: "1".to_string(),
            title: "Test Alert".to_string(),
            description: String::new(),
            event_type: String::new(),
            severity: crate::weather_api::alerts::AlertSeverity::Minor,
            start_time: 0,
            end_time: 0,
            urgency: String::new(),
            certainty: String::new(),
            area_name: String::new(),
            instruction: vec![],
            safety_recommendations: vec![],
        }];
        state.selected_forecast_day = Some(1);
        state.last_updated = Some(Instant::now());

        let _ = update(&mut state, Message::LocationSwitched(1));

        assert_eq!(state.config.current_location_index, 1);
        assert!(matches!(state.weather, WeatherStatus::Loading));
        assert!(matches!(state.forecast, ForecastStatus::Loading));
        assert!(
            state.alerts.is_empty(),
            "alerts belong to the previous location and shouldn't carry forward"
        );
        assert_eq!(state.selected_forecast_day, None);
        assert_eq!(state.last_updated, None);

        // Persisted immediately, independent of Preferences Save/Cancel.
        let saved = std::fs::read_to_string(&path).unwrap();
        assert!(saved.contains("\"current_location_index\": 1"));

        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn test_location_switched_ignores_out_of_range_and_same_index() {
        let (mut state, path) = test_state(two_location_config());
        state.weather = WeatherStatus::Loaded(sample_weather("Peoria"));

        // Out of range: no-op.
        let _ = update(&mut state, Message::LocationSwitched(5));
        assert_eq!(state.config.current_location_index, 0);
        assert!(matches!(state.weather, WeatherStatus::Loaded(_)));

        // Same as current: no-op, shouldn't discard perfectly good data.
        let _ = update(&mut state, Message::LocationSwitched(0));
        assert_eq!(state.config.current_location_index, 0);
        assert!(matches!(state.weather, WeatherStatus::Loaded(_)));

        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn test_refresh_requested_carries_forward_loaded_data_as_refreshing() {
        let (mut state, path) = test_state(AppConfig::default());
        state.weather = WeatherStatus::Loaded(sample_weather("Peoria"));
        state.forecast = ForecastStatus::Loaded(sample_forecast(2));

        let _ = update(&mut state, Message::RefreshRequested);

        match &state.weather {
            WeatherStatus::Refreshing(data) => assert_eq!(data.name, "Peoria"),
            other => panic!("expected Refreshing, got {other:?}"),
        }
        match &state.forecast {
            ForecastStatus::Refreshing(data) => assert_eq!(data.days.len(), 2),
            other => panic!("expected Refreshing, got {other:?}"),
        }

        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn test_refresh_requested_leaves_loading_as_loading() {
        let (mut state, path) = test_state(AppConfig::default());
        // Fresh state: still `Loading` (never fetched yet).
        let _ = update(&mut state, Message::RefreshRequested);
        assert!(matches!(state.weather, WeatherStatus::Loading));
        assert!(matches!(state.forecast, ForecastStatus::Loading));

        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn test_weather_fetched_ok_sets_loaded_and_last_updated() {
        let (mut state, path) = test_state(AppConfig::default());
        assert!(state.last_updated.is_none());

        let _ = update(
            &mut state,
            Message::WeatherFetched(Ok(sample_weather("Peoria"))),
        );

        assert!(matches!(state.weather, WeatherStatus::Loaded(_)));
        assert!(state.last_updated.is_some());

        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn test_weather_fetched_err_keeps_data_when_refreshing_else_shows_error() {
        let (mut state, path) = test_state(AppConfig::default());

        // A background refresh failing shouldn't blank out good data.
        state.weather = WeatherStatus::Refreshing(sample_weather("Peoria"));
        let _ = update(&mut state, Message::WeatherFetched(Err("boom".to_string())));
        match &state.weather {
            WeatherStatus::Loaded(data) => assert_eq!(data.name, "Peoria"),
            other => panic!("expected Loaded (kept), got {other:?}"),
        }

        // A first-load failure (nothing to show yet) surfaces the error.
        state.weather = WeatherStatus::Loading;
        let _ = update(&mut state, Message::WeatherFetched(Err("boom".to_string())));
        assert!(matches!(state.weather, WeatherStatus::Error(_)));

        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn test_forecast_fetched_clears_stale_selected_day_when_out_of_range() {
        let (mut state, path) = test_state(AppConfig::default());
        state.selected_forecast_day = Some(4);

        let _ = update(&mut state, Message::ForecastFetched(Ok(sample_forecast(2))));

        assert_eq!(
            state.selected_forecast_day, None,
            "a selection past the newly-fetched day count should reset to live conditions"
        );
        assert!(matches!(state.forecast, ForecastStatus::Loaded(_)));

        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn test_forecast_fetched_keeps_selected_day_when_still_in_range() {
        let (mut state, path) = test_state(AppConfig::default());
        state.selected_forecast_day = Some(1);

        let _ = update(&mut state, Message::ForecastFetched(Ok(sample_forecast(3))));

        assert_eq!(state.selected_forecast_day, Some(1));

        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn test_alerts_fetched_ok_replaces_and_err_keeps_existing() {
        let (mut state, path) = test_state(AppConfig::default());

        let _ = update(&mut state, Message::AlertsFetched(Ok(vec![])));
        assert!(state.alerts.is_empty());

        let alert = crate::weather_api::alerts::WeatherAlert {
            id: "1".to_string(),
            title: "Severe Thunderstorm".to_string(),
            description: String::new(),
            event_type: String::new(),
            severity: crate::weather_api::alerts::AlertSeverity::Severe,
            start_time: 0,
            end_time: 0,
            urgency: String::new(),
            certainty: String::new(),
            area_name: String::new(),
            instruction: vec![],
            safety_recommendations: vec![],
        };
        let _ = update(&mut state, Message::AlertsFetched(Ok(vec![alert])));
        assert_eq!(state.alerts.len(), 1);

        // A failed alerts fetch keeps whatever was already there.
        let _ = update(&mut state, Message::AlertsFetched(Err("boom".to_string())));
        assert_eq!(state.alerts.len(), 1);

        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn test_open_preferences_and_about_are_idempotent() {
        let (mut state, path) = test_state(AppConfig::default());

        let _ = update(&mut state, Message::OpenPreferences);
        assert!(state.prefs_window.is_some());
        assert!(state.prefs_state.is_some());
        let first_prefs_window = state.prefs_window;

        // Already open -- a second request shouldn't replace the window or
        // reset the in-progress draft.
        let _ = update(&mut state, Message::OpenPreferences);
        assert_eq!(state.prefs_window, first_prefs_window);

        let _ = update(&mut state, Message::OpenAbout);
        assert!(state.about_window.is_some());
        let first_about_window = state.about_window;

        let _ = update(&mut state, Message::OpenAbout);
        assert_eq!(state.about_window, first_about_window);

        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn test_preferences_save_discards_stale_data_only_when_current_location_changes() {
        use crate::ui::preferences;

        // Case 1: an edit unrelated to location (theme) shouldn't touch
        // weather/forecast at all -- the ordinary `RefreshRequested` path
        // (fired after Save, not exercised directly here) still gets to
        // decide whether to keep showing last-known-good data.
        let (mut state, path) = test_state(AppConfig::default());
        state.weather = WeatherStatus::Loaded(sample_weather("Peoria"));
        state.prefs_state = Some(preferences::State::from_config(&state.config));
        preferences::update(
            state.prefs_state.as_mut().unwrap(),
            preferences::Message::ThemePreferenceSelected(ThemePreference::Dark),
        );
        let _ = update(&mut state, Message::Preferences(preferences::Message::Save));
        assert!(
            matches!(state.weather, WeatherStatus::Loaded(_)),
            "a save that doesn't change the current location shouldn't discard existing data"
        );
        let _ = std::fs::remove_file(&path);

        // Case 2: editing the *active* location's own city directly changes
        // what current_location() resolves to, and should discard stale data
        // the same way `LocationSwitched` does.
        let (mut state, path) = test_state(AppConfig::default());
        state.weather = WeatherStatus::Loaded(sample_weather("Peoria"));
        state.prefs_state = Some(preferences::State::from_config(&state.config));
        preferences::update(
            state.prefs_state.as_mut().unwrap(),
            preferences::Message::CityChanged("Chicago".to_string()),
        );
        let _ = update(&mut state, Message::Preferences(preferences::Message::Save));
        assert!(
            matches!(state.weather, WeatherStatus::Loading),
            "editing the active location's own fields should discard the old place's data"
        );
        assert_eq!(state.config.current_location().city, "Chicago");
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn test_preferences_cancel_discards_draft_without_saving() {
        use crate::ui::preferences;

        let (mut state, path) = test_state(AppConfig::default());
        let _ = update(&mut state, Message::OpenPreferences);
        preferences::update(
            state.prefs_state.as_mut().unwrap(),
            preferences::Message::CityChanged("Chicago".to_string()),
        );

        let _ = update(
            &mut state,
            Message::Preferences(preferences::Message::Cancel),
        );

        assert!(state.prefs_state.is_none());
        assert!(state.prefs_window.is_none());
        assert_eq!(
            state.config.current_location().city,
            "Peoria",
            "Cancel must not write the draft's edits back into AppConfig"
        );

        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn test_tray_tooltip_text_reflects_weather_status_and_units() {
        assert_eq!(
            tray_tooltip_text(&WeatherStatus::Loading, false),
            "Weather Wizard — Loading…"
        );
        assert_eq!(
            tray_tooltip_text(&WeatherStatus::Error("boom".to_string()), false),
            "Weather Wizard — couldn't fetch weather"
        );

        let weather = WeatherStatus::Loaded(sample_weather("Peoria"));
        assert_eq!(
            tray_tooltip_text(&weather, false),
            "Weather Wizard — 20°C clear sky"
        );
        assert_eq!(
            tray_tooltip_text(&weather, true),
            "Weather Wizard — 68°F clear sky"
        );

        // `Refreshing` (last-known-good data mid-fetch) reads the same as
        // `Loaded` -- the tooltip shouldn't flicker to "Loading…" on every
        // background refresh.
        let mut refreshing_data = sample_weather("Peoria");
        refreshing_data.main.temp = 20.0;
        assert_eq!(
            tray_tooltip_text(&WeatherStatus::Refreshing(refreshing_data), false),
            "Weather Wizard — 20°C clear sky"
        );
    }

    #[test]
    fn test_tray_title_text_is_compact_and_absent_when_theres_nothing_to_say() {
        // No title at all while loading or after an error -- a blank
        // title next to the icon would just be visual clutter, not a
        // placeholder worth showing.
        assert_eq!(tray_title_text(&WeatherStatus::Loading, false), None);
        assert_eq!(
            tray_title_text(&WeatherStatus::Error("boom".to_string()), false),
            None
        );

        let weather = WeatherStatus::Loaded(sample_weather("Peoria"));
        assert_eq!(tray_title_text(&weather, false), Some("20°C".to_string()));
        assert_eq!(tray_title_text(&weather, true), Some("68°F".to_string()));

        // Same "Refreshing reads like Loaded" rule as the tooltip.
        assert_eq!(
            tray_title_text(&WeatherStatus::Refreshing(sample_weather("Peoria")), false),
            Some("20°C".to_string())
        );
    }

    #[test]
    fn test_sync_tray_display_is_a_no_op_without_a_tray_icon() {
        // No real assertion beyond "doesn't panic" -- `test_state` always
        // sets `tray_icon: None`, so this exercises the early-return path
        // that every other test in this module already relies on
        // implicitly every time `sync_tray_display` runs inside `update()`.
        let (mut state, path) = test_state(AppConfig::default());
        state.weather = WeatherStatus::Loaded(sample_weather("Peoria"));
        sync_tray_display(&state);
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn test_theme_resolves_explicit_light_and_dark_regardless_of_system() {
        let (mut state, path) = test_state(AppConfig::default());
        state.system_theme = Theme::Dark;

        state.config.theme_preference = ThemePreference::Light;
        assert_eq!(theme(&state, state.main_window), Theme::Light);

        state.config.theme_preference = ThemePreference::Dark;
        assert_eq!(theme(&state, state.main_window), Theme::Dark);

        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn test_theme_system_uses_detected_system_theme() {
        let (mut state, path) = test_state(AppConfig::default());
        state.config.theme_preference = ThemePreference::System;
        state.system_theme = Theme::Dark;

        assert_eq!(theme(&state, state.main_window), Theme::Dark);

        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn test_theme_previews_live_preferences_draft_over_saved_config() {
        use crate::ui::preferences;

        let (mut state, path) = test_state(AppConfig::default());
        state.config.theme_preference = ThemePreference::Light;
        state.prefs_state = Some(preferences::State::from_config(&state.config));

        // Not saved yet -- but the open draft's live edit should already
        // preview across every window.
        preferences::update(
            state.prefs_state.as_mut().unwrap(),
            preferences::Message::ThemePreferenceSelected(ThemePreference::Dark),
        );
        assert_eq!(theme(&state, state.main_window), Theme::Dark);
        assert_eq!(
            state.config.theme_preference,
            ThemePreference::Light,
            "the saved config itself shouldn't change until Save"
        );

        // Once the draft is gone (Cancel/Save closed the window), fall back
        // to the persisted config again.
        state.prefs_state = None;
        assert_eq!(theme(&state, state.main_window), Theme::Light);

        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn test_title_reflects_which_window_and_first_run_state() {
        let (mut state, path) = test_state(AppConfig::default());
        let prefs_window = window::Id::unique();
        let about_window = window::Id::unique();
        state.prefs_window = Some(prefs_window);
        state.about_window = Some(about_window);

        assert_eq!(title(&state, state.main_window), "Weather Wizard");
        assert_eq!(title(&state, about_window), "About Weather Wizard");

        state.is_first_run = true;
        assert_eq!(title(&state, prefs_window), "Welcome to Weather Wizard");
        state.is_first_run = false;
        assert_eq!(title(&state, prefs_window), "Preferences");

        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn test_title_shows_current_location_name_only_with_multiple_locations() {
        let (state, path) = test_state(AppConfig::default());
        assert_eq!(
            title(&state, state.main_window),
            "Weather Wizard",
            "a single 'Home' location is unnamed noise in the title"
        );
        let _ = std::fs::remove_file(&path);

        let (state, path) = test_state(two_location_config());
        assert_eq!(title(&state, state.main_window), "Weather Wizard — Home");
        let _ = std::fs::remove_file(&path);
    }
}
