//! One-off asset generator for per-condition tray icons (issue #56 phase 3,
//! throwaway tool -- see its own doc comment, excluded from the published
//! crate like `lottie_spike.rs`/`tray_spike.rs`, see `Cargo.toml`).
//!
//! Rasterizes each `WeatherSymbol`'s existing `assets/static/*.svg` (the
//! same source `ui/icons.rs`'s `handle_for` already uses for the main
//! window) down to a small, tray-icon-sized PNG under `assets/tray/`,
//! rather than shipping `usvg`/`resvg`/`tiny-skia` as real runtime
//! dependencies just to rasterize SVGs on the fly -- the tray icon only
//! ever needs to change when the current condition changes, not on every
//! frame, so pre-rendering once and committing the result is simpler and
//! adds nothing to the shipped binary's dependency tree.
//!
//! `tiny_skia::Pixmap::save_png` handles the premultiplied-alpha ->
//! straight-alpha conversion PNG needs internally, so the output loads
//! through the exact same `image::load_from_memory(...).to_rgba8()`
//! pipeline `ui/icons.rs::load_tray_icon` already uses for every other
//! embedded icon.
//!
//! Run with: `cargo run --example generate_tray_icons`, then commit the
//! resulting `assets/tray/*.png` files. Only needs re-running if a
//! `WeatherSymbol` variant's source SVG (or the table below) changes.

use std::path::Path;

/// The distinct source SVGs `ui/icons.rs::asset_path` maps *some*
/// `WeatherSymbol` to (several symbols intentionally share one icon there,
/// e.g. `Mist`/`Smoke`/`Fog` all use `fog.svg` -- there's no visually
/// distinct "smoke" icon in the bundled set). Output files are named after
/// the *source* SVG, not any one symbol that happens to use it, so
/// `ui/icons.rs::tray_asset_path` can derive the tray PNG path straight
/// from `asset_path`'s own basename instead of needing a second hand-kept
/// symbol table in the runtime app -- only this generator needs its own
/// list, and only of the 12 SVGs actually referenced, not all 16 symbols.
const SOURCE_SVGS: &[&str] = &[
    "static/clear-day.svg",
    "static/cloudy-2-day.svg",
    "static/rainy-3.svg",
    "static/rainy-1.svg",
    "static/thunderstorms.svg",
    "static/snowy-2.svg",
    "static/fog.svg",
    "static/haze.svg",
    "static/dust.svg",
    "static/wind.svg",
    "static/tornado.svg",
    "static/cloudy.svg",
];

/// Target size in pixels -- large enough to stay crisp on a Retina menu
/// bar (2x a ~22pt native tray icon), small enough that these stay tiny
/// files.
const TRAY_ICON_SIZE: u32 = 64;

fn main() {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let assets_dir = Path::new(manifest_dir).join("assets");
    let out_dir = assets_dir.join("tray");
    std::fs::create_dir_all(&out_dir).expect("failed to create assets/tray/");

    let opt = usvg::Options::default();

    for svg_relative_path in SOURCE_SVGS {
        let svg_path = assets_dir.join(svg_relative_path);
        let svg_data =
            std::fs::read(&svg_path).unwrap_or_else(|e| panic!("failed to read {svg_path:?}: {e}"));

        let tree = usvg::Tree::from_data(&svg_data, &opt)
            .unwrap_or_else(|e| panic!("failed to parse {svg_path:?}: {e}"));

        // Uniform scale-to-fit (not stretch), then center the result in the
        // square canvas -- these source SVGs aren't square (mostly 56x48),
        // so filling both dimensions independently would distort them.
        let native_size = tree.size();
        let scale = (TRAY_ICON_SIZE as f32 / native_size.width())
            .min(TRAY_ICON_SIZE as f32 / native_size.height());
        let scaled_width = native_size.width() * scale;
        let scaled_height = native_size.height() * scale;
        let transform = tiny_skia::Transform::from_scale(scale, scale).post_translate(
            (TRAY_ICON_SIZE as f32 - scaled_width) / 2.0,
            (TRAY_ICON_SIZE as f32 - scaled_height) / 2.0,
        );

        let mut pixmap = tiny_skia::Pixmap::new(TRAY_ICON_SIZE, TRAY_ICON_SIZE)
            .expect("TRAY_ICON_SIZE must be non-zero");
        resvg::render(&tree, transform, &mut pixmap.as_mut());

        let basename = Path::new(svg_relative_path)
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or_else(|| panic!("unexpected SVG path shape: {svg_relative_path}"));
        let out_path = out_dir.join(format!("{basename}.png"));
        pixmap
            .save_png(&out_path)
            .unwrap_or_else(|e| panic!("failed to write {out_path:?}: {e}"));
        println!("wrote {}", out_path.display());
    }
}
