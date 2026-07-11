//! # Preferences Screen
//!
//! Renders in its own OS window (opened by `Message::OpenPreferences` in `src/app.rs`),
//! matching the previous GTK version's transient preferences dialog. Owns its own
//! form-field state; the parent `AppState` intercepts `Save`/`Cancel` since only it
//! holds the persisted `AppConfig`/`ConfigManager`.

use iced::widget::{
    button, column, container, pick_list, row, scrollable, space, text, text_input, toggler,
};
use iced::{Alignment, Element, Font, Length, font};

use crate::config::{
    AppConfig, Language, LocationConfig, SavedLocation, ThemePreference, WeatherApiProvider,
};
use crate::ui::style;

const BOLD: Font = Font {
    weight: font::Weight::Bold,
    ..Font::DEFAULT
};

const PROVIDERS: [WeatherApiProvider; 2] = [
    WeatherApiProvider::OpenWeather,
    WeatherApiProvider::GoogleWeather,
];

const THEME_PREFERENCES: [ThemePreference; 3] = [
    ThemePreference::Light,
    ThemePreference::Dark,
    ThemePreference::System,
];

const LANGUAGES: [Language; 12] = [
    Language::English,
    Language::Spanish,
    Language::French,
    Language::German,
    Language::Italian,
    Language::Portuguese,
    Language::Russian,
    Language::Japanese,
    Language::Korean,
    Language::Arabic,
    Language::Hindi,
    Language::Dutch,
];

/// Presets for the auto-refresh polling interval, offered as a pick list
/// to prevent accidental rates-quota violations.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RefreshIntervalPreset {
    ThirtySeconds,
    OneMinute,
    FiveMinutes,
    FifteenMinutes,
    ThirtyMinutes,
}

impl RefreshIntervalPreset {
    pub const ALL: [Self; 5] = [
        Self::ThirtySeconds,
        Self::OneMinute,
        Self::FiveMinutes,
        Self::FifteenMinutes,
        Self::ThirtyMinutes,
    ];

    pub fn to_secs(self) -> u64 {
        match self {
            Self::ThirtySeconds => 30,
            Self::OneMinute => 60,
            Self::FiveMinutes => 5 * 60,
            Self::FifteenMinutes => 15 * 60,
            Self::ThirtyMinutes => 30 * 60,
        }
    }

    pub fn from_secs(secs: u64) -> Self {
        match secs {
            30 => Self::ThirtySeconds,
            60 => Self::OneMinute,
            300 => Self::FiveMinutes,
            900 => Self::FifteenMinutes,
            1800 => Self::ThirtyMinutes,
            _ => {
                if secs <= 30 {
                    Self::ThirtySeconds
                } else if secs <= 60 {
                    Self::OneMinute
                } else if secs <= 300 {
                    Self::FiveMinutes
                } else if secs <= 900 {
                    Self::FifteenMinutes
                } else {
                    Self::ThirtyMinutes
                }
            }
        }
    }
}

impl std::fmt::Display for RefreshIntervalPreset {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ThirtySeconds => write!(f, "30 seconds"),
            Self::OneMinute => write!(f, "1 minute"),
            Self::FiveMinutes => write!(f, "5 minutes"),
            Self::FifteenMinutes => write!(f, "15 minutes"),
            Self::ThirtyMinutes => write!(f, "30 minutes"),
        }
    }
}

/// A saved location as edited in the Preferences form -- flat fields
/// (rather than nesting `LocationConfig`) so each field can be wired to its
/// own `text_input::on_input` the same way the old single-location fields
/// were.
#[derive(Debug, Clone, PartialEq)]
pub struct LocationEntry {
    pub name: String,
    pub city: String,
    pub state: String,
    pub country: String,
}

impl From<&SavedLocation> for LocationEntry {
    fn from(saved: &SavedLocation) -> Self {
        Self {
            name: saved.name.clone(),
            city: saved.location.city.clone(),
            state: saved.location.state.clone(),
            country: saved.location.country.clone(),
        }
    }
}

impl From<&LocationEntry> for SavedLocation {
    fn from(entry: &LocationEntry) -> Self {
        Self {
            name: entry.name.clone(),
            location: LocationConfig {
                city: entry.city.clone(),
                state: entry.state.clone(),
                country: entry.country.clone(),
            },
        }
    }
}

