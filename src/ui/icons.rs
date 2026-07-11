//! # Weather Icon Assets
//!
//! Embeds the bundled SVG weather icons and exposes them as `iced::widget::svg::Handle`s
//! keyed by `WeatherSymbol`, eager-loaded once at startup so `view()` never touches the
//! embedded asset table on the hot render path.
//!
//! Every condition also has a hand-authored Lottie animation (`assets/lottie/*.json`,
//! shapes/motion adapted from the upstream `animated/` icon set's CSS keyframes --
//! see `docs/ICON_MAPPING.md`); `view()` dispatches to the animated `lottie` widget
//! and only falls back to the static SVG if a composition fails to load.

use std::collections::HashMap;
use std::sync::{Arc, LazyLock};
use std::time::Instant;

use iced::Element;
use iced::widget::svg;
use rust_embed::RustEmbed;

use crate::ui::lottie;
use crate::weather_api::openweather_api::WeatherSymbol;

/// Embeds the contents of the `assets/` directory into the application binary.
#[derive(RustEmbed)]
#[folder = "assets/"]
struct WeatherIconsAsset;

/// Maps a `WeatherSymbol` to its corresponding static SVG asset path
/// (`assets/static/`), used as the fallback if a symbol's Lottie
/// composition fails to load.
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

/// Loads an embedded PNG asset (e.g. `assets/icon/icon.png`) as an
/// `iced::window::Icon`, for `window::Settings::icon` (Dock/taskbar icon).
pub fn load_window_icon(asset_path: &str) -> Option<iced::window::Icon> {
    let file = WeatherIconsAsset::get(asset_path)?;
    let rgba = image::load_from_memory(file.data.as_ref()).ok()?.to_rgba8();
    let (width, height) = rgba.dimensions();
    iced::window::icon::from_rgba(rgba.into_raw(), width, height).ok()
}

/// Loads an embedded PNG asset as a `tray::Icon`, for the persistent tray/
/// menu bar icon (issue #56). Same embedded-asset source as
/// `load_window_icon` -- a packaged/installed binary has no `assets/`
/// directory alongside it to read from a filesystem path at runtime.
pub fn load_tray_icon(asset_path: &str) -> Option<tray::Icon> {
    let file = WeatherIconsAsset::get(asset_path)?;
    let rgba = image::load_from_memory(file.data.as_ref()).ok()?.to_rgba8();
    let (width, height) = rgba.dimensions();
    tray::Icon::from_rgba(rgba.into_raw(), width, height).ok()
}

/// Maps a `WeatherSymbol` to its pre-rendered tray icon under
/// `assets/tray/` (issue #56 phase 3) -- derived from `asset_path`'s own
/// mapping (stripping `static/`/`.svg` for `tray/`/`.png`) rather than a
/// second hand-kept table, so the two can never silently drift apart:
/// several symbols intentionally share one source SVG (see `asset_path`'s
/// docs), and this reuses exactly the same grouping. The actual PNGs are
/// generated once by `examples/generate_tray_icons.rs`, not rasterized at
/// runtime -- see that file's docs for why.
fn tray_asset_path(symbol: WeatherSymbol) -> String {
    let svg_path = asset_path(symbol);
    let basename = svg_path
        .strip_prefix("static/")
        .and_then(|s| s.strip_suffix(".svg"))
        .expect("asset_path always returns \"static/<name>.svg\"");
    format!("tray/{basename}.png")
}

/// Loads the tray icon variant for the given `WeatherSymbol` -- `None` if
/// the asset is somehow missing or fails to decode, in which case callers
/// should just leave the tray icon showing whatever it already had rather
/// than clearing it to nothing.
pub fn tray_icon_for(symbol: WeatherSymbol) -> Option<tray::Icon> {
    load_tray_icon(&tray_asset_path(symbol))
}

