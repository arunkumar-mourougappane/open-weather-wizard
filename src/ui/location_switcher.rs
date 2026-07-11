//! # Location Switcher
//!
//! A row of small pill buttons near the toolbar letting the user flip
//! between saved locations (issue #55) without opening Preferences.
//! Mirrors `forecast_row`'s index-based card-selection pattern, just with
//! plain buttons instead of full day cards -- there's no imagery/stats to
//! show per entry, only a name.

use iced::widget::{button, row, text};
use iced::{Alignment, Element};

use crate::app::{AppState, Message};
use crate::ui::style;

/// Renders the switcher, or `None` when there's only one saved location --
/// no point cluttering the toolbar with a single, un-clickable pill.
pub fn view(state: &AppState) -> Option<Element<'_, Message>> {
    if state.config.locations.len() <= 1 {
        return None;
    }

    let mut strip = row![].spacing(6);
    for (index, saved) in state.config.locations.iter().enumerate() {
        let is_current = index == state.config.current_location_index;
        let label = if saved.name.trim().is_empty() {
            "(unnamed)".to_string()
        } else {
            saved.name.clone()
        };
        strip = strip.push(
            button(text(label).size(12))
                .on_press(Message::LocationSwitched(index))
                .style(if is_current {
                    style::accent_button
                } else {
                    style::secondary_button
                }),
        );
    }

    Some(strip.align_y(Alignment::Center).into())
}
