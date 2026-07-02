//! # UI Module
//!
//! Screen views for the iced application. The application root (`AppState`,
//! `Message`, `update`, `view`, `subscription`) lives in `src/app.rs`; this module
//! only holds per-screen view functions and shared assets.

pub mod about;
pub mod forecast_row;
pub mod icons;
pub mod lottie;
pub mod main_screen;
pub mod preferences;
