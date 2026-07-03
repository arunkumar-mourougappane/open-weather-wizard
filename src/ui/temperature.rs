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

/// Wind speed is always fetched in meters/sec (`units=metric`); converts to
/// the more commonly displayed km/h or mph depending on the units preference.
pub fn speed_to_display(mps: f64, fahrenheit: bool) -> f64 {
    if fahrenheit {
        mps * 2.236_936
    } else {
        mps * 3.6
    }
}

pub fn speed_unit(fahrenheit: bool) -> &'static str {
    if fahrenheit { "mph" } else { "km/h" }
}

/// Visibility is always fetched in meters; converts to km or miles.
pub fn distance_to_display(meters: f64, fahrenheit: bool) -> f64 {
    if fahrenheit {
        meters / 1609.344
    } else {
        meters / 1000.0
    }
}

pub fn distance_unit(fahrenheit: bool) -> &'static str {
    if fahrenheit { "mi" } else { "km" }
}
