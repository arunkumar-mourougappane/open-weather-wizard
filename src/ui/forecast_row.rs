//! # Forecast Row
//!
//! A horizontally-scrollable row of day cards (icon + hi/lo + short description),
//! rendered below the current-conditions card on the main screen. Omitted entirely
//! (not shown as an empty placeholder) when there's no forecast data -- e.g. for
//! the Google Weather mock provider, which has no real forecast integration.

use iced::widget::{column, container, row, scrollable, svg, text};
use iced::{Alignment, Element, Font, Length, font};

use crate::app::{ForecastStatus, Message};
use crate::ui::icons;
use crate::weather_api::forecast::ForecastDay;

const BOLD: Font = Font {
    weight: font::Weight::Bold,
    ..Font::DEFAULT
};

/// Renders the forecast row, or `None` if there's nothing to show (loading with
/// no prior data yet, an error, or an empty day list).
pub fn view(forecast: &ForecastStatus) -> Option<Element<'_, Message>> {
    match forecast {
        ForecastStatus::Loading => None,
        ForecastStatus::Error => None,
        ForecastStatus::Loaded(response) if response.days.is_empty() => None,
        ForecastStatus::Loaded(response) => {
            let cards = response.days.iter().map(day_card);
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

fn day_card(day: &ForecastDay) -> Element<'_, Message> {
    let handle = icons::handle_for(day.symbol);

    container(
        column![
            text(day.date.clone()).size(14).font(BOLD),
            svg(handle).width(48).height(48),
            text(format!(
                "{:.0}\u{b0} / {:.0}\u{b0}",
                day.temp_max, day.temp_min
            ))
            .size(14),
            text(day.description.clone()).size(12),
        ]
        .spacing(4)
        .align_x(Alignment::Center)
        .width(96),
    )
    .padding(8)
    .into()
}