#[derive(Debug, Clone)]
pub struct State {
    pub provider: WeatherApiProvider,
    pub token_input: String,
    /// Every saved location, in display order -- a draft copy of
    /// `AppConfig.locations`, discarded on Cancel like every other field
    /// here. Always has at least one entry; `RemoveLocationRequested`
    /// refuses to drop the last one (see its docs).
    pub locations: Vec<LocationEntry>,
    /// Which entry in `locations` the form below is currently showing/
    /// editing -- purely a Preferences-form concern, distinct from
    /// `AppConfig.current_location_index` (which one the main window
    /// shows), so opening Preferences to rename "Work" doesn't change what
    /// the main window is looking at.
    pub selected_location_index: usize,
    /// Tracks the position of whatever entry was `AppConfig.
    /// current_location_index` when this form opened. Kept in sync as an
    /// *index* by `AddLocationRequested`/`RemoveLocationRequested`/
    /// `MoveLocationUp`/`MoveLocationDown` (the only messages that actually
    /// shift entries around) rather than re-derived from the entry's name
    /// after the fact -- a rename doesn't move an entry's position, so
    /// tracking the position survives a rename for free, and two entries
    /// are always allowed to share a name (nothing here enforces
    /// uniqueness) which would make a name-based lookup ambiguous anyway.
    /// `None` only once the tracked entry has actually been removed, at
    /// which point `apply_to` falls back to index 0.
    current_location_index: Option<usize>,
    /// The language weather descriptions are requested in -- not a UI
    /// localization setting, see `Language`'s docs.
    pub language: Language,
    pub theme_preference: ThemePreference,
    pub use_fahrenheit: bool,
    pub launch_at_login: bool,
    pub refresh_interval: RefreshIntervalPreset,
    /// Set by `app::boot` when this window was opened automatically because
    /// no config file existed yet (see `ConfigManager::config_exists`).
    /// Purely cosmetic -- swaps in a welcome banner (`view`) and the
    /// window's title (`app::title`); every field and validation rule
    /// behaves identically either way.
    pub is_first_run: bool,
    /// Whether an IP-based location lookup (`Message::DetectLocationRequested`,
    /// intercepted by `app::update`) is currently in flight -- disables the
    /// "Detect my location" button and swaps its label so a slow/failed
    /// lookup doesn't look like a dead button.
    pub is_detecting_location: bool,
    /// Set if the last detection attempt failed, cleared on the next
    /// attempt or a successful one. Never blocks Save -- detection is a
    /// convenience prefill, not a required step; the fields can always be
    /// typed in by hand instead.
    pub location_detection_error: Option<String>,
    /// Whether a connection test (`Message::TestConnectionRequested`,
    /// intercepted by `app::update`) is currently in flight -- disables the
    /// "Verify API" button and swaps its label, same idea as
    /// `is_detecting_location`.
    pub is_testing_connection: bool,
    /// Result of the last connection test, cleared on the next attempt.
    /// Purely informational -- never blocks Save, same philosophy as
    /// location detection being best-effort.
    pub connection_test_result: Option<Result<(), String>>,
}

impl State {
    /// Builds the form's initial state from the persisted config -- except
    /// `token_input`, deliberately left empty here rather than reading it
    /// synchronously via `AppConfig::get_api_token`. That's a blocking OS
    /// keychain call that can pop a permission prompt (macOS re-prompts per
    /// build, or every time if the user picked "Allow" over "Always
    /// Allow"); calling it here would run it on `app::update`'s own thread
    /// and freeze the whole UI -- including the window that's about to
    /// open -- until any hidden/background prompt is dismissed. The caller
    /// (`app::update`'s `OpenPreferences` handler and `boot`) instead fires
    /// an async `Task` to read it off-thread and fills it in once resolved
    /// via `Message::ApiTokenLoaded`.
    pub fn from_config(config: &AppConfig) -> Self {
        Self {
            provider: config.weather_provider.clone(),
            token_input: String::new(),
            locations: config.locations.iter().map(LocationEntry::from).collect(),
            selected_location_index: config
                .current_location_index
                .min(config.locations.len().saturating_sub(1)),
            current_location_index: Some(
                config
                    .current_location_index
                    .min(config.locations.len().saturating_sub(1)),
            ),
            language: config.language,
            theme_preference: config.theme_preference,
            use_fahrenheit: config.use_fahrenheit,
            launch_at_login: config.launch_at_login,
            refresh_interval: config
                .refresh_interval_secs
                .map(RefreshIntervalPreset::from_secs)
                .unwrap_or_else(|| match config.weather_provider {
                    WeatherApiProvider::GoogleWeather => RefreshIntervalPreset::FifteenMinutes,
                    WeatherApiProvider::OpenWeather => RefreshIntervalPreset::ThirtySeconds,
                }),
            is_first_run: false,
            is_detecting_location: false,
            location_detection_error: None,
            is_testing_connection: false,
            connection_test_result: None,
        }
    }

