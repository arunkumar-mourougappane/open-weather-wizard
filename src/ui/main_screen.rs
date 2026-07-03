//! # Main Screen
//!
//! Renders the current-conditions view: weather icon, location, temperature,
//! description, and humidity, plus Loading/Error states. This is the content of
//! the app's main window (see `src/app.rs::view`).

use std::time::Instant;

use iced::widget::{button, column, container, row, scrollable, space, text, tooltip};
use iced::{Alignment, Color, Element, Font, Length, font};

use crate::app::{AppState, ForecastStatus, Message, WeatherStatus};
use crate::ui::temperature::{
    celsius_to_display, distance_to_display, distance_unit, speed_to_display, speed_unit,
    unit_symbol,
};
use crate::ui::{forecast_row, icons, skeleton, style};
use crate::weather_api::openweather_api::{ApiResponse, Weather, get_weather_symbol};

const BOLD: Font = Font {
    weight: font::Weight::Bold,
    ..Font::DEFAULT
};

const ITALIC: Font = Font {
    style: font::Style::Italic,
    ..Font::DEFAULT
};

pub fn view(state: &AppState) -> Element<'_, Message> {
    let is_refreshing = matches!(
        state.weather,
        WeatherStatus::Loading | WeatherStatus::Refreshing(_)
    );

    let toolbar = row![
        text("Weather Wizard")
            .size(20)
            .font(BOLD)
            .style(style::accent),
        space::horizontal(),
        // Disabled while a fetch is already in flight, both to avoid
        // piling up redundant requests and as a small "yes, it's working"
        // signal beyond the spinner in the panel below.
        toolbar_button(
            "\u{21bb}",
            "Refresh",
            (!is_refreshing).then_some(Message::RefreshRequested),
        ),
        toolbar_button("\u{2699}", "Preferences", Some(Message::OpenPreferences)),
        toolbar_button("\u{24d8}", "About", Some(Message::OpenAbout)),
    ]
    .spacing(8)
    .align_y(Alignment::Center);

    let content: Element<'_, Message> = if let Some(weather_data) = state.weather.data() {
        let Some(weather) = weather_data.weather.first() else {
            return text("No weather data available").into();
        };

        let location_text = if !state.config.location.state.is_empty() {
            format!("{}, {}", weather_data.name, state.config.location.state)
        } else {
            weather_data.name.clone()
        };

        let mut card = column![
            row![
                hero_view(
                    weather_data,
                    weather,
                    location_text,
                    state.config.use_fahrenheit
                ),
                stats_view(weather_data, state.config.use_fahrenheit),
            ]
            .spacing(28)
            .align_y(Alignment::Start)
        ]
        .spacing(12)
        .align_x(Alignment::Center)
        .width(Length::Fill);

        if let Some(label) = updated_label(state.last_updated) {
            card = card.push(text(label).size(12).style(style::muted));
        }

        card.into()
    } else {
        match &state.weather {
            WeatherStatus::Error(message) => column![
                text(format!("Error: {}", message))
                    .size(16)
                    .style(style::danger),
                button("Retry")
                    .on_press(Message::RefreshRequested)
                    .style(style::primary_button),
            ]
            .spacing(12)
            .align_x(Alignment::Center)
            .into(),
            // `Loading` is the only remaining case reachable here: `.data()`
            // returned `None`, which only `Loading` and `Error` do, and
            // `Error` was just matched above. Only reachable before the
            // very first successful fetch (or retrying after a first-load
            // error) -- see `WeatherStatus`'s docs in `src/app.rs`.
            _ => row![skeleton::hero(), skeleton::stats()]
                .spacing(28)
                .align_y(Alignment::Start)
                .into(),
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

    if let Some(forecast) = forecast_row::view(&state.forecast, state.config.use_fahrenheit) {
        layout = layout.push(forecast);
    } else if matches!(state.forecast, ForecastStatus::Loading) {
        layout = layout.push(skeleton::forecast_row());
    }

    // If the window is shorter than the content needs (a narrow custom resize,
    // or a display with unusual scaling), scroll instead of letting iced
    // silently squeeze fixed-size widgets like the animated icons into
    // whatever space is left -- that squeeze is what actually distorted them,
    // not the icons' own sizing.
    scrollable(layout).height(Length::Fill).into()
}

/// The left-hand hero: icon, location, big temperature, and a short
/// description -- the one thing a glance at the app should land on first.
fn hero_view<'a>(
    weather_data: &'a ApiResponse,
    weather: &'a Weather,
    location_text: String,
    use_fahrenheit: bool,
) -> Element<'a, Message> {
    let symbol = get_weather_symbol(&weather.main);
    let unit = unit_symbol(use_fahrenheit);
    let temp = celsius_to_display(weather_data.main.temp, use_fahrenheit);

    column![
        icons::view(symbol, 108.0),
        text(location_text).size(20).font(BOLD),
        text(format!("{:.1}{unit}", temp))
            .size(38)
            .font(BOLD)
            .style(style::accent),
        text(weather.description.clone())
            .size(15)
            .font(ITALIC)
            .style(style::muted),
    ]
    .spacing(6)
    .align_x(Alignment::Center)
    .width(Length::Shrink)
    .into()
}

