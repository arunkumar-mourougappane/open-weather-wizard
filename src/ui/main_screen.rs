//! # Main Screen
//!
//! Renders the current-conditions view: weather icon, location, temperature,
//! description, and humidity, plus Loading/Error states. This is the content of
//! the app's main window (see `src/app.rs::view`).

use std::time::Instant;

use iced::widget::{button, column, container, row, scrollable, space, text, tooltip};
use iced::{Alignment, Color, Element, Font, Length, Theme, font};

use crate::app::{AppState, ForecastStatus, Message, WeatherStatus};
use crate::config::WeatherApiProvider;
use crate::ui::temperature::{
    celsius_to_display, compass_direction, distance_to_display, distance_unit, format_local_time,
    pressure_to_display, pressure_unit, speed_to_display, speed_unit, unit_symbol,
};
use crate::ui::transition::ValueTracker;
use crate::ui::{forecast_row, icons, location_switcher, skeleton, style};
use crate::weather_api::alerts::{AlertSeverity, WeatherAlert};
use crate::weather_api::forecast::ForecastDay;
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

    let content: Element<'_, Message> = if let Some(index) = state.selected_forecast_day
        && let Some(forecast) = state.forecast.data()
        && let Some(day) = forecast.days.get(index)
    {
        column![
            row![
                hero_view_forecast(day, state.config.use_fahrenheit),
                stats_view_forecast(day, state.config.use_fahrenheit),
            ]
            .spacing(28)
            .align_y(Alignment::Start),
            button(text("\u{2190} Back to current conditions").size(12))
                .on_press(Message::ForecastDaySelected(0))
                .style(style::link_button)
        ]
        .spacing(12)
        .align_x(Alignment::Center)
        .width(Length::Fill)
        .into()
    } else if let Some(weather_data) = state.weather.data() {
        let Some(weather) = weather_data.weather.first() else {
            return text("No weather data available").into();
        };

        let current_location = state.config.current_location();
        let location_text = if !current_location.state.is_empty() {
            format!("{}, {}", weather_data.name, current_location.state)
        } else {
            weather_data.name.clone()
        };

        let mut card = column![
            alerts_view(&state.alerts),
            row![
                hero_view(
                    weather_data,
                    weather,
                    location_text,
                    state.config.use_fahrenheit,
                    &state.value_tracker
                ),
                stats_view(
                    weather_data,
                    state.config.use_fahrenheit,
                    &state.value_tracker
                ),
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

    let mut layout = column![toolbar].spacing(16).padding(16);

    if let Some(switcher) = location_switcher::view(state) {
        layout = layout.push(switcher);
    }

    layout = layout.push(
        container(content)
            .width(Length::Fill)
            .center_x(Length::Fill)
            .padding(24)
            .style(style::panel),
    );

    if let Some(forecast) = forecast_row::view(
        &state.forecast,
        state.config.use_fahrenheit,
        &state.value_tracker,
        state.selected_forecast_day,
    ) {
        layout = layout.push(forecast);
    } else if matches!(state.forecast, ForecastStatus::Loading) {
        layout = layout.push(skeleton::forecast_row());
    }

    // If the window is shorter than the content needs (a narrow custom resize,
    // or a display with unusual scaling), scroll instead of letting iced
    // silently squeeze fixed-size widgets like the animated icons into
    // whatever space is left -- that squeeze is what actually distorted them,
    // not the icons' own sizing. The provider ribbon sits outside the
    // scrollable, as a sibling `column` entry, so it stays pinned to the
    // bottom of the window rather than scrolling away with the content.
    column![
        scrollable(layout).height(Length::Fill),
        provider_ribbon(&state.config.weather_provider),
    ]
    .into()
}

fn alerts_view(alerts: &[WeatherAlert]) -> Element<'_, Message> {
    if alerts.is_empty() {
        return iced::widget::Space::new().into();
    }

    let mut layout = column![].spacing(8).width(Length::Fill);

    for alert in alerts {
        let is_severe = matches!(
            alert.severity,
            AlertSeverity::Extreme | AlertSeverity::Severe
        );
        let icon = if is_severe { "\u{26A0}" } else { "\u{24D8}" };
        let text_style = if is_severe {
            style::danger
        } else {
            style::warning
        };

        let mut banner_content = column![
            row![
                text(icon).size(16).style(text_style),
                text(&alert.title).size(14).font(BOLD).style(text_style),
            ]
            .spacing(8)
            .align_y(Alignment::Center),
        ]
        .spacing(6);

        if !alert.safety_recommendations.is_empty() {
            for recommendation in &alert.safety_recommendations {
                banner_content =
                    banner_content.push(text(recommendation).size(12).style(style::muted));
            }
        }

        let alert_banner = container(banner_content)
            .padding(12)
            .width(Length::Fill)
            .style(move |theme: &Theme| {
                let accent_color = if is_severe {
                    style::danger(theme).color.unwrap_or(Color::BLACK)
                } else {
                    style::warning(theme).color.unwrap_or(Color::BLACK)
                };
                container::Style {
                    background: Some(iced::Background::Color(Color {
                        a: 0.1,
                        ..accent_color
                    })),
                    border: iced::Border {
                        color: accent_color,
                        width: 1.0,
                        radius: 8.0.into(),
                    },
                    ..container::Style::default()
                }
            });

        layout = layout.push(alert_banner);
    }

    layout.into()
}

/// A thin footer strip naming whichever provider is currently powering the
/// displayed data, linked to that provider's own homepage -- config-driven
/// rather than tracked per-fetch, since a fetch always uses whatever
/// provider is currently configured (there's no per-request override). Uses
/// its own display label rather than `WeatherApiProvider`'s `Display` impl
/// (which renders "OpenWeather" as one word, matching the service's own
/// branding elsewhere, e.g. Preferences).
fn provider_ribbon(provider: &WeatherApiProvider) -> Element<'_, Message> {
    let (label, homepage) = match provider {
        WeatherApiProvider::OpenWeather => ("Open Weather", "https://openweathermap.org/"),
        WeatherApiProvider::GoogleWeather => (
            "Google Weather",
            "https://mapsplatform.google.com/maps-products/weather/",
        ),
    };

    container(
        row![
            space::horizontal(),
            text("Powered by").size(11).style(style::muted),
            button(text(label).size(11))
                .on_press(Message::OpenUrl(homepage.to_string()))
                .style(style::link_button)
                .padding(0),
        ]
        .spacing(4)
        .align_y(Alignment::Center),
    )
    .width(Length::Fill)
    .padding(6)
    .style(style::ribbon)
    .into()
}

