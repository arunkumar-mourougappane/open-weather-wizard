//! # Shared Visual Style
//!
//! A small hand-picked accent palette plus reusable container/button/text
//! styles, used across every screen instead of one-off inline colors.
//! Backgrounds/borders/muted text are derived from the active `Theme`'s
//! `extended_palette()` (not hardcoded) so the same styles work correctly
//! under both `Theme::Light` and `Theme::Dark` -- see `AppConfig::dark_mode`.

use iced::widget::{button, container, text};
use iced::{Background, Border, Color, Shadow, Theme, Vector};

/// The corner radius shared by every form field (`text_input`/`pick_list`)
/// and button, so inputs and actions read as one consistent, rounded style
/// instead of iced's default 2px -- almost-square -- corners.
const FIELD_RADIUS: f32 = 8.0;

/// Sky-blue accent, used for primary actions and the temperature/location
/// text -- the two things a glance at the app should land on first. Kept as
/// a single fixed brand color rather than theme-derived, since it reads
/// clearly against both a white and a dark panel background.
pub const ACCENT: Color = Color::from_rgb(0.11, 0.45, 0.85);
/// Slightly darker accent for hover/press states.
pub const ACCENT_STRONG: Color = Color::from_rgb(0.07, 0.35, 0.70);

/// A small fixed palette used to color-code the current-conditions detail
/// chips (humidity, wind, pressure, etc.) -- one glance at the badge color
/// distinguishes a stat from its neighbors without reading the label first.
pub const STAT_FEELS_LIKE: Color = ACCENT;
pub const STAT_HUMIDITY: Color = Color::from_rgb(0.20, 0.60, 0.72);
pub const STAT_WIND: Color = Color::from_rgb(0.27, 0.62, 0.38);
pub const STAT_PRESSURE: Color = Color::from_rgb(0.56, 0.38, 0.78);
pub const STAT_VISIBILITY: Color = Color::from_rgb(0.40, 0.46, 0.56);
pub const STAT_RANGE: Color = Color::from_rgb(0.87, 0.55, 0.18);
pub const STAT_SUNRISE: Color = Color::from_rgb(0.90, 0.62, 0.12);
pub const STAT_SUNSET: Color = Color::from_rgb(0.46, 0.34, 0.64);
pub const STAT_POP: Color = Color::from_rgb(0.24, 0.52, 0.80);

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

/// The thin footer ribbon naming the currently-active weather provider --
/// pinned to the bottom of the main window (see `main_screen::view`),
/// deliberately flatter/lower-contrast than `panel`/`day_card` since it's an
/// attribution strip, not another content card.
pub fn ribbon(theme: &Theme) -> container::Style {
    let palette = theme.extended_palette();

    container::Style {
        background: Some(Background::Color(palette.background.weak.color)),
        border: Border {
            color: palette.background.strong.color,
            width: 1.0,
            radius: 0.0.into(),
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

/// The forecast card currently expanded into the main panel's detail view:
/// a tinted accent wash (rather than `day_card_today`'s outline-only
/// treatment) so "this card is driving the panel above" reads as visually
/// distinct from "this card is today."
pub fn day_card_selected(_theme: &Theme) -> container::Style {
    container::Style {
        background: Some(Background::Color(Color { a: 0.14, ..ACCENT })),
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

/// An accent-outlined button for secondary actions that still need to stand
/// out against a `day_card` background -- unlike `secondary_button`'s
/// neutral, theme-derived border (which reads as barely-there against the
/// card's own near-identical gray in both light and dark mode), this uses
/// the fixed `ACCENT` color for both border and text, matching the section
/// headers so it stays legible regardless of theme.
pub fn accent_button(theme: &Theme, status: button::Status) -> button::Style {
    let palette = theme.extended_palette();

    let (border_color, text_color) = match status {
        button::Status::Hovered | button::Status::Pressed => (ACCENT_STRONG, ACCENT_STRONG),
        button::Status::Active => (ACCENT, ACCENT),
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
            width: 1.5,
            radius: 8.0.into(),
        },
        ..button::Style::default()
    }
}

/// A borderless, backgroundless button for inline "links" (e.g. the
/// homepage URL in the About window) -- just accent-colored text that
/// darkens slightly on hover, no button chrome.
pub fn link_button(_theme: &Theme, status: button::Status) -> button::Style {
    let text_color = match status {
        button::Status::Hovered | button::Status::Pressed => ACCENT_STRONG,
        _ => ACCENT,
    };

    button::Style {
        background: None,
        text_color,
        border: Border::default(),
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

/// Success text (e.g. a passed connection test) -- iced's `Theme` has no
/// built-in success palette to derive from (unlike `danger`), so this reuses
/// `STAT_WIND`'s green as a fixed color that reads as "success" without
/// clashing with the stat chips it's borrowed from.
pub fn success(_theme: &Theme) -> text::Style {
    text::Style {
        color: Some(STAT_WIND),
    }
}

pub fn accent(_theme: &Theme) -> text::Style {
    text::Style {
        color: Some(ACCENT),
    }
}

/// Rounded corners for every `text_input` (API Token, City, State/Province,
/// Country) -- otherwise identical to iced's own `text_input::default`.
/// Uses fully-qualified paths rather than importing `iced::widget::text_input`
/// since that name is reused for this function itself.
pub fn text_input(
    theme: &Theme,
    status: iced::widget::text_input::Status,
) -> iced::widget::text_input::Style {
    let default = iced::widget::text_input::default(theme, status);
    iced::widget::text_input::Style {
        border: Border {
            radius: FIELD_RADIUS.into(),
            ..default.border
        },
        ..default
    }
}

/// Rounded corners for the Provider `pick_list`, matching `text_input`'s so
/// every field in the form reads as the same rounded style.
pub fn pick_list(
    theme: &Theme,
    status: iced::widget::pick_list::Status,
) -> iced::widget::pick_list::Style {
    let default = iced::widget::pick_list::default(theme, status);
    iced::widget::pick_list::Style {
        border: Border {
            radius: FIELD_RADIUS.into(),
            ..default.border
        },
        ..default
    }
}

/// A small round, tinted badge behind a stat-chip's glyph -- a faint wash of
/// `color` rather than a solid fill, so it reads as an accent rather than
/// competing with the chip's own value text for attention.
pub fn stat_badge(color: Color) -> impl Fn(&Theme) -> container::Style {
    move |_theme: &Theme| container::Style {
        background: Some(Background::Color(Color { a: 0.16, ..color })),
        text_color: Some(color),
        border: Border {
            radius: 999.0.into(),
            ..Border::default()
        },
        ..container::Style::default()
    }
}