    /// Writes the edited fields back into the shared `AppConfig`. Two steps
    /// can fail: `set_api_token` (an OS keychain write) and
    /// `update_auto_launch` (an OS-level login-item registration) --
    /// everything else here is an in-memory field assignment.
    pub fn apply_to(&self, config: &mut AppConfig) -> Result<(), String> {
        config.weather_provider = self.provider.clone();
        if !self.token_input.is_empty() {
            config.set_api_token(&self.token_input)?;
        }
        config.locations = self.locations.iter().map(SavedLocation::from).collect();
        // `current_location_index` has already been kept in sync as an
        // index by every message that actually moves entries around (see
        // its docs) -- `None` only once the tracked entry has been removed
        // outright, same "just don't crash, degrade to something sane"
        // philosophy as `AppConfig::current_location`.
        config.current_location_index = self.current_location_index.unwrap_or(0);
        config.language = self.language;
        config.theme_preference = self.theme_preference;
        config.use_fahrenheit = self.use_fahrenheit;
        config.launch_at_login = self.launch_at_login;
        config.refresh_interval_secs = Some(self.refresh_interval.to_secs());
        config
            .update_auto_launch()
            .map_err(|e| format!("Failed to configure auto-launch: {}", e))?;
        Ok(())
    }

    /// Field-level problems that must be fixed before `Save` is allowed.
    /// State/Province is deliberately not validated -- plenty of real
    /// locations don't have one.
    pub fn validation_errors(&self) -> Vec<String> {
        let mut errors = Vec::new();

        for entry in &self.locations {
            // A blank name isn't just cosmetic here -- `apply_to` re-finds
            // the current location by name after Save, so an empty or
            // duplicate name would make that lookup ambiguous.
            let label = if entry.name.trim().is_empty() {
                "(unnamed)".to_string()
            } else {
                entry.name.clone()
            };
            if entry.name.trim().is_empty() {
                errors.push("Every saved location needs a name.".to_string());
            }
            if entry.city.trim().is_empty() {
                errors.push(format!("\"{label}\" needs a city."));
            }
            if entry.country.trim().is_empty() {
                errors.push(format!("\"{label}\" needs a country."));
            }
        }
        // Both providers require a token -- WeatherProviderFactory::create_provider
        // errors out without one for either WeatherApiProvider variant.
        if self.token_input.trim().is_empty() {
            errors.push(format!("API Token is required for {}.", self.provider));
        }
        // Validate Google Weather refresh interval constraint
        if self.provider == WeatherApiProvider::GoogleWeather
            && self.refresh_interval.to_secs() < 15 * 60
        {
            errors.push(
                "Google Weather requires a refresh interval of at least 15 minutes.".to_string(),
            );
        }

        errors
    }
}

