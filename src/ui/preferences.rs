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

#[derive(Debug, Clone)]
pub struct State {
    pub provider: WeatherApiProvider,
    pub token_input: String,
    pub city_input: String,
    pub state_input: String,
    pub country_input: String,
    pub dark_mode: bool,
    pub use_fahrenheit: bool,
    /// Set by `app::boot` when this window was opened automatically because
    /// no config file existed yet (see `ConfigManager::config_exists`).
    /// Purely cosmetic -- swaps in a welcome banner (`view`) and the
    /// window's title (`app::title`); every field and validation rule
    /// behaves identically either way.
    pub is_first_run: bool,
}

impl State {
    pub fn from_config(config: &AppConfig) -> Self {
        Self {
            provider: config.weather_provider.clone(),
            token_input: config.get_api_token().unwrap_or_default(),
            city_input: config.location.city.clone(),
            state_input: config.location.state.clone(),
            country_input: config.location.country.clone(),
            dark_mode: config.dark_mode,
            use_fahrenheit: config.use_fahrenheit,
            is_first_run: false,
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
        // Both providers require a token (see WeatherProvider::requires_api_key).
        if self.token_input.trim().is_empty() {
            errors.push(format!("API Token is required for {}.", self.provider));
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
    /// The "Get an API key" link -- intercepted by the parent (see
    /// `src/app.rs`) and turned into `Message::OpenUrl`, since opening a
    /// browser is an app-level concern, not something this module does
    /// itself.
    OpenUrl(String),
    Save,
    Cancel,
}

/// Mutates field-edit messages; `Save`/`Cancel`/`OpenUrl` are intercepted by
/// the parent `AppState::update` (see `src/app.rs`) since they need access to
/// `AppConfig`/the OS's URL opener respectively.
pub fn update(state: &mut State, message: Message) {
    match message {
        Message::ProviderSelected(provider) => state.provider = provider,
        Message::TokenChanged(value) => state.token_input = value,
        Message::CityChanged(value) => state.city_input = value,
        Message::StateChanged(value) => state.state_input = value,
        Message::CountryChanged(value) => state.country_input = value,
        Message::DarkModeToggled(value) => state.dark_mode = value,
        Message::UnitsToggled(value) => state.use_fahrenheit = value,
        Message::OpenUrl(_) | Message::Save | Message::Cancel => {
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

    let provider_section = section(
        "\u{2699} Weather Provider",
        column![
            labeled_row(
                "Provider:",
                pick_list(
                    PROVIDERS,
                    Some(state.provider.clone()),
                    Message::ProviderSelected
                )
                .into()
            ),
            labeled_row(
                "API Token:",
                text_input("Enter your API token", &state.token_input)
                    .secure(true)
                    .on_input(Message::TokenChanged)
                    .into()
            ),
            api_key_hint_row(hint_label, hint_url),
        ]
        .spacing(12)
        .into(),
    );

    let location_section = section(
        "\u{25ce} Default Location",
        column![
            labeled_row(
                "City:",
                text_input("Enter city name", &state.city_input)
                    .on_input(Message::CityChanged)
                    .into()
            ),
            labeled_row(
                "State/Province:",
                text_input("Enter state or province", &state.state_input)
                    .on_input(Message::StateChanged)
                    .into()
            ),
            labeled_row(
                "Country:",
                text_input("Enter country code (e.g., US, CA)", &state.country_input)
                    .on_input(Message::CountryChanged)
                    .into()
            ),
        ]
        .spacing(12)
        .into(),
    );

    let appearance_section = section(
        "\u{263e} Appearance",
        column![
            toggler(state.dark_mode)
                .label("Dark mode")
                .on_toggle(Message::DarkModeToggled),
            toggler(state.use_fahrenheit)
                .label("Use Fahrenheit (\u{b0}F)")
                .on_toggle(Message::UnitsToggled),
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
                     default location to get started."
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
    .into()
}
