//! # Shared Visual Style
//!
//! A small hand-picked accent palette plus reusable container/button/text
//! styles, used across every screen instead of one-off inline colors.
//! Backgrounds/borders/muted text are derived from the active `Theme`'s
//! `extended_palette()` (not hardcoded) so the same styles work correctly
//! under both `Theme::Light` and `Theme::Dark` -- see `AppConfig::dark_mode`.

use iced::widget::{button, container, text};
use iced::{Background, Border, Color, Shadow, Theme, Vector};

/// Sky-blue accent, used for primary actions and the temperature/location
/// text -- the two things a glance at the app should land on first. Kept as
/// a single fixed brand color rather than theme-derived, since it reads
/// clearly against both a white and a dark panel background.
pub const ACCENT: Color = Color::from_rgb(0.11, 0.45, 0.85);
/// Slightly darker accent for hover/press states.
pub const ACCENT_STRONG: Color = Color::from_rgb(0.07, 0.35, 0.70);

/// The main content panel behind the current-conditions display: a card in
/// the theme's base background/border colors with a faint shadow, lifting
/// it off the window background rather than the text floating on bare
/// window color.
pub fn panel(theme: &Theme) -> container::Style {
    let palette = theme.extended_palette();

    container::Style {
        background: Some(Background::Color(palette.background.base.color)),
        border: Border {
            color: palette.background.strong.color,
            width: 1.0,
            radius: 16.0.into(),
        },
        shadow: Shadow {
            color: Color::from_rgba(0.0, 0.0, 0.0, 0.08),
            offset: Vector::new(0.0, 2.0),
            blur_radius: 12.0,
        },
        ..container::Style::default()
    }
}

/// A single forecast day card: a subtly-tinted card so the row reads as a
/// set of distinct chips rather than bare padding.
pub fn day_card(theme: &Theme) -> container::Style {
    let palette = theme.extended_palette();

    container::Style {
        background: Some(Background::Color(palette.background.weak.color)),
        border: Border {
            color: palette.background.strong.color,
            width: 1.0,
            radius: 10.0.into(),
        },
        ..container::Style::default()
    }
}

/// Today's forecast card: same shape as `day_card`, but with an accent
/// border so it stands out at a glance from the other four days.
pub fn day_card_today(theme: &Theme) -> container::Style {
    let palette = theme.extended_palette();

    container::Style {
        background: Some(Background::Color(palette.background.weak.color)),
        border: Border {
            color: ACCENT,
            width: 2.0,
            radius: 10.0.into(),
        },
        ..container::Style::default()
    }
}

/// The filled accent button used for primary actions (Save, Preferences).
pub fn primary_button(theme: &Theme, status: button::Status) -> button::Style {
    let palette = theme.extended_palette();

    let background = match status {
        button::Status::Hovered | button::Status::Pressed => ACCENT_STRONG,
        button::Status::Active => ACCENT,
        button::Status::Disabled => palette.background.strong.color,
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
pub fn secondary_button(theme: &Theme, status: button::Status) -> button::Style {
    let palette = theme.extended_palette();

    let (border_color, text_color) = match status {
        button::Status::Hovered | button::Status::Pressed => (ACCENT_STRONG, ACCENT_STRONG),
        button::Status::Active => (
            palette.background.strong.color,
            palette.background.base.text,
        ),
        button::Status::Disabled => (
            palette.background.weak.color,
            palette.background.strong.color,
        ),
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

/// A dimmer version of the theme's own text color, for secondary/supporting
/// text (descriptions, timestamps, hints).
pub fn muted(theme: &Theme) -> text::Style {
    let palette = theme.extended_palette();
    let base = palette.background.base.text;

    text::Style {
        color: Some(Color { a: 0.6, ..base }),
    }
}

/// The inherited/default text color -- exists so call sites that only
/// sometimes want a color override (e.g. "accent this label if it's today")
/// can pick between two `Fn(&Theme) -> text::Style` values of the same type
/// instead of conditionally calling `.style()` at all.
pub fn default_text(_theme: &Theme) -> text::Style {
    text::Style { color: None }
}

/// Error/danger text, using the theme's own danger palette so it stays
/// legible (and stays "red", not just "less blue") in dark mode too.
pub fn danger(theme: &Theme) -> text::Style {
    text::Style {
        color: Some(theme.extended_palette().danger.base.color),
    }
}

pub fn accent(_theme: &Theme) -> text::Style {
    text::Style {
        color: Some(ACCENT),
    }
}