#[derive(Debug, Clone)]
pub enum Message {
    ProviderSelected(WeatherApiProvider),
    TokenChanged(String),
    /// Switches which entry in `State::locations` the form fields below
    /// are showing/editing -- not which one the main window displays, see
    /// `State::selected_location_index`'s docs.
    LocationSelected(usize),
    LocationNameChanged(String),
    CityChanged(String),
    StateChanged(String),
    CountryChanged(String),
    /// Appends a new blank entry to `State::locations` and selects it.
    AddLocationRequested,
    /// Removes the currently-selected entry from `State::locations` -- a
    /// no-op if it's the only one left (see `update`'s docs on why removing
    /// the last location isn't allowed).
    RemoveLocationRequested,
    /// Swaps the currently-selected entry with its neighbor to reorder the
    /// list, following the selection so repeated clicks keep moving the
    /// same entry.
    MoveLocationUp,
    MoveLocationDown,
    LanguageSelected(Language),
    ThemePreferenceSelected(ThemePreference),
    UnitsToggled(bool),
    LaunchAtLoginToggled(bool),
    RefreshIntervalSelected(RefreshIntervalPreset),
    /// The "Get an API key" link -- intercepted by the parent (see
    /// `src/app.rs`) and turned into `Message::OpenUrl`, since opening a
    /// browser is an app-level concern, not something this module does
    /// itself.
    OpenUrl(String),
    /// The "Detect my location" button -- intercepted by the parent, which
    /// kicks off the async IP lookup (`crate::geolocation::detect_location`)
    /// and reports back via the app-level `Message::LocationDetected`, since
    /// firing an async `Task` isn't something this module's synchronous
    /// `update` can do itself.
    DetectLocationRequested,
    /// The "Verify API" button -- intercepted by the parent, which
    /// builds a provider from the *currently-typed* provider/token/location
    /// (not the saved config), fires a single `get_weather()` call, and
    /// reports back via the app-level `Message::ConnectionTested`, since
    /// firing an async `Task` isn't something this module's synchronous
    /// `update` can do itself.
    TestConnectionRequested,
    Save,
    Cancel,
}

/// Mutates field-edit messages; `Save`/`Cancel`/`OpenUrl`/
/// `DetectLocationRequested`/`TestConnectionRequested` are intercepted by the
/// parent `AppState::update` (see `src/app.rs`) since they need access to
/// `AppConfig`/the OS's URL opener/an async `Task` respectively.
pub fn update(state: &mut State, message: Message) {
    match message {
        Message::ProviderSelected(provider) => state.provider = provider,
        Message::TokenChanged(value) => state.token_input = value,
        Message::LocationSelected(index) => {
            if index < state.locations.len() {
                state.selected_location_index = index;
                // Stale detection state/error from a different entry
                // shouldn't bleed into the newly-selected one.
                state.is_detecting_location = false;
                state.location_detection_error = None;
            }
        }
        Message::LocationNameChanged(value) => {
            if let Some(entry) = state.locations.get_mut(state.selected_location_index) {
                entry.name = value;
            }
        }
        Message::CityChanged(value) => {
            if let Some(entry) = state.locations.get_mut(state.selected_location_index) {
                entry.city = value;
            }
        }
        Message::StateChanged(value) => {
            if let Some(entry) = state.locations.get_mut(state.selected_location_index) {
                entry.state = value;
            }
        }
        Message::CountryChanged(value) => {
            if let Some(entry) = state.locations.get_mut(state.selected_location_index) {
                entry.country = value;
            }
        }
        Message::AddLocationRequested => {
            state.locations.push(LocationEntry {
                name: format!("Location {}", state.locations.len() + 1),
                city: String::new(),
                state: String::new(),
                country: String::new(),
            });
            state.selected_location_index = state.locations.len() - 1;
            // Appending doesn't shift anything before it, so the tracked
            // "current" index (if any) still points at the same entry.
            state.is_detecting_location = false;
            state.location_detection_error = None;
        }
        Message::RemoveLocationRequested => {
            // At least one saved location must always exist -- the main
            // window has nothing to show otherwise. The button driving this
            // message is itself disabled at that point (see `view`), but
            // guard here too since nothing else enforces it.
            if state.locations.len() > 1 {
                let removed_index = state.selected_location_index;
                state.locations.remove(removed_index);
                state.selected_location_index =
                    state.selected_location_index.min(state.locations.len() - 1);
                state.current_location_index = match state.current_location_index {
                    // The tracked entry itself was just removed -- `apply_to`
                    // falls back to index 0 for a `None` here.
                    Some(current) if current == removed_index => None,
                    // Everything after the removed entry shifts down by one.
                    Some(current) if current > removed_index => Some(current - 1),
                    other => other,
                };
                state.is_detecting_location = false;
                state.location_detection_error = None;
            }
        }
        Message::MoveLocationUp => {
            if state.selected_location_index > 0 {
                let (a, b) = (
                    state.selected_location_index,
                    state.selected_location_index - 1,
                );
                state.locations.swap(a, b);
                state.selected_location_index = b;
                state.current_location_index =
                    swap_tracked_index(state.current_location_index, a, b);
                state.is_detecting_location = false;
                state.location_detection_error = None;
            }
        }
        Message::MoveLocationDown => {
            if state.selected_location_index + 1 < state.locations.len() {
                let (a, b) = (
                    state.selected_location_index,
                    state.selected_location_index + 1,
                );
                state.locations.swap(a, b);
                state.selected_location_index = b;
                state.current_location_index =
                    swap_tracked_index(state.current_location_index, a, b);
                state.is_detecting_location = false;
                state.location_detection_error = None;
            }
        }
        Message::LanguageSelected(value) => state.language = value,
        Message::ThemePreferenceSelected(value) => state.theme_preference = value,
        Message::UnitsToggled(value) => state.use_fahrenheit = value,
        Message::LaunchAtLoginToggled(value) => state.launch_at_login = value,
        Message::RefreshIntervalSelected(value) => state.refresh_interval = value,
        Message::OpenUrl(_)
        | Message::DetectLocationRequested
        | Message::TestConnectionRequested
        | Message::Save
        | Message::Cancel => {
            // Handled by the parent; nothing to do locally.
        }
    }
}

