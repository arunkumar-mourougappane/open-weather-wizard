//! # About Screen
//!
//! Renders in its own OS window (opened by `Message::OpenAbout` in `src/app.rs`).
//! Static content pulled from `Cargo.toml` via `env!` macros, mirroring the
//! previous GTK `AboutDialog`.

use iced::widget::text::Alignment as TextAlignment;
use iced::widget::{button, column, container, image, row, scrollable, text};
use iced::{Alignment, Element, Font, Length, font};

use crate::app::Message;
use crate::ui::{icons, style};

const BOLD: Font = Font {
    weight: font::Weight::Bold,
    ..Font::DEFAULT
};

const HOMEPAGE: &str = env!("CARGO_PKG_HOMEPAGE");

/// The card's fixed content width -- keeps the description paragraph
/// wrapping at a readable line length instead of stretching edge-to-edge.
const CARD_WIDTH: f32 = 320.0;

/// A single `name <email>` entry from Cargo's `:`-separated `CARGO_PKG_AUTHORS`.
struct Author<'a> {
    name: &'a str,
    email: Option<&'a str>,
}

fn parse_author(entry: &str) -> Author<'_> {
    match entry.split_once('<') {
        Some((name, email)) => Author {
            name: name.trim(),
            email: Some(email.trim_end_matches('>').trim()),
        },
        None => Author {
            name: entry.trim(),
            email: None,
        },
    }
}

pub fn view<'a>() -> Element<'a, Message> {
    let authors: Vec<Author<'_>> = env!("CARGO_PKG_AUTHORS")
        .split(':')
        .map(parse_author)
        .collect();
    let license_url = format!("{HOMEPAGE}/blob/main/LICENSE");

    let mut card = column![].spacing(10).align_x(Alignment::Center);

    if let Some(handle) = icons::load_embedded_image("icon/icon.png") {
        card = card.push(image(handle).width(64).height(64));
    }

    card = card
        .push(
            text("Weather Wizard")
                .size(20)
                .font(BOLD)
                .style(style::accent),
        )
        .push(text(format!("v{}", env!("CARGO_PKG_VERSION"))).style(style::muted))
        .push(
            // Left-aligned rather than centered: a multi-line paragraph
            // centered creates ragged edges on every line that are harder
            // to scan, unlike the single short lines around it. The `\n`s
            // in CARGO_PKG_DESCRIPTION are there for crates.io/cargo's own
            // formatting, not meant as hard line breaks here -- collapse
            // them to spaces so the paragraph wraps naturally at CARD_WIDTH
            // instead of forcing one sentence per line.
            text(env!("CARGO_PKG_DESCRIPTION").replace('\n', " "))
                .size(12)
                .style(style::muted)
                .align_x(TextAlignment::Justified)
                .width(CARD_WIDTH),
        );

    let mut authors_column = column![text("Author").size(12).style(style::muted)]
        .spacing(4)
        .align_x(Alignment::Center);

    for author in &authors {
        authors_column = authors_column.push(text(author.name).size(14));
        if let Some(email) = author.email {
            authors_column = authors_column.push(
                button(text(email).size(12))
                    .on_press(Message::OpenUrl(format!("mailto:{email}")))
                    .style(style::link_button)
                    .padding(0),
            );
        }
    }

    card = card.push(authors_column).push(
        row![
            button(text("Homepage").size(13))
                .on_press(Message::OpenUrl(HOMEPAGE.to_string()))
                .style(style::link_button)
                .padding(0),
            text("\u{b7}").style(style::muted),
            button(text("MIT License").size(13))
                .on_press(Message::OpenUrl(license_url))
                .style(style::link_button)
                .padding(0),
        ]
        .spacing(10)
        .align_y(Alignment::Center),
    );

    let content = container(card).padding(20).style(style::panel);

    scrollable(
        container(content)
            .width(Length::Fill)
            .center_x(Length::Fill)
            .padding(24),
    )
    .height(Length::Fill)
    .into()
}
