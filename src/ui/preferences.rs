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

use crate::config::{AppConfig, WeatherApiProvider};
use crate::ui::style;

const BOLD: Font = Font {
    weight: font::Weight::Bold,
    ..Font::DEFAULT
};

const PROVIDERS: [WeatherApiProvider; 2] = [
    WeatherApiProvider::OpenWeather,
    WeatherApiProvider::GoogleWeather,
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

#[derive(Debug, Clone)]
pub struct State {
    pub provider: WeatherApiProvider,
    pub token_input: String,
    pub city_input: String,
    pub state_input: String,
    pub country_input: String,
    pub dark_mode: bool,
    pub use_fahrenheit: bool,
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
            city_input: config.location.city.clone(),
            state_input: config.location.state.clone(),
            country_input: config.location.country.clone(),
            dark_mode: config.dark_mode,
            use_fahrenheit: config.use_fahrenheit,
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

    /// Writes the edited fields back into the shared `AppConfig`. Only
    /// `set_api_token` (an OS keychain write) can actually fail -- everything
    /// else here is an in-memory field assignment.
    pub fn apply_to(&self, config: &mut AppConfig) -> Result<(), String> {
        config.weather_provider = self.provider.clone();
        if !self.token_input.is_empty() {
            config.set_api_token(&self.token_input)?;
        }
        config.location.city = self.city_input.clone();
        config.location.state = self.state_input.clone();
        config.location.country = self.country_input.clone();
        config.dark_mode = self.dark_mode;
        config.use_fahrenheit = self.use_fahrenheit;
        config.refresh_interval_secs = Some(self.refresh_interval.to_secs());
        Ok(())
    }

    /// Field-level problems that must be fixed before `Save` is allowed.
    /// State/Province is deliberately not validated -- plenty of real
    /// locations don't have one.
    pub fn validation_errors(&self) -> Vec<String> {
        let mut errors = Vec::new();

        if self.city_input.trim().is_empty() {
            errors.push("City is required.".to_string());
        }
        if self.country_input.trim().is_empty() {
            errors.push("Country is required.".to_string());
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
    CityChanged(String),
    StateChanged(String),
    CountryChanged(String),
    DarkModeToggled(bool),
    UnitsToggled(bool),
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
        Message::CityChanged(value) => state.city_input = value,
        Message::StateChanged(value) => state.state_input = value,
        Message::CountryChanged(value) => state.country_input = value,
        Message::DarkModeToggled(value) => state.dark_mode = value,
        Message::UnitsToggled(value) => state.use_fahrenheit = value,
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

    let mut location_column = column![
        labeled_row(
            "City:",
            text_input("Enter city name", &state.city_input)
                .on_input(Message::CityChanged)
                .style(style::text_input)
                .into()
        ),
        labeled_row(
            "State/Province:",
            text_input("Enter state or province", &state.state_input)
                .on_input(Message::StateChanged)
                .style(style::text_input)
                .into()
        ),
        labeled_row(
            "Country:",
            text_input("Enter country code (e.g., US, CA)", &state.country_input)
                .on_input(Message::CountryChanged)
                .style(style::text_input)
                .into()
        ),
        detect_location_row(state.is_detecting_location),
    ]
    .spacing(12);

    if let Some(error) = &state.location_detection_error {
        location_column = location_column.push(location_hint_row(
            text(error.clone()).size(12).style(style::danger).into(),
        ));
    }

    // "Home" rather than "Default Location": this is a single saved place
    // (where the app opens showing conditions for), not a location picker --
    // multi-location support is tracked separately (issue #5).
    let location_section = section("\u{2302} Home", location_column.into());

    let appearance_section = section(
        "\u{263e} Appearance & Refresh",
        column![
            toggler(state.dark_mode)
                .label("Dark mode")
                .on_toggle(Message::DarkModeToggled),
            toggler(state.use_fahrenheit)
                .label("Use Fahrenheit (\u{b0}F)")
                .on_toggle(Message::UnitsToggled),
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