/// Adjusts a tracked location index after swapping the entries at `a` and
/// `b` (`MoveLocationUp`/`MoveLocationDown`) -- if the tracked index was
/// pointing at either swapped position, it now points at the other one;
/// otherwise it's untouched.
fn swap_tracked_index(tracked: Option<usize>, a: usize, b: usize) -> Option<usize> {
    tracked.map(|i| {
        if i == a {
            b
        } else if i == b {
            a
        } else {
            i
        }
    })
}

/// Where to get an API key for each provider, and a matching link label --
/// shown under the API Token field regardless of first-run status, since
/// switching providers later needs the same pointer.
fn api_key_hint(provider: &WeatherApiProvider) -> (&'static str, &'static str) {
    match provider {
        WeatherApiProvider::OpenWeather => (
            "Get an OpenWeatherMap API key",
            "https://home.openweathermap.org/users/sign_up",
        ),
        WeatherApiProvider::GoogleWeather => (
            "Get a Google Weather API key",
            "https://developers.google.com/maps/documentation/weather/overview",
        ),
    }
}

pub fn view(state: &State) -> Element<'_, Message> {
    let (hint_label, hint_url) = api_key_hint(&state.provider);

    let connected = matches!(state.connection_test_result, Some(Ok(())));

    let mut provider_column = column![
        labeled_row(
            "Provider:",
            pick_list(
                PROVIDERS,
                Some(state.provider.clone()),
                Message::ProviderSelected
            )
            .style(style::pick_list)
            .into()
        ),
        labeled_row(
            "Language:",
            pick_list(
                &LANGUAGES[..],
                Some(state.language),
                Message::LanguageSelected
            )
            .style(style::pick_list)
            .into()
        ),
        labeled_row(
            "API Token:",
            text_input("Enter your API token", &state.token_input)
                .secure(true)
                .on_input(Message::TokenChanged)
                .style(style::text_input)
                .into()
        ),
        api_key_hint_row(hint_label, hint_url),
        test_connection_row(state.is_testing_connection, connected),
    ]
    .spacing(12);

    if let Some(Err(e)) = &state.connection_test_result {
        provider_column = provider_column.push(location_hint_row(
            text(format!("\u{2717} {e}"))
                .size(12)
                .style(style::danger)
                .into(),
        ));
    }

    let provider_section = section("\u{2699} Weather Provider", provider_column.into());

    let mut location_tabs = row![].spacing(6);
    for (index, entry) in state.locations.iter().enumerate() {
        let label = if entry.name.trim().is_empty() {
            "(unnamed)".to_string()
        } else {
            entry.name.clone()
        };
        let is_selected = index == state.selected_location_index;
        location_tabs = location_tabs.push(
            button(text(label).size(12))
                .on_press(Message::LocationSelected(index))
                .style(if is_selected {
                    style::accent_button
                } else {
                    style::secondary_button
                }),
        );
    }
    location_tabs = location_tabs.push(
        button(text("+ Add").size(12))
            .on_press(Message::AddLocationRequested)
            .style(style::secondary_button),
    );

    // Always in bounds: `update()` clamps `selected_location_index` on every
    // add/remove/reorder, and `locations` is never emptied (see its docs).
    let selected = &state.locations[state.selected_location_index];
    let can_remove = state.locations.len() > 1;

    let mut location_column = column![
        location_tabs,
        labeled_row(
            "Name:",
            text_input("Enter a name for this location", &selected.name)
                .on_input(Message::LocationNameChanged)
                .style(style::text_input)
                .into()
        ),
        labeled_row(
            "City:",
            text_input("Enter city name", &selected.city)
                .on_input(Message::CityChanged)
                .style(style::text_input)
                .into()
        ),
        labeled_row(
            "State/Province:",
            text_input("Enter state or province", &selected.state)
                .on_input(Message::StateChanged)
                .style(style::text_input)
                .into()
        ),
        labeled_row(
            "Country:",
            text_input("Enter country code (e.g., US, CA)", &selected.country)
                .on_input(Message::CountryChanged)
                .style(style::text_input)
                .into()
        ),
        detect_location_row(state.is_detecting_location),
        location_actions_row(
            can_remove,
            state.selected_location_index,
            state.locations.len()
        ),
    ]
    .spacing(12);

    if let Some(error) = &state.location_detection_error {
        location_column = location_column.push(location_hint_row(
            text(error.clone()).size(12).style(style::danger).into(),
        ));
    }

    // "Locations" rather than "Home": the app now supports saving several
    // places and switching between them from the main window (issue #55);
    // the tab strip above picks which one this form is editing.
    let location_section = section("\u{2302} Locations", location_column.into());

    let appearance_section = section(
        "\u{263e} Appearance & Refresh",
        column![
            labeled_row(
                "Theme:",
                pick_list(
                    &THEME_PREFERENCES[..],
                    Some(state.theme_preference),
                    Message::ThemePreferenceSelected
                )
                .style(style::pick_list)
                .into()
            ),
            toggler(state.use_fahrenheit)
                .label("Use Fahrenheit (\u{b0}F)")
                .on_toggle(Message::UnitsToggled),
            toggler(state.launch_at_login)
                .label("Launch at login")
                .on_toggle(Message::LaunchAtLoginToggled),
            labeled_row(
                "Refresh Interval:",
                pick_list(
                    &RefreshIntervalPreset::ALL[..],
                    Some(state.refresh_interval),
                    Message::RefreshIntervalSelected
                )
                .style(style::pick_list)
                .into()
            ),
        ]
        .spacing(12)
        .into(),
    );

    let errors = state.validation_errors();

    let buttons = row![
        space::horizontal(),
        button("Cancel")
            .on_press(Message::Cancel)
            .style(style::secondary_button),
        button("Save")
            .on_press_maybe(errors.is_empty().then_some(Message::Save))
            .style(style::primary_button),
    ]
    .spacing(8)
    .align_y(Alignment::Center);

    // Normally the window's own title bar already reads "Preferences" (see
    // `app::title`), so an in-content heading would just repeat it -- but on
    // first run the title bar instead reads "Welcome to Weather Wizard",
    // and this banner is the one place that explains *why* the window
    // opened on its own and what the three sections below are for.
    let mut layout = column![].spacing(16).padding(20).width(Length::Fill);

    if state.is_first_run {
        layout = layout.push(
            column![
                text("Welcome to Weather Wizard!")
                    .size(16)
                    .font(BOLD)
                    .style(style::accent),
                text(
                    "Choose a weather provider, add its API key, and set your \
                     Home location (typed in, or detected from your IP address) \
                     to get started."
                )
                .size(12)
                .style(style::muted),
            ]
            .spacing(4),
        );
    }

    layout = layout
        .push(provider_section)
        .push(location_section)
        .push(appearance_section);

    if !errors.is_empty() {
        let mut error_list = column![].spacing(2);
        for error in errors.iter().cloned() {
            error_list = error_list.push(text(error).size(12).style(style::danger));
        }
        layout = layout.push(error_list);
    }

    scrollable(container(layout.push(buttons)))
        .height(Length::Fill)
        .into()
}

