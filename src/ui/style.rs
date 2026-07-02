//! # Shared Visual Style
//!
//! A small hand-picked palette and reusable container/button/text styles,
//! used across every screen instead of the raw `Theme::Light` defaults.
//! Centralized here so the look stays consistent without copy-pasting
//! `Color::from_rgb(...)` literals at each call site.

use iced::widget::{button, container, text};
use iced::{Background, Border, Color, Shadow, Theme, Vector};

/// Sky-blue accent, used for primary actions and the temperature/location
/// text -- the two things a glance at the app should land on first.
pub const ACCENT: Color = Color::from_rgb(0.11, 0.45, 0.85);
/// Slightly darker accent for hover/press states.
pub const ACCENT_STRONG: Color = Color::from_rgb(0.07, 0.35, 0.70);
/// Warm amber, reserved for error text (distinct from the accent so errors
/// don't read as "just another blue label").
pub const DANGER: Color = Color::from_rgb(0.82, 0.22, 0.18);

pub const CARD_BACKGROUND: Color = Color::from_rgb(0.96, 0.98, 1.0);
pub const CARD_BORDER: Color = Color::from_rgb(0.82, 0.89, 0.96);
pub const PANEL_BACKGROUND: Color = Color::WHITE;

pub const TEXT_PRIMARY: Color = Color::from_rgb(0.13, 0.15, 0.18);
pub const TEXT_MUTED: Color = Color::from_rgb(0.45, 0.49, 0.54);

/// The main content panel behind the current-conditions display: a soft
/// white card with a faint border and shadow, lifting it off the window
/// background instead of the text just floating on bare white.
pub fn panel(_theme: &Theme) -> container::Style {
    container::Style {
        background: Some(Background::Color(PANEL_BACKGROUND)),
        border: Border {
            color: CARD_BORDER,
            width: 1.0,
            radius: 16.0.into(),
        },
        shadow: Shadow {
            color: Color::from_rgba(0.0, 0.0, 0.0, 0.06),
            offset: Vector::new(0.0, 2.0),
            blur_radius: 12.0,
        },
        ..container::Style::default()
    }
}

/// A single forecast day card: a smaller, pale-blue-tinted card so the row
/// reads as a set of distinct chips rather than bare padding.
pub fn day_card(_theme: &Theme) -> container::Style {
    container::Style {
        background: Some(Background::Color(CARD_BACKGROUND)),
        border: Border {
            color: CARD_BORDER,
            width: 1.0,
            radius: 10.0.into(),
        },
        ..container::Style::default()
    }
}

/// Today's forecast card: same shape as `day_card`, but with an accent
/// border so it stands out at a glance from the other four days.
pub fn day_card_today(_theme: &Theme) -> container::Style {
    container::Style {
        background: Some(Background::Color(CARD_BACKGROUND)),
        border: Border {
            color: ACCENT,
            width: 2.0,
            radius: 10.0.into(),
        },
        ..container::Style::default()
    }
}

/// The filled accent button used for primary actions (Save, Preferences).
pub fn primary_button(_theme: &Theme, status: button::Status) -> button::Style {
    let background = match status {
        button::Status::Hovered | button::Status::Pressed => ACCENT_STRONG,
        button::Status::Active => ACCENT,
        button::Status::Disabled => Color::from_rgb(0.75, 0.78, 0.82),
    };

    button::Style {
        background: Some(Background::Color(background)),
        text_color: Color::WHITE,
        border: Border {
            radius: 8.0.into(),
            ..Border::default()
        },
        ..button::Style::default()
    }
}

/// The outlined/ghost button used for secondary actions (Cancel).
pub fn secondary_button(_theme: &Theme, status: button::Status) -> button::Style {
    let (border_color, text_color) = match status {
        button::Status::Hovered | button::Status::Pressed => (ACCENT_STRONG, ACCENT_STRONG),
        button::Status::Active => (CARD_BORDER, TEXT_PRIMARY),
        button::Status::Disabled => (CARD_BORDER, TEXT_MUTED),
    };

    button::Style {
        background: None,
        text_color,
        border: Border {
            color: border_color,
            width: 1.0,
            radius: 8.0.into(),
        },
        ..button::Style::default()
    }
}

pub fn muted(_theme: &Theme) -> text::Style {
    text::Style {
        color: Some(TEXT_MUTED),
    }
}

/// The inherited/default text color -- exists so call sites that only
/// sometimes want a color override (e.g. "accent this label if it's today")
/// can pick between two `Fn(&Theme) -> text::Style` values of the same type
/// instead of conditionally calling `.style()` at all.
pub fn default_text(_theme: &Theme) -> text::Style {
    text::Style { color: None }
}

pub fn danger(_theme: &Theme) -> text::Style {
    text::Style {
        color: Some(DANGER),
    }
}

pub fn accent(_theme: &Theme) -> text::Style {
    text::Style {
        color: Some(ACCENT),
    }
}