/// Sets the Dock icon directly via AppKit, bypassing `iced`/`winit` --
/// `winit::window::Window::set_window_icon` (what `iced::window::Settings::
/// icon` maps to, see `load_window_icon`) is a documented no-op on macOS,
/// so a bare `cargo run` dev binary (not packaged into a proper `.app`
/// bundle with an `Info.plist`/`.icns`) would otherwise show a generic
/// executable icon in the Dock instead of this app's own. Call once, early
/// in `boot()`; a no-op (with a warning logged) if the asset is missing,
/// the image fails to decode, or this somehow isn't running on the main
/// thread -- a wrong/missing Dock icon is cosmetic, never worth a crash.
#[cfg(target_os = "macos")]
pub fn set_dock_icon_macos(asset_path: &str) {
    use objc2::AnyThread;
    use objc2_app_kit::{NSApplication, NSImage};
    use objc2_foundation::{MainThreadMarker, NSData};

    let Some(file) = WeatherIconsAsset::get(asset_path) else {
        log::warn!("Dock icon asset not found: {asset_path}");
        return;
    };
    let Some(mtm) = MainThreadMarker::new() else {
        log::warn!("Could not set Dock icon: not running on the main thread");
        return;
    };

    let data = NSData::with_bytes(file.data.as_ref());
    let image = NSImage::initWithData(NSImage::alloc(), &data);
    let Some(image) = image else {
        log::warn!("Failed to decode Dock icon image from {asset_path}");
        return;
    };

    let app = NSApplication::sharedApplication(mtm);
    unsafe { app.setApplicationIconImage(Some(&image)) };
}

/// Maps every `WeatherSymbol` to its hand-authored `assets/lottie/*.json`
/// animation. Conditions that don't have a visually distinct animation of
/// their own share the closest match (e.g. Mist/Smoke/Fog all drift the same
/// fog bands; Dust/Sand/Ash share the tinted haze bands) -- `view()` falls
/// back to the static SVG only if a composition fails to load.
fn lottie_asset_path(symbol: WeatherSymbol) -> Option<&'static str> {
    match symbol {
        WeatherSymbol::Clear => Some("lottie/sun.json"),
        WeatherSymbol::Clouds => Some("lottie/clouds.json"),
        WeatherSymbol::Rain => Some("lottie/rain.json"),
        WeatherSymbol::Drizzle => Some("lottie/drizzle.json"),
        WeatherSymbol::Thunderstorm => Some("lottie/thunderstorm.json"),
        WeatherSymbol::Snow => Some("lottie/snow.json"),
        WeatherSymbol::Mist => Some("lottie/fog.json"),
        WeatherSymbol::Smoke => Some("lottie/fog.json"),
        WeatherSymbol::Haze => Some("lottie/haze.json"),
        WeatherSymbol::Dust => Some("lottie/haze.json"),
        WeatherSymbol::Fog => Some("lottie/fog.json"),
        WeatherSymbol::Sand => Some("lottie/haze.json"),
        WeatherSymbol::Ash => Some("lottie/haze.json"),
        WeatherSymbol::Squall => Some("lottie/wind.json"),
        WeatherSymbol::Tornado => Some("lottie/tornado.json"),
        WeatherSymbol::Default => Some("lottie/clouds.json"),
    }
}

static ANIMATED_COMPOSITIONS: LazyLock<HashMap<&'static str, Arc<velato::Composition>>> =
    LazyLock::new(|| {
        let mut compositions = HashMap::new();
        for symbol in ALL_SYMBOLS {
            let Some(path) = lottie_asset_path(symbol) else {
                continue;
            };
            let Some(embedded_file) = WeatherIconsAsset::get(path) else {
                log::warn!("Lottie asset not found: {}", path);
                continue;
            };
            match velato::Composition::from_slice(embedded_file.data.as_ref()) {
                Ok(composition) => {
                    compositions.insert(path, Arc::new(composition));
                }
                Err(e) => log::warn!("Failed to parse Lottie asset {}: {:?}", path, e),
            }
        }
        compositions
    });

/// The instant animation playback is measured from, shared by every animated
/// icon so their frame timing stays consistent with each other; there's no
/// per-icon "start" since these are continuous ambient loops, not one-shots.
static ANIMATION_START: LazyLock<Instant> = LazyLock::new(Instant::now);

/// Renders the given weather symbol at `size` logical pixels square: the
/// animated Lottie widget for every symbol, falling back to the static SVG
/// only if its composition somehow failed to load at startup.
pub fn view<'a, Message: 'a>(symbol: WeatherSymbol, size: f32) -> Element<'a, Message> {
    if let Some(path) = lottie_asset_path(symbol)
        && let Some(composition) = ANIMATED_COMPOSITIONS.get(path)
    {
        let frame = lottie::frame_at(composition, *ANIMATION_START);
        return lottie::lottie(composition.clone(), frame, size);
    }

    svg(handle_for(symbol)).width(size).height(size).into()
}