/// The right-hand detail grid: feels-like, humidity, wind, pressure,
/// visibility, today's high/low, and sunrise/sunset -- laid out as a 2x4
/// grid of color-coded chips so the extra data reads as scannable stats
/// rather than another wall of text.
fn stats_view(weather_data: &ApiResponse, use_fahrenheit: bool) -> Element<'_, Message> {
    let unit = unit_symbol(use_fahrenheit);
    let feels_like = celsius_to_display(weather_data.main.feels_like, use_fahrenheit);
    let temp_min = celsius_to_display(weather_data.main.temp_min, use_fahrenheit);
    let temp_max = celsius_to_display(weather_data.main.temp_max, use_fahrenheit);

    let wind_speed = speed_to_display(weather_data.wind.speed, use_fahrenheit);
    let wind_unit = speed_unit(use_fahrenheit);
    let compass = compass_direction(weather_data.wind.deg);

    let visibility = distance_to_display(weather_data.visibility as f64, use_fahrenheit);
    let visibility_unit = distance_unit(use_fahrenheit);

    let sunrise = format_local_time(weather_data.sys.sunrise, weather_data.timezone);
    let sunset = format_local_time(weather_data.sys.sunset, weather_data.timezone);

    column![
        row![
            stat_chip(
                "\u{2248}",
                style::STAT_FEELS_LIKE,
                "Feels like",
                format!("{:.0}{unit}", feels_like),
            ),
            stat_chip(
                "\u{2614}",
                style::STAT_HUMIDITY,
                "Humidity",
                format!("{}%", weather_data.main.humidity),
            ),
        ]
        .spacing(10),
        row![
            stat_chip(
                "\u{2197}",
                style::STAT_WIND,
                "Wind",
                format!("{:.0} {wind_unit} {compass}", wind_speed),
            ),
            stat_chip(
                "\u{2696}",
                style::STAT_PRESSURE,
                "Pressure",
                format!("{} hPa", weather_data.main.pressure),
            ),
        ]
        .spacing(10),
        row![
            stat_chip(
                "\u{25ce}",
                style::STAT_VISIBILITY,
                "Visibility",
                format!("{:.1} {visibility_unit}", visibility),
            ),
            stat_chip(
                "\u{21c5}",
                style::STAT_RANGE,
                "High / Low",
                format!("{:.0}{unit} / {:.0}{unit}", temp_max, temp_min),
            ),
        ]
        .spacing(10),
        row![
            stat_chip("\u{2600}", style::STAT_SUNRISE, "Sunrise", sunrise),
            stat_chip("\u{263e}", style::STAT_SUNSET, "Sunset", sunset),
        ]
        .spacing(10),
    ]
    .spacing(10)
    .width(Length::Fill)
    .into()
}

/// A single detail stat: a round tinted glyph badge next to a label/value
/// pair, in a card matching the forecast row's visual language.
fn stat_chip<'a>(
    glyph: &'static str,
    color: Color,
    label: &'static str,
    value: String,
) -> Element<'a, Message> {
    let badge = container(text(glyph).size(15))
        .center(30)
        .style(style::stat_badge(color));

    container(
        row![
            badge,
            column![
                text(label).size(11).style(style::muted),
                text(value).size(15).font(BOLD),
            ]
            .spacing(2),
        ]
        .spacing(10)
        .align_y(Alignment::Center),
    )
    .padding(10)
    .width(Length::Fill)
    .style(style::day_card)
    .into()
}

/// Meteorological degrees (0 = due north, clockwise) to a 16-point compass
/// abbreviation.
fn compass_direction(deg: i64) -> &'static str {
    const DIRECTIONS: [&str; 16] = [
        "N", "NNE", "NE", "ENE", "E", "ESE", "SE", "SSE", "S", "SSW", "SW", "WSW", "W", "WNW",
        "NW", "NNW",
    ];
    let normalized = deg.rem_euclid(360) as f64;
    let index = ((normalized / 22.5) + 0.5) as usize % 16;
    DIRECTIONS[index]
}

/// Renders a Unix timestamp as a local 12-hour clock time using the API's
/// `timezone` offset (seconds from UTC) -- avoids pulling in a date/time
/// crate for what's ultimately just "HH:MM AM/PM".
fn format_local_time(unix_ts: i64, tz_offset_secs: i64) -> String {
    let local_secs = (unix_ts + tz_offset_secs).rem_euclid(86_400);
    let hours24 = local_secs / 3600;
    let minutes = (local_secs % 3600) / 60;
    let period = if hours24 < 12 { "AM" } else { "PM" };
    let hours12 = match hours24 % 12 {
        0 => 12,
        h => h,
    };
    format!("{hours12}:{minutes:02} {period}")
}

/// A square icon-only toolbar button, with the action's name shown in a
/// hover tooltip since a bare glyph alone isn't self-explanatory.
fn toolbar_button<'a>(
    glyph: &'a str,
    label: &'a str,
    on_press: Option<Message>,
) -> Element<'a, Message> {
    let btn = button(text(glyph).size(18).align_x(Alignment::Center))
        .width(36)
        .height(36)
        .on_press_maybe(on_press)
        .style(style::secondary_button);

    tooltip(btn, text(label).size(12), tooltip::Position::Bottom)
        .style(style::panel)
        .into()
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
