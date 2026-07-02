//! # Lottie Animation Support
//!
//! Renders `velato::Composition`s (parsed Lottie animations) as iced widgets,
//! sharing iced's own `wgpu` device for direct GPU compositing (see
//! `widget.rs` for the mechanics, and `examples/lottie_spike.rs` for the
//! throwaway prototype that first proved this viable).

mod widget;

pub use widget::lottie;

use std::time::Instant;

/// Computes the current (fractional) animation frame for `composition`,
/// looping continuously, given when its playback started.
pub fn frame_at(composition: &velato::Composition, start: Instant) -> f64 {
    let elapsed = start.elapsed().as_secs_f64();
    let duration = composition.frames.end - composition.frames.start;
    if duration <= 0.0 {
        return composition.frames.start;
    }
    composition.frames.start + (elapsed * composition.frame_rate) % duration
}
