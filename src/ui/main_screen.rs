//! # Main Screen
//!
//! Renders the current-conditions view: weather icon, location, temperature,
//! description, and humidity, plus Loading/Error states. This is the content of
//! the app's main window (see `src/app.rs::view`).

use std::time::Instant;

use iced::widget::{button, column, container, row, scrollable, space, text};
use iced::{Alignment, Element, Font, Length, font};

use crate::app::{AppState, Message, WeatherStatus};
use crate::ui::{forecast_row, icons, style};
use crate::weather_api::openweather_api::get_weather_symbol;

const BOLD: Font = Font {
    weight: font::Weight::Bold,
    ..Font::DEFAULT
};

const ITALIC: Font = Font {
    style: font::Style::Italic,
    ..Font::DEFAULT
};

pub fn view(state: &AppState) -> Element<'_, Message> {
    let toolbar = row![
        text("Weather Wizard")
            .size(20)
            .font(BOLD)
            .style(style::accent),
        space::horizontal(),
        button("Preferences")
            .on_press(Message::OpenPreferences)
            .style(style::secondary_button),
        button("About")
            .on_press(Message::OpenAbout)
            .style(style::secondary_button),
    ]
    .spacing(8)
    .align_y(Alignment::Center);

    let content: Element<'_, Message> = match &state.weather {
        WeatherStatus::Loading => text("Fetching weather...")
            .size(18)
            .style(style::muted)
            .into(),
        WeatherStatus::Error(message) => text(format!("Error: {}", message))
            .size(16)
            .style(style::danger)
            .into(),
        WeatherStatus::Loaded(weather_data) => {
            let Some(weather) = weather_data.weather.first() else {
                return text("No weather data available").into();
            };
            let symbol = get_weather_symbol(&weather.main);

            let location_text = if !state.config.location.state.is_empty() {
                format!("{}, {}", weather_data.name, state.config.location.state)
            } else {
                weather_data.name.clone()
            };

            let mut card = column![
                icons::view(symbol, 128.0),
                text(location_text).size(24).font(BOLD),
                text(format!("{:.1}\u{b0}C", weather_data.main.temp))
                    .size(34)
                    .font(BOLD)
                    .style(style::accent),
                text(weather.description.clone()).size(18).font(ITALIC),
                text(format!("Humidity: {}%", weather_data.main.humidity))
                    .size(14)
                    .style(style::muted),
            ]
            .spacing(8)
            .align_x(Alignment::Center);

            if let Some(label) = updated_label(state.last_updated) {
                card = card.push(text(label).size(12).style(style::muted));
            }

            card.into()
        }
    };

    let mut layout = column![
        toolbar,
        container(content)
            .width(Length::Fill)
            .center_x(Length::Fill)
            .padding(24)
            .style(style::panel),
    ]
    .spacing(16)
    .padding(16);

    if let Some(forecast) = forecast_row::view(&state.forecast) {
        layout = layout.push(forecast);
    }

    // If the window is shorter than the content needs (a narrow custom resize,
    // or a display with unusual scaling), scroll instead of letting iced
    // silently squeeze fixed-size widgets like the animated icons into
    // whatever space is left -- that squeeze is what actually distorted them,
    // not the icons' own sizing.
    scrollable(layout).height(Length::Fill).into()
}

/// Formats "Updated just now" / "Updated Xm ago" from the last successful
/// fetch time. `None` (nothing fetched yet) renders nothing.
fn updated_label(last_updated: Option<Instant>) -> Option<String> {
    let elapsed = last_updated?.elapsed();
    let label = if elapsed.as_secs() < 60 {
        "Updated just now".to_string()
    } else {
        format!("Updated {}m ago", elapsed.as_secs() / 60)
    };
    Some(label)
}
