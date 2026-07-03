//! # Forecast Row
//!
//! A horizontally-scrollable row of day cards (icon + hi/lo + short description),
//! rendered below the current-conditions card on the main screen. Omitted entirely
//! while loading or on error (either already communicated elsewhere in the UI),
//! but a provider with no real forecast integration (e.g. the Google Weather
//! mock) gets an explicit muted hint rather than the row just silently
//! vanishing, which otherwise reads as a bug rather than a provider limitation.

use iced::widget::{column, container, responsive, row, scrollable, text};
use iced::{Alignment, Element, Font, Length, font};

use crate::app::{ForecastStatus, Message};
use crate::ui::temperature::{celsius_to_display, unit_symbol};
use crate::ui::{icons, style};
use crate::weather_api::forecast::ForecastDay;

const BOLD: Font = Font {
    weight: font::Weight::Bold,
    ..Font::DEFAULT
};

/// `day_card`'s content column width (100) plus its container's padding
/// (10 on each side).
const CARD_WIDTH: f32 = 120.0;
const CARD_SPACING: f32 = 12.0;

/// Total width `n` cards need laid out in a row with `CARD_SPACING` between
/// them (no trailing gap after the last card).
fn cards_width(n: usize) -> f32 {
    if n == 0 {
        return 0.0;
    }
    n as f32 * CARD_WIDTH + (n - 1) as f32 * CARD_SPACING
}

/// Tall enough for `day_card`'s content (date + 48px icon + hi/lo + short
/// description, plus its container's padding) with a little slack. Set
/// explicitly because `responsive` defaults to `Length::Fill` for height,
/// which would otherwise try to consume all remaining vertical space in the
/// column it sits in.
const ROW_HEIGHT: f32 = 140.0;

/// Renders the forecast row, or `None` if there's nothing to show at all
/// (loading with no prior data yet, or an error).
pub fn view(forecast: &ForecastStatus, use_fahrenheit: bool) -> Option<Element<'_, Message>> {
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
            let days = &response.days;

            // `scrollable` gives its content an *infinite* max-width limit
            // along the scrolling axis (so content is actually allowed to
            // exceed the viewport and scroll) -- which means a `Length::Fill`
            // container placed inside it never resolves to "the visible
            // viewport width" and can't be used to center content there.
            // `responsive` sidesteps this by measuring the real available
            // size at layout time: center a plain (non-scrolling) row when
            // the cards fit, or fall back to the hidden-scrollbar carousel
            // only once they don't.
            Some(
                responsive(move |size| {
                    let cards = || {
                        days.iter()
                            .enumerate()
                            .map(|(index, day)| day_card(day, index == 0, use_fahrenheit))
                    };

                    if cards_width(days.len()) <= size.width {
                        container(row(cards()).spacing(CARD_SPACING))
                            .center_x(Length::Fill)
                            .into()
                    } else {
                        // A carousel, not a document: the scrollbar
                        // track/thumb are hidden (Scrollbar::hidden() zeroes
                        // their width), but the row still scrolls via
                        // trackpad/mouse-wheel/click-drag -- hiding the
                        // scrollbar doesn't disable scrolling itself.
                        scrollable(row(cards()).spacing(CARD_SPACING))
                            .direction(scrollable::Direction::Horizontal(
                                scrollable::Scrollbar::hidden(),
                            ))
                            .width(Length::Fill)
                            .into()
                    }
                })
                .height(ROW_HEIGHT)
                .into(),
            )
        }
    }
}

fn day_card(day: &ForecastDay, is_today: bool, use_fahrenheit: bool) -> Element<'_, Message> {
    let date_label = if is_today {
        "Today".to_string()
    } else {
        day.date.clone()
    };

    let unit = unit_symbol(use_fahrenheit);
    let temp_max = celsius_to_display(day.temp_max, use_fahrenheit);
    let temp_min = celsius_to_display(day.temp_min, use_fahrenheit);

    container(
        column![
            text(date_label).size(13).font(BOLD).style(if is_today {
                style::accent
            } else {
                style::default_text
            }),
            icons::view(day.symbol, 48.0),
            text(format!("{:.0}{unit} / {:.0}{unit}", temp_max, temp_min))
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