/// The left-hand hero: icon, location, big temperature, and a short
/// description -- the one thing a glance at the app should land on first.
fn hero_view<'a>(
    weather_data: &'a ApiResponse,
    weather: &'a Weather,
    location_text: String,
    use_fahrenheit: bool,
    tracker: &ValueTracker,
) -> Element<'a, Message> {
    let symbol = get_weather_symbol(&weather.main);
    let unit = unit_symbol(use_fahrenheit);
    let temp = celsius_to_display(weather_data.main.temp, use_fahrenheit);

    column![
        icons::view(symbol, 108.0),
        text(location_text).size(20).font(BOLD),
        tracker.cross_fade(
            "temp",
            format!("{:.1}{unit}", temp),
            38,
            BOLD,
            style::accent,
        ),
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

/// The hero for a selected forecast day's detail view -- same shape as
/// `hero_view`, but a forecast day only has a hi/lo range, not one "right
/// now" reading, so the big accent number is that range instead of a
/// single temperature.
fn hero_view_forecast(day: &ForecastDay, use_fahrenheit: bool) -> Element<'_, Message> {
    let unit = unit_symbol(use_fahrenheit);
    let temp_max = celsius_to_display(day.temp_max, use_fahrenheit);
    let temp_min = celsius_to_display(day.temp_min, use_fahrenheit);

    column![
        icons::view(day.symbol, 108.0),
        text(day.date.clone()).size(20).font(BOLD),
        text(format!("{:.0}{unit} / {:.0}{unit}", temp_max, temp_min))
            .size(34)
            .font(BOLD)
            .style(style::accent),
        text(day.description.clone())
            .size(15)
            .font(ITALIC)
            .style(style::muted),
    ]
    .spacing(6)
    .align_x(Alignment::Center)
    .width(Length::Shrink)
    .into()
}