/// A titled card grouping related fields, matching the forecast day-card
/// visual style so the form reads as distinct sections instead of one flat
/// list.
fn section<'a>(title: &'a str, content: Element<'a, Message>) -> Element<'a, Message> {
    container(
        column![
            text(title).size(14).font(BOLD).style(style::accent),
            content
        ]
        .spacing(12)
        .width(Length::Fill),
    )
    .padding(14)
    .style(style::day_card)
    .into()
}

fn labeled_row<'a>(label: &'a str, field: Element<'a, Message>) -> Element<'a, Message> {
    row![text(label).width(160), field]
        .spacing(12)
        .align_y(Alignment::Center)
        .into()
}

/// The "Get an API key" link under the API Token field, indented to align
/// under the input rather than the label (matching `labeled_row`'s 160px
/// label column).
fn api_key_hint_row(label: &'static str, url: &'static str) -> Element<'static, Message> {
    row![
        space::horizontal().width(160),
        button(text(label).size(12))
            .on_press(Message::OpenUrl(url.to_string()))
            .style(style::link_button)
            .padding(0),
    ]
    .spacing(12)
    .into()
}

/// Indents an arbitrary element under the Home section's fields, aligning it
/// under the input column rather than the label column (matching
/// `labeled_row`'s 160px label width) -- used for the location-detection
/// error line, which isn't itself a button/link like the other hint rows.
fn location_hint_row(content: Element<'_, Message>) -> Element<'_, Message> {
    row![space::horizontal().width(160), content]
        .spacing(12)
        .into()
}

/// The "Detect my location" button, indented to align under the Home
/// section's input fields. A separate function (rather than inline in
/// `view`) purely to give the surrounding `column!` macro's `Into<Element>`
/// call an unambiguous type to infer against.
fn detect_location_row(is_detecting: bool) -> Element<'static, Message> {
    row![
        space::horizontal().width(160),
        button(text(if is_detecting {
            "Detecting..."
        } else {
            "Detect my location"
        }))
        .on_press_maybe((!is_detecting).then_some(Message::DetectLocationRequested))
        .style(style::secondary_button),
    ]
    .align_y(Alignment::Center)
    .into()
}

