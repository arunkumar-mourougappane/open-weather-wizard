//! # Weather Display Helpers
//!
//! The API is always fetched in Celsius/metric; the °C/°F preference only
//! affects how values are formatted for display, so toggling it never
//! triggers a re-fetch. Also home to a couple of small formatting helpers
//! (compass direction, local time-of-day) shared between `main_screen` and
//! `app::update`'s cross-fade field tracking.

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

/// Pressure is always fetched in hPa; converts to inHg for the imperial
/// preference (1 hPa = 0.02953 inHg).
pub fn pressure_to_display(hpa: i64, fahrenheit: bool) -> f64 {
    if fahrenheit {
        hpa as f64 * 0.02953
    } else {
        hpa as f64
    }
}

pub fn pressure_unit(fahrenheit: bool) -> &'static str {
    if fahrenheit { "inHg" } else { "hPa" }
}

/// Meteorological degrees (0 = due north, clockwise) to a 16-point compass
/// abbreviation.
pub fn compass_direction(deg: i64) -> &'static str {
    const DIRECTIONS: [&str; 16] = [
        "N", "NNE", "NE", "ENE", "E", "ESE", "SE", "SSE", "S", "SSW", "SW", "WSW", "W", "WNW",
        "NW", "NNW",
    ];
    let normalized = deg.rem_euclid(360) as f64;
    let index = ((normalized / 22.5) + 0.5) as usize % 16;
    DIRECTIONS[index]
}

/// Renders a Unix timestamp as a local 12-hour clock time using the API's
/// `timezone` offset (seconds from UTC) -- avoids pulling in a date/time
/// crate for what's ultimately just "HH:MM AM/PM".
pub fn format_local_time(unix_ts: i64, tz_offset_secs: i64) -> String {
    let local_secs = (unix_ts + tz_offset_secs).rem_euclid(86_400);
    let hours24 = local_secs / 3600;
    let minutes = (local_secs % 3600) / 60;
    let period = if hours24 < 12 { "AM" } else { "PM" };
    let hours12 = match hours24 % 12 {
        0 => 12,
        h => h,
    };
    format!("{hours12}:{minutes:02} {period}")
}
