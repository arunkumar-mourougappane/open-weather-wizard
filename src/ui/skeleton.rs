//! # Loading Skeleton
//!
//! Placeholder shapes shown only before the very first successful fetch
//! (see `WeatherStatus`/`ForecastStatus` in `src/app.rs` -- once real data
//! has ever loaded, `Refreshing` keeps it on screen instead of falling back
//! to `Loading`, so this is genuinely a first-launch-only screen).
//!
//! Each block pulses its opacity via the same "shared `LazyLock<Instant>` +
//! pure function of elapsed time, redrawn on the existing `AnimationTick`
//! subscription" pattern used by the animated weather icons (`ui::icons`)
//! and the loading spinner (`ui::spinner`) -- no new timer or state needed.

use std::f32::consts::TAU;
use std::sync::LazyLock;
use std::time::Instant;

use iced::widget::{Space, column, container, row};
use iced::{Alignment, Background, Border, Color, Element, Length, Theme};

use crate::ui::forecast_row::{CARD_SPACING, ROW_HEIGHT};
use crate::ui::style;

static PULSE_START: LazyLock<Instant> = LazyLock::new(Instant::now);
const PULSE_PERIOD_SECS: f32 = 1.4;
const PULSE_MIN_ALPHA: f32 = 0.35;
const PULSE_MAX_ALPHA: f32 = 0.70;

/// A gently pulsing gray block standing in for real content.
fn skeleton_block<'a, Message: 'a>(
    width: impl Into<Length>,
    height: f32,
    radius: f32,
) -> Element<'a, Message> {
    container(Space::new())
        .width(width)
        .height(height)
        .style(move |theme: &Theme| {
            let phase = PULSE_START.elapsed().as_secs_f32() / PULSE_PERIOD_SECS * TAU;
            let mid = (PULSE_MIN_ALPHA + PULSE_MAX_ALPHA) / 2.0;
            let swing = (PULSE_MAX_ALPHA - PULSE_MIN_ALPHA) / 2.0;
            let alpha = mid + swing * phase.sin();

            container::Style {
                background: Some(Background::Color(Color {
                    a: alpha,
                    ..theme.extended_palette().background.strong.color
                })),
                border: Border {
                    radius: radius.into(),
                    ..Border::default()
                },
                ..container::Style::default()
            }
        })
        .into()
}

/// Stands in for `main_screen::hero_view`: icon, location, big temperature,
/// description.
pub fn hero<'a, Message: 'a>() -> Element<'a, Message> {
    column![
        skeleton_block(108.0, 108.0, 54.0),
        skeleton_block(140.0, 20.0, 4.0),
        skeleton_block(160.0, 38.0, 6.0),
        skeleton_block(180.0, 15.0, 4.0),
    ]
    .spacing(6)
    .align_x(Alignment::Center)
    .width(Length::Shrink)
    .into()
}

/// Stands in for `main_screen::stats_view`: a 2x4 grid of stat chips.
pub fn stats<'a, Message: 'a>() -> Element<'a, Message> {
    let chip_row = || {
        row![
            skeleton_block(Length::Fill, 50.0, 8.0),
            skeleton_block(Length::Fill, 50.0, 8.0),
        ]
        .spacing(10)
    };

    column![chip_row(), chip_row(), chip_row(), chip_row()]
        .spacing(10)
        .width(Length::Fill)
        .into()
}

/// Stands in for `forecast_row::view`: a row of day cards, sized identically
/// to `forecast_row::day_card` so nothing reflows once real data arrives.
pub fn forecast_row<'a, Message: 'a>() -> Element<'a, Message> {
    let card = || {
        container(
            column![
                skeleton_block(60.0, 13.0, 3.0),
                skeleton_block(48.0, 48.0, 24.0),
                skeleton_block(70.0, 14.0, 3.0),
                skeleton_block(80.0, 12.0, 3.0),
            ]
            .spacing(6)
            .align_x(Alignment::Center)
            .width(100),
        )
        .padding(10)
        .style(style::day_card)
    };

    container(row((0..5).map(|_| card().into())).spacing(CARD_SPACING))
        .height(ROW_HEIGHT)
        .center_x(Length::Fill)
        .into()
}