/// The detail grid for a selected forecast day: feels-like, humidity, wind,
/// pressure, visibility, and chance of rain -- everything `ForecastDay`
/// carries. Sunrise/sunset are omitted (only available for today, from live
/// current conditions, not per forecast day); unlike `stats_view`, values
/// here aren't cross-faded -- this panel only appears while a day is
/// selected, not during the always-on 30s ambient refresh.
fn stats_view_forecast(day: &ForecastDay, use_fahrenheit: bool) -> Element<'_, Message> {
    let unit = unit_symbol(use_fahrenheit);
    let feels_like = celsius_to_display(day.feels_like, use_fahrenheit);
    let wind_speed = speed_to_display(day.wind_speed, use_fahrenheit);
    let wind_unit = speed_unit(use_fahrenheit);
    let compass = compass_direction(day.wind_deg);
    let visibility = distance_to_display(day.visibility as f64, use_fahrenheit);
    let visibility_unit = distance_unit(use_fahrenheit);
    let pressure = pressure_to_display(day.pressure, use_fahrenheit);
    let pressure_unit_str = pressure_unit(use_fahrenheit);
    let pressure_precision = if use_fahrenheit { 2 } else { 0 };

    column![
        row![
            stat_chip(
                "\u{2248}",
                style::STAT_FEELS_LIKE,
                "Feels like",
                text(format!("{:.0}{unit}", feels_like))
                    .size(15)
                    .font(BOLD)
                    .into(),
            ),
            stat_chip(
                "\u{2614}",
                style::STAT_HUMIDITY,
                "Humidity",
                text(format!("{}%", day.humidity))
                    .size(15)
                    .font(BOLD)
                    .into(),
            ),
        ]
        .spacing(10),
        row![
            stat_chip(
                "\u{2197}",
                style::STAT_WIND,
                "Wind",
                text(format!("{:.0} {wind_unit} {compass}", wind_speed))
                    .size(15)
                    .font(BOLD)
                    .into(),
            ),
            stat_chip(
                "\u{2696}",
                style::STAT_PRESSURE,
                "Pressure",
                text(format!(
                    "{:.*} {pressure_unit_str}",
                    pressure_precision, pressure
                ))
                .size(15)
                .font(BOLD)
                .into(),
            ),
        ]
        .spacing(10),
        row![
            stat_chip(
                "\u{25ce}",
                style::STAT_VISIBILITY,
                "Visibility",
                text(format!("{:.1} {visibility_unit}", visibility))
                    .size(15)
                    .font(BOLD)
                    .into(),
            ),
            stat_chip(
                "\u{2602}",
                style::STAT_POP,
                "Chance of rain",
                text(format!("{:.0}%", day.pop * 100.0))
                    .size(15)
                    .font(BOLD)
                    .into(),
            ),
        ]
        .spacing(10),
    ]
    .spacing(10)
    .width(Length::Fill)
    .into()
}

