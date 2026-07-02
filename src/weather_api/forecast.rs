//! # Forecast Data Model
//!
//! Types and pure aggregation logic for turning OpenWeatherMap's 5-day/3-hour
//! forecast response (`data/2.5/forecast`) into daily summaries the UI can render
//! as a horizontally-scrollable row of day cards.
//!
//! The free OpenWeatherMap tier doesn't include the newer One Call daily endpoint,
//! so this aggregates the 3-hourly entries into daily min/max/dominant-condition
//! buckets instead.

use serde::Deserialize;

use crate::weather_api::openweather_api::{Main, Weather, WeatherSymbol, get_weather_symbol};

/// A single 3-hourly entry from OpenWeatherMap's `data/2.5/forecast` `list` array.
#[derive(Deserialize, Debug)]
pub struct ForecastListItem {
    /// Unix timestamp (UTC) of this forecast entry.
    #[allow(dead_code)]
    pub dt: i64,
    pub main: Main,
    pub weather: Vec<Weather>,
    /// e.g. "2026-07-02 15:00:00" (UTC). Used for daily bucketing.
    pub dt_txt: String,
}

/// The `city` object in OpenWeatherMap's forecast response.
#[derive(Deserialize, Debug)]
pub struct ForecastCity {
    pub name: String,
}

/// The raw shape of OpenWeatherMap's `data/2.5/forecast` response.
#[derive(Deserialize, Debug)]
pub struct RawForecastResponse {
    pub list: Vec<ForecastListItem>,
    pub city: ForecastCity,
}

/// An app-level daily forecast summary, aggregated from several 3-hourly entries.
#[derive(Debug, Clone)]
pub struct ForecastDay {
    /// UTC calendar date, e.g. "2026-07-02". Kept as a `String` bucket key rather
    /// than adding a date-handling crate; this is a display label, not something
    /// the app performs date arithmetic on.
    pub date: String,
    pub temp_min: f64,
    pub temp_max: f64,
    pub description: String,
    pub symbol: WeatherSymbol,
}

/// An app-level forecast, ready for the UI to render.
#[derive(Debug, Clone)]
pub struct ForecastResponse {
    pub location_name: String,
    pub days: Vec<ForecastDay>,
}

/// Number of daily cards to show in the forecast row.
const MAX_FORECAST_DAYS: usize = 5;

/// Aggregates OpenWeatherMap's 3-hourly forecast entries into daily summaries.
///
/// Buckets by the UTC calendar date portion of `dt_txt` (the API returns UTC
/// timestamps; this accepts minor skew near local midnight rather than doing full
/// timezone arithmetic -- a known simplification for a first cut).
///
/// Within each day, `temp_min`/`temp_max` are the extremes across all entries, and
/// the "dominant" condition is taken from the entry closest to local noon
/// (12:00-15:00), falling back to the most frequent condition for the day if no
/// midday entry exists -- this avoids biasing the icon/description toward
/// whatever happened at 00:00/03:00.
pub fn aggregate_daily(raw: RawForecastResponse) -> ForecastResponse {
    use std::collections::BTreeMap;

    let mut by_date: BTreeMap<&str, Vec<&ForecastListItem>> = BTreeMap::new();
    for item in &raw.list {
        let date = item.dt_txt.split(' ').next().unwrap_or(&item.dt_txt);
        by_date.entry(date).or_default().push(item);
    }

    let mut days: Vec<ForecastDay> = by_date
        .into_iter()
        .filter_map(|(date, items)| {
            if items.is_empty() {
                return None;
            }

            let temp_min = items
                .iter()
                .map(|i| i.main.temp)
                .fold(f64::INFINITY, f64::min);
            let temp_max = items
                .iter()
                .map(|i| i.main.temp)
                .fold(f64::NEG_INFINITY, f64::max);

            let midday = items.iter().find(|item| {
                item.dt_txt
                    .split(' ')
                    .nth(1)
                    .map(|time| ("12:00:00"..="15:00:00").contains(&time))
                    .unwrap_or(false)
            });

            let representative = midday.copied().unwrap_or_else(|| {
                let mut counts: BTreeMap<&str, usize> = BTreeMap::new();
                for item in &items {
                    if let Some(weather) = item.weather.first() {
                        *counts.entry(weather.main.as_str()).or_insert(0) += 1;
                    }
                }
                let mode = counts
                    .into_iter()
                    .max_by_key(|(_, count)| *count)
                    .map(|(main, _)| main);
                items
                    .iter()
                    .find(|item| {
                        item.weather
                            .first()
                            .is_some_and(|w| Some(w.main.as_str()) == mode)
                    })
                    .copied()
                    .unwrap_or(items[0])
            });

            let weather = representative.weather.first();
            let dominant_condition = weather.map(|w| w.main.as_str()).unwrap_or("");
            let description = weather.map(|w| w.description.clone()).unwrap_or_default();

            Some(ForecastDay {
                date: date.to_string(),
                temp_min,
                temp_max,
                description,
                symbol: get_weather_symbol(dominant_condition),
            })
        })
        .collect();

    days.truncate(MAX_FORECAST_DAYS);

    ForecastResponse {
        location_name: raw.city.name.clone(),
        days,
    }
}
