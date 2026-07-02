//! # Main Screen
//!
//! Renders the current-conditions view: weather icon, location, temperature,
//! description, and humidity, plus Loading/Error states. This is the content of
//! the app's main window (see `src/app.rs::view`).

use iced::widget::{button, column, container, row, space, text};
use iced::{Alignment, Color, Element, Font, Length, font};

use crate::app::{AppState, Message, WeatherStatus};
use crate::ui::{forecast_row, icons};
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
        text("Weather Wizard").size(20).font(BOLD),
        space::horizontal(),
        button("Preferences").on_press(Message::OpenPreferences),
        button("About").on_press(Message::OpenAbout),
    ]
    .spacing(8)
    .align_y(Alignment::Center);

    let content: Element<'_, Message> = match &state.weather {
        WeatherStatus::Loading => text("Fetching weather...").size(18).into(),
        WeatherStatus::Error(message) => text(format!("Error: {}", message))
            .size(16)
            .color(Color::from_rgb(0.8, 0.1, 0.1))
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

            column![
                icons::view(symbol, 128.0),
                text(location_text).size(24).font(BOLD),
                text(format!("{:.1}\u{b0}C", weather_data.main.temp))
                    .size(30)
                    .font(BOLD),
                text(weather.description.clone()).size(18).font(ITALIC),
                text(format!("Humidity: {}%", weather_data.main.humidity)).size(14),
            ]
            .spacing(6)
            .align_x(Alignment::Center)
            .into()
        }
    };

    let mut layout = column![
        toolbar,
        container(content)
            .width(Length::Fill)
            .center_x(Length::Fill)
            .padding(20),
    ]
    .spacing(12)
    .padding(12);

    if let Some(forecast) = forecast_row::view(&state.forecast) {
        layout = layout.push(forecast);
    }

    layout.into()
}
