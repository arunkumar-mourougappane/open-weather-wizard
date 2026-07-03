//! # Temperature Display Helpers
//!
//! The API is always fetched in Celsius/metric; the °C/°F preference only
//! affects how values are formatted for display, so toggling it never
//! triggers a re-fetch.

pub fn celsius_to_display(celsius: f64, fahrenheit: bool) -> f64 {
    if fahrenheit {
        celsius * 9.0 / 5.0 + 32.0
    } else {
        celsius
    }
}

pub fn unit_symbol(fahrenheit: bool) -> &'static str {
    if fahrenheit { "\u{b0}F" } else { "\u{b0}C" }
}