/// Remove/reorder controls for the currently-selected saved location,
/// indented to align under the Locations section's fields. Remove is
/// disabled entirely (rather than erroring on press) when it's the only
/// location left; Move Up/Down are disabled at the ends of the list.
fn location_actions_row(can_remove: bool, index: usize, count: usize) -> Element<'static, Message> {
    row![
        space::horizontal().width(160),
        button(text("Remove").size(12))
            .on_press_maybe(can_remove.then_some(Message::RemoveLocationRequested))
            .style(style::secondary_button),
        button(text("\u{2191} Move Up").size(12))
            .on_press_maybe((index > 0).then_some(Message::MoveLocationUp))
            .style(style::secondary_button),
        button(text("\u{2193} Move Down").size(12))
            .on_press_maybe((index + 1 < count).then_some(Message::MoveLocationDown))
            .style(style::secondary_button),
    ]
    .spacing(8)
    .align_y(Alignment::Center)
    .into()
}

/// The "Verify API" button, indented to align under the API Token
/// field -- same pattern as `detect_location_row`. A successful result is
/// shown inline right next to the button (rather than a full row below,
/// like the error case in `view`) since "\u{2714} Connected" is short
/// enough to never wrap. Uses the plain Heavy Check Mark glyph (colored via
/// `style::success`) rather than the ✅ emoji -- iced's text renderer draws
/// glyphs from the system's default font, and color emoji isn't guaranteed
/// to render in full color there, unlike a typographic glyph.
fn test_connection_row(is_testing: bool, connected: bool) -> Element<'static, Message> {
    let mut content = row![
        space::horizontal().width(160),
        button(text(if is_testing {
            "Verifying..."
        } else {
            "Verify API"
        }))
        .on_press_maybe((!is_testing).then_some(Message::TestConnectionRequested))
        .style(style::accent_button),
    ]
    .spacing(12)
    .align_y(Alignment::Center);

    if connected {
        content = content.push(text("\u{2714} Connected").size(13).style(style::success));
    }

    content.into()
}