/// The right-hand detail grid: feels-like, humidity, wind, pressure,
/// visibility, today's high/low, and sunrise/sunset -- laid out as a 2x4
/// grid of color-coded chips so the extra data reads as scannable stats
/// rather than another wall of text.
fn stats_view<'a>(
    weather_data: &'a ApiResponse,
    use_fahrenheit: bool,
    tracker: &ValueTracker,
) -> Element<'a, Message> {
    let unit = unit_symbol(use_fahrenheit);
    let feels_like = celsius_to_display(weather_data.main.feels_like, use_fahrenheit);
    let temp_min = celsius_to_display(weather_data.main.temp_min, use_fahrenheit);
    let temp_max = celsius_to_display(weather_data.main.temp_max, use_fahrenheit);

    let wind_speed = speed_to_display(weather_data.wind.speed, use_fahrenheit);
    let wind_unit = speed_unit(use_fahrenheit);
    let compass = compass_direction(weather_data.wind.deg);

    let visibility = distance_to_display(weather_data.visibility as f64, use_fahrenheit);
    let visibility_unit = distance_unit(use_fahrenheit);

    let pressure = pressure_to_display(weather_data.main.pressure, use_fahrenheit);
    let pressure_unit_str = pressure_unit(use_fahrenheit);
    let pressure_precision = if use_fahrenheit { 2 } else { 0 };

    let sunrise = format_local_time(weather_data.sys.sunrise, weather_data.timezone);
    let sunset = format_local_time(weather_data.sys.sunset, weather_data.timezone);

    column![
        row![
            stat_chip(
                "\u{2248}",
                style::STAT_FEELS_LIKE,
                "Feels like",
                tracker.cross_fade(
                    "feels_like",
                    format!("{:.0}{unit}", feels_like),
                    15,
                    BOLD,
                    style::default_text,
                ),
            ),
            stat_chip(
                "\u{2614}",
                style::STAT_HUMIDITY,
                "Humidity",
                tracker.cross_fade(
                    "humidity",
                    format!("{}%", weather_data.main.humidity),
                    15,
                    BOLD,
                    style::default_text,
                ),
            ),
        ]
        .spacing(10),
        row![
            stat_chip(
                "\u{2197}",
                style::STAT_WIND,
                "Wind",
                tracker.cross_fade(
                    "wind",
                    format!("{:.0} {wind_unit} {compass}", wind_speed),
                    15,
                    BOLD,
                    style::default_text,
                ),
            ),
            stat_chip(
                "\u{2696}",
                style::STAT_PRESSURE,
                "Pressure",
                tracker.cross_fade(
                    "pressure",
                    format!("{:.*} {pressure_unit_str}", pressure_precision, pressure),
                    15,
                    BOLD,
                    style::default_text,
                ),
            ),
        ]
        .spacing(10),
        row![
            stat_chip(
                "\u{25ce}",
                style::STAT_VISIBILITY,
                "Visibility",
                tracker.cross_fade(
                    "visibility",
                    format!("{:.1} {visibility_unit}", visibility),
                    15,
                    BOLD,
                    style::default_text,
                ),
            ),
            stat_chip(
                "\u{21c5}",
                style::STAT_RANGE,
                "High / Low",
                tracker.cross_fade(
                    "high_low",
                    format!("{:.0}{unit} / {:.0}{unit}", temp_max, temp_min),
                    15,
                    BOLD,
                    style::default_text,
                ),
            ),
        ]
        .spacing(10),
        row![
            stat_chip(
                "\u{2600}",
                style::STAT_SUNRISE,
                "Sunrise",
                tracker.cross_fade("sunrise", sunrise, 15, BOLD, style::default_text),
            ),
            stat_chip(
                "\u{263e}",
                style::STAT_SUNSET,
                "Sunset",
                tracker.cross_fade("sunset", sunset, 15, BOLD, style::default_text),
            ),
        ]
        .spacing(10),
    ]
    .spacing(10)
    .width(Length::Fill)
    .into()
}

/// A single detail stat: a round tinted glyph badge next to a label/value
/// pair, in a card matching the forecast row's visual language. `value` is
/// an `Element` rather than a plain string so callers can pass either plain
/// text or a `ValueTracker::cross_fade` result.
fn stat_chip<'a>(
    glyph: &'static str,
    color: Color,
    label: &'static str,
    value: Element<'a, Message>,
) -> Element<'a, Message> {
    let badge = container(text(glyph).size(15))
        .center(30)
        .style(style::stat_badge(color));

    container(
        row![
            badge,
            column![text(label).size(11).style(style::muted), value].spacing(2),
        ]
        .spacing(10)
        .align_y(Alignment::Center),
    )
    .padding(10)
    .width(Length::Fill)
    .style(style::day_card)
    .into()
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
