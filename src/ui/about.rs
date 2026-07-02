//! # About Screen
//!
//! Renders in its own OS window (opened by `Message::OpenAbout` in `src/app.rs`).
//! Static content pulled from `Cargo.toml` via `env!` macros, mirroring the
//! previous GTK `AboutDialog`.

use iced::widget::{column, container, image, text};
use iced::{Alignment, Element, Length};

use crate::app::Message;
use crate::ui::icons;

pub fn view<'a>() -> Element<'a, Message> {
    let authors_env = env!("CARGO_PKG_AUTHORS");
    let authors: Vec<&str> = authors_env.split(':').collect();

    let mut content = column![].spacing(8).align_x(Alignment::Center);

    if let Some(handle) = icons::load_embedded_image("icon/icon.png") {
        content = content.push(image(handle).width(64).height(64));
    }

    content = content
        .push(text("Weather Wizard").size(20))
        .push(text(format!("v{}", env!("CARGO_PKG_VERSION"))))
        .push(text(authors.join(", ")))
        .push(text("MIT License"))
        .push(text(env!("CARGO_PKG_HOMEPAGE")).size(12));

    container(content)
        .width(Length::Fill)
        .center_x(Length::Fill)
        .padding(20)
        .into()
}
