//! # Per-Value Cross-Fade
//!
//! Tracks the last-displayed string for arbitrary named fields (temperature,
//! humidity, a forecast card's hi/lo, ...) so that when fresh data changes a
//! value, the view can cross-fade the old text out and the new text in
//! instead of popping instantly. Bookkeeping (`note`) happens in
//! `app::update()` when new data lands; rendering (`cross_fade`) happens in
//! view code every frame, driven by the app's existing `AnimationTick`
//! subscription -- the fade progress is a pure function of elapsed time
//! since the change, the same shape as `icons::frame_at`.

use std::collections::HashMap;
use std::time::{Duration, Instant};

use iced::widget::{stack, text};
use iced::{Color, Element, Font, Pixels, Theme};

const CROSS_FADE_DURATION: Duration = Duration::from_millis(300);

struct Transition {
    outgoing: String,
    changed_at: Instant,
}

/// Owned by `AppState`; one shared map for both current-conditions and
/// forecast-card fields, keyed by short string constants like `"temp"` or
/// `format!("forecast_{index}_hilo")`.
#[derive(Default)]
pub struct ValueTracker {
    current: HashMap<String, String>,
    transitions: HashMap<String, Transition>,
}

impl ValueTracker {
    /// Records a freshly-fetched value for `key`. If it differs from what
    /// was last noted there, starts a new cross-fade transition from the
    /// old value. Call only from `app::update()` on fetch success -- never
    /// from view code, which must stay a pure read of state.
    pub fn note(&mut self, key: &str, new_value: &str) {
        let prior = self.current.insert(key.to_string(), new_value.to_string());
        if let Some(prior_value) = prior
            && prior_value != new_value
        {
            self.transitions.insert(
                key.to_string(),
                Transition {
                    outgoing: prior_value,
                    changed_at: Instant::now(),
                },
            );
        }
    }

    /// Renders `current` at `key`: plain text once any transition has
    /// finished (or none ever started), or a cross-fade -- the outgoing
    /// value fading out layered under the incoming value fading in -- while
    /// still within `CROSS_FADE_DURATION` of the last noted change.
    pub fn cross_fade<'a, Message: 'a>(
        &self,
        key: &str,
        current: String,
        size: impl Into<Pixels>,
        font: Font,
        style: impl Fn(&Theme) -> text::Style + Clone + 'a,
    ) -> Element<'a, Message> {
        let size = size.into();
        if let Some(transition) = self.transitions.get(key) {
            let progress =
                transition.changed_at.elapsed().as_secs_f32() / CROSS_FADE_DURATION.as_secs_f32();
            if progress < 1.0 {
                let outgoing_style = style.clone();
                let incoming_style = style;
                return stack([
                    text(transition.outgoing.clone())
                        .size(size)
                        .font(font)
                        .style(move |theme| faded(&outgoing_style, theme, 1.0 - progress))
                        .into(),
                    text(current)
                        .size(size)
                        .font(font)
                        .style(move |theme| faded(&incoming_style, theme, progress))
                        .into(),
                ])
                .into();
            }
        }

        text(current).size(size).font(font).style(style).into()
    }
}

/// Scales a style's resolved color's alpha by `factor`, falling back to the
/// theme's default text color when the style leaves color unset (e.g.
/// `style::default_text`) -- so cross-fading works the same regardless of
/// which style function a field normally renders with.
fn faded(style: &impl Fn(&Theme) -> text::Style, theme: &Theme, factor: f32) -> text::Style {
    let resolved = style(theme);
    let base = resolved
        .color
        .unwrap_or(theme.extended_palette().background.base.text);
    text::Style {
        color: Some(Color {
            a: base.a * factor,
            ..base
        }),
    }
}
