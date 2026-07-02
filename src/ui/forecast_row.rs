//! # Forecast Row
//!
//! A horizontally-scrollable row of day cards (icon + hi/lo + short description),
//! rendered below the current-conditions card on the main screen. Omitted entirely
//! while loading or on error (either already communicated elsewhere in the UI),
//! but a provider with no real forecast integration (e.g. the Google Weather
//! mock) gets an explicit muted hint rather than the row just silently
//! vanishing, which otherwise reads as a bug rather than a provider limitation.

use iced::widget::{column, container, row, scrollable, text};
use iced::{Alignment, Element, Font, Length, font};

use crate::app::{ForecastStatus, Message};
use crate::ui::{icons, style};
use crate::weather_api::forecast::ForecastDay;

const BOLD: Font = Font {
    weight: font::Weight::Bold,
    ..Font::DEFAULT
};

/// Renders the forecast row, or `None` if there's nothing to show at all
/// (loading with no prior data yet, or an error).
pub fn view(forecast: &ForecastStatus) -> Option<Element<'_, Message>> {
    match forecast {
        ForecastStatus::Loading => None,
        ForecastStatus::Error => None,
        ForecastStatus::Loaded(response) if response.days.is_empty() => Some(
            text("Forecast not available for this provider")
                .size(13)
                .style(style::muted)
                .into(),
        ),
        ForecastStatus::Loaded(response) => {
            // OpenWeatherMap's forecast always starts from "now", so the
            // first aggregated day is definitionally today -- no date/time
            // crate needed to figure out which card that is.
            let cards = response
                .days
                .iter()
                .enumerate()
                .map(|(index, day)| day_card(day, index == 0));
            let cards_row = row(cards).spacing(12);

            Some(
                scrollable(cards_row)
                    .direction(scrollable::Direction::Horizontal(
                        scrollable::Scrollbar::default(),
                    ))
                    .width(Length::Fill)
                    .into(),
            )
        }
    }
}

fn day_card(day: &ForecastDay, is_today: bool) -> Element<'_, Message> {
    let date_label = if is_today {
        "Today".to_string()
    } else {
        day.date.clone()
    };

    container(
        column![
            text(date_label).size(13).font(BOLD).style(if is_today {
                style::accent
            } else {
                style::default_text
            }),
            icons::view(day.symbol, 48.0),
            text(format!(
                "{:.0}\u{b0} / {:.0}\u{b0}",
                day.temp_max, day.temp_min
            ))
            .size(14)
            .font(BOLD),
            text(day.description.clone()).size(12).style(style::muted),
        ]
        .spacing(6)
        .align_x(Alignment::Center)
        .width(100),
    )
    .padding(10)
    .style(if is_today {
        style::day_card_today
    } else {
        style::day_card
    })
    .into()
}
