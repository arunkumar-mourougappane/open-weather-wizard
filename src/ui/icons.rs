//! # Weather Icon Assets
//!
//! Embeds the bundled SVG weather icons and exposes them as `iced::widget::svg::Handle`s
//! keyed by `WeatherSymbol`, eager-loaded once at startup so `view()` never touches the
//! embedded asset table on the hot render path.

use std::collections::HashMap;
use std::sync::LazyLock;

use iced::widget::svg;
use rust_embed::RustEmbed;

use crate::weather_api::openweather_api::WeatherSymbol;

/// Embeds the contents of the `assets/` directory into the application binary.
#[derive(RustEmbed)]
#[folder = "assets/"]
struct WeatherIconsAsset;

/// Maps a `WeatherSymbol` to its corresponding static SVG asset path.
///
/// Phase A renders icons statically (`assets/static/`); `assets/animated/` holds the
/// same set with CSS `@keyframes` reserved for the Phase C Lottie-authoring reference.
fn asset_path(symbol: WeatherSymbol) -> &'static str {
    match symbol {
        WeatherSymbol::Clear => "static/clear-day.svg",
        WeatherSymbol::Clouds => "static/cloudy-2-day.svg",
        WeatherSymbol::Rain => "static/rainy-3.svg",
        WeatherSymbol::Drizzle => "static/rainy-1.svg",
        WeatherSymbol::Thunderstorm => "static/thunderstorms.svg",
        WeatherSymbol::Snow => "static/snowy-2.svg",
        WeatherSymbol::Mist => "static/fog.svg",
        WeatherSymbol::Smoke => "static/fog.svg",
        WeatherSymbol::Haze => "static/haze.svg",
        WeatherSymbol::Dust => "static/dust.svg",
        WeatherSymbol::Fog => "static/fog.svg",
        WeatherSymbol::Sand => "static/dust.svg",
        WeatherSymbol::Ash => "static/dust.svg",
        WeatherSymbol::Squall => "static/wind.svg",
        WeatherSymbol::Tornado => "static/tornado.svg",
        WeatherSymbol::Default => "static/cloudy.svg",
    }
}

const ALL_SYMBOLS: [WeatherSymbol; 16] = [
    WeatherSymbol::Clear,
    WeatherSymbol::Clouds,
    WeatherSymbol::Rain,
    WeatherSymbol::Drizzle,
    WeatherSymbol::Thunderstorm,
    WeatherSymbol::Snow,
    WeatherSymbol::Mist,
    WeatherSymbol::Smoke,
    WeatherSymbol::Haze,
    WeatherSymbol::Dust,
    WeatherSymbol::Fog,
    WeatherSymbol::Sand,
    WeatherSymbol::Ash,
    WeatherSymbol::Squall,
    WeatherSymbol::Tornado,
    WeatherSymbol::Default,
];

static ICON_HANDLES: LazyLock<HashMap<&'static str, svg::Handle>> = LazyLock::new(|| {
    let mut handles = HashMap::new();
    for symbol in ALL_SYMBOLS {
        let path = asset_path(symbol);
        if let Some(embedded_file) = WeatherIconsAsset::get(path) {
            let handle = svg::Handle::from_memory(embedded_file.data.into_owned());
            handles.insert(path, handle);
        } else {
            log::warn!("Weather icon asset not found: {}", path);
        }
    }
    handles
});

/// Returns the cached `svg::Handle` for the given weather symbol.
///
/// Falls back to the `Default` symbol's icon if, for any reason, the requested
/// symbol's asset failed to embed (should not happen with the bundled asset set).
pub fn handle_for(symbol: WeatherSymbol) -> svg::Handle {
    let path = asset_path(symbol);
    ICON_HANDLES
        .get(path)
        .or_else(|| ICON_HANDLES.get(asset_path(WeatherSymbol::Default)))
        .cloned()
        .expect("default weather icon asset must be embedded")
}

/// Loads an embedded raster asset (e.g. `assets/icon/icon.png`) as an `iced::widget::image::Handle`.
pub fn load_embedded_image(asset_path: &str) -> Option<iced::widget::image::Handle> {
    WeatherIconsAsset::get(asset_path)
        .map(|file| iced::widget::image::Handle::from_bytes(file.data.into_owned()))
}
