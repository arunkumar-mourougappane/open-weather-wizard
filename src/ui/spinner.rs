//! # Loading Spinner
//!
//! A small rotating-arc indicator shown while a weather fetch is in flight.
//! Deliberately independent of the Lottie/vello pipeline (see `ui::lottie`):
//! a plain `canvas::Program` is simpler and sufficient for a one-shape
//! indicator, and doesn't need iced's `wgpu` device sharing at all.

use std::f32::consts::{PI, TAU};
use std::sync::LazyLock;
use std::time::Instant;

use iced::widget::canvas::{self, Path, Stroke};
use iced::{Element, Radians, Rectangle, Renderer, Theme, mouse};

use crate::ui::style;

/// Shared across every spinner instance so multiple on-screen spinners (were
/// there ever more than one) stay in sync, matching the animated weather
/// icons' `ANIMATION_START` convention in `ui::icons`.
static START: LazyLock<Instant> = LazyLock::new(Instant::now);

const ROTATION_PERIOD_SECS: f32 = 1.2;
const SWEEP: f32 = PI * 1.3;

pub fn spinner<'a, Message: 'a>(size: f32) -> Element<'a, Message> {
    canvas::Canvas::new(Spinner).width(size).height(size).into()
}

struct Spinner;

impl<Message> canvas::Program<Message, Theme, Renderer> for Spinner {
    type State = ();

    fn draw(
        &self,
        _state: &(),
        renderer: &Renderer,
        _theme: &Theme,
        bounds: Rectangle,
        _cursor: mouse::Cursor,
    ) -> Vec<canvas::Geometry> {
        let mut frame = canvas::Frame::new(renderer, bounds.size());
        let radius = (bounds.width.min(bounds.height) / 2.0) - 3.0;

        let progress = (START.elapsed().as_secs_f32() / ROTATION_PERIOD_SECS) % 1.0;
        let start_angle = Radians(progress * TAU);
        let end_angle = Radians(progress * TAU + SWEEP);

        let path = Path::new(|builder| {
            builder.arc(canvas::path::Arc {
                center: frame.center(),
                radius,
                start_angle,
                end_angle,
            });
        });

        frame.stroke(
            &path,
            Stroke {
                style: canvas::Style::Solid(style::ACCENT),
                width: 3.0,
                line_cap: canvas::LineCap::Round,
                ..Stroke::default()
            },
        );

        vec![frame.into_geometry()]
    }
}
