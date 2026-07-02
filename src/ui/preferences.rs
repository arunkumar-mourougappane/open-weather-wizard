//! # Preferences Screen
//!
//! Renders in its own OS window (opened by `Message::OpenPreferences` in `src/app.rs`),
//! matching the previous GTK version's transient preferences dialog. Owns its own
//! form-field state; the parent `AppState` intercepts `Save`/`Cancel` since only it
//! holds the persisted `AppConfig`/`ConfigManager`.

use iced::widget::{
    button, column, container, pick_list, row, scrollable, text, text_input, toggler,
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
        }
    }

    /// Writes the edited fields back into the shared `AppConfig`.
    pub fn apply_to(&self, config: &mut AppConfig) {
        config.weather_provider = self.provider.clone();
        if !self.token_input.is_empty() {
            config.set_api_token(&self.token_input);
        }
        config.location.city = self.city_input.clone();
        config.location.state = self.state_input.clone();
        config.location.country = self.country_input.clone();
        config.dark_mode = self.dark_mode;
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
        // Only OpenWeather actually needs a token; the Google Weather mock
        // provider works with none (see WeatherProvider::requires_api_key).
        if self.provider == WeatherApiProvider::OpenWeather && self.token_input.trim().is_empty() {
            errors.push("API Token is required for OpenWeather.".to_string());
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
    Save,
    Cancel,
}

/// Mutates field-edit messages; `Save`/`Cancel` are intercepted by the parent
/// `AppState::update` (see `src/app.rs`) since they need access to `AppConfig`.
pub fn update(state: &mut State, message: Message) {
    match message {
        Message::ProviderSelected(provider) => state.provider = provider,
        Message::TokenChanged(value) => state.token_input = value,
        Message::CityChanged(value) => state.city_input = value,
        Message::StateChanged(value) => state.state_input = value,
        Message::CountryChanged(value) => state.country_input = value,
        Message::DarkModeToggled(value) => state.dark_mode = value,
        Message::Save | Message::Cancel => {
            // Handled by the parent; nothing to do locally.
        }
    }
}

pub fn view(state: &State) -> Element<'_, Message> {
    let provider_section = section(
        "Weather Provider",
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
        ]
        .spacing(12)
        .into(),
    );

    let location_section = section(
        "Default Location",
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
        "Appearance",
        toggler(state.dark_mode)
            .label("Dark mode")
            .on_toggle(Message::DarkModeToggled)
            .into(),
    );

    let errors = state.validation_errors();

    let buttons = row![
        button("Cancel")
            .on_press(Message::Cancel)
            .style(style::secondary_button),
        button("Save")
            .on_press_maybe(errors.is_empty().then_some(Message::Save))
            .style(style::primary_button),
    ]
    .spacing(8)
    .align_y(Alignment::Center);

    let mut layout = column![
        text("Preferences").size(20).font(BOLD),
        provider_section,
        location_section,
        appearance_section,
    ]
    .spacing(16)
    .padding(20)
    .width(Length::Fill);

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
