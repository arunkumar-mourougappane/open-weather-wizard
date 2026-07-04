# Architecture

## Why iced, why velato

Open Weather Wizard was originally built on `gtk4-rs`. It was rewritten onto
[iced](https://github.com/iced-rs/iced) (a pure-Rust, Elm-architecture GUI
toolkit) for two concrete reasons, not aesthetic preference:

1. **Cross-platform CI regression.** GTK4's system-library/pkg-config
   requirements forced macOS and Windows out of the CI build matrix
   (`ci: Remove macOS and Windows from build matrix`). iced has no system
   dependencies beyond what `wgpu` needs to *build* (a few Linux windowing
   headers, covered below) — macOS/Windows support can come back once the
   Linux-only migration proves out.
2. **Animated icons never actually animated.** The bundled "animated" SVGs
   (`assets/animated/*.svg`) use CSS `@keyframes`, which `gdk-pixbuf`/`librsvg`
   silently ignore — they were always rendered as static frames. Every
   candidate pure-Rust toolkit (`iced`, `egui`, `Slint`) has the same
   limitation, since they all rasterize SVG via `resvg`, a static renderer.
   Real animation needed a dedicated animation runtime, not a smarter SVG
   renderer — hence `velato` (Lottie → `vello::Scene`), chosen over `rlottie`
   (C++ bindings, reintroduces a native build toolchain) and `rasterlottie`
   (CPU rasterizer, less maintained).

## Module map

| Old (GTK4) | New | Status |
|---|---|---|
| `src/ui/mod.rs` | `src/app.rs` | replaced |
| `src/ui/build_elements.rs` | `src/ui/icons.rs` | replaced |
| `src/ui/preferences.rs` | `src/ui/preferences.rs` | rewritten (iced `State`/`Message`/`view`) |
| `src/ui/about.rs` | `src/ui/about.rs` | rewritten (static iced `view`) |
| `src/style.css` | inline `.size()`/`.style()`/`Font` calls per view | replaced (see `UI_DESIGN.md`) |
| — | `src/ui/main_screen.rs` | new: current-conditions view fragment |
| — | `src/ui/forecast_row.rs` | new: scrollable day-card row |
| — | `src/ui/lottie/{mod,widget}.rs` | new: animated-icon widget |
| — | `src/weather_api/forecast.rs` | new: forecast types + `aggregate_daily()` |

Reused **unchanged**: `src/config.rs` (already framework-agnostic), and
`src/weather_api/{weather_provider,openweather_api,google_weather_api}.rs`
with only additive changes (`get_forecast` trait method).

## State / Message architecture

`src/app.rs` is an `iced::daemon` (not `iced::application`): a daemon opens no
window by default and doesn't exit when all windows close, which is what
multi-window apps need. Preferences and About are separate OS windows opened
via `window::open`/closed via `window::close`, tracked as
`Option<window::Id>` in `AppState` — chosen over an in-app overlay
(`iced_aw::Modal`) to match the previous GTK app's transient-window feel
without an extra dependency.

```
AppState {
    config, config_manager,          // reused from config.rs, owned directly
    weather: WeatherStatus,          // Loading | Loaded(ApiResponse) | Error(String)
    forecast: ForecastStatus,        // Loading | Loaded(ForecastResponse) | Error
    main_window: window::Id,
    prefs_window, prefs_state, about_window: Option<...>,
}
```

`AppConfig` is owned directly, not `Arc<Mutex<>>` as in the GTK version: GTK
needed shared mutable state across independently-registered signal-handler
closures; iced's Elm architecture already serializes every mutation through
one `update()` call on one owned `AppState`, so the mutex was solving a
problem that doesn't exist here.

### Message → window flow

```
                 ┌─────────────────┐
                 │   main window    │◄──────────────┐
                 └────────┬─────────┘                │
        OpenPreferences   │   OpenAbout               │ WindowCloseRequested
                 ▼         ▼                          │ (closes that window,
     ┌──────────────┐ ┌──────────┐                    │  or exits if it's
     │ prefs window  │ │  about   │                    │  the main window)
     └──────┬────────┘ └──────────┘                    │
             │ Save → apply_to(config), save_config,    │
             │        close window, RefreshRequested ───┘
             │ Cancel → discard, close window
             ▼
     (back to main window)
```

### Fetch lifecycle

`RefreshRequested` (manual, via Preferences Save) and `Tick` (the
auto-refresh subscription — 30s for OpenWeatherMap, 15 minutes for Google
Weather to stay within its free-tier call quota, see `GOOGLE_WEATHER_
REFRESH_INTERVAL` in `src/app.rs` and `docs/GOOGLE_WEATHER_API.md`) both set
`weather`/`forecast` to `Loading` and return `Task::batch([fetch_weather_task,
fetch_forecast_task])` (`Task::perform` wrapping the existing
`WeatherProviderFactory` + `WeatherProvider::get_weather`/`get_forecast`
calls). The two fetches resolve **independently** — `WeatherFetched`/
`ForecastFetched` each update their own status — so a forecast failure never
blanks out current conditions.

### Subscriptions

| Subscription | Interval | Purpose |
|---|---|---|
| `iced::time::every` | 30s | `Message::Tick` → auto-refresh weather + forecast |
| `iced::time::every` | ~33ms (30fps) | `Message::AnimationTick` → redraw animated icons (see below) |
| `window::close_requests()` | event-driven | `Message::WindowCloseRequested` |

## Animated icons: GPU-shared rendering

`src/ui/lottie/widget.rs` renders `velato::Composition` frames directly into
iced's own `wgpu` render target — no CPU pixel readback. This was the
plan's single largest open risk (there's no off-the-shelf iced↔vello
integration crate) and was resolved via a spike (`examples/lottie_spike.rs`)
before committing to the approach.

**The key discovery**: don't add `vello` as a separate top-level dependency.
Doing so pulls in a *second*, incompatible `wgpu` major version (iced pins
`wgpu 27`, a directly-added `vello 0.9` pins `wgpu 29` — `cargo tree -i wgpu`
reports these as ambiguous/separate crate instances, and Rust treats
`wgpu27::Device` and `wgpu29::Device` as unrelated types). Instead, depend on
`velato` alone and use its re-exported `velato::vello` — velato 0.10 pins
`vello 0.7`, whose `wgpu` requirement unifies with iced's into one shared
crate instance. That's what makes passing iced's own `&wgpu::Device` straight
into `vello::Renderer::new()` type-check at all.

**Rendering strategy**: `vello::Renderer::render_to_texture` needs
device+queue together, which only iced's `Primitive::prepare()` hook
provides — so each frame renders into an offscreen `Rgba8Unorm` storage
texture there (vello's internal compute pipeline requires that exact format,
regardless of the window's actual swapchain format, which was `Bgra8Unorm`
on the development machine). `Primitive::draw()` then composites that
texture into iced's actual target via a textured full-screen-triangle blit,
inside the render pass iced already provides scoped to the widget's bounds.

Two non-obvious bugs surfaced only by actually running the app (not visible
from code review or the single-icon spike) and are documented in
`src/ui/lottie/widget.rs`:

1. iced's renderer calls `prepare()` on **every** on-screen primitive before
   calling `draw()` on **any** of them. A single shared offscreen-texture
   slot (fine for the one-icon spike) gets overwritten by each icon in turn
   before any of them are actually drawn — every icon ends up blank or
   showing the wrong frame. Fixed by caching one texture per
   `(composition identity, pixel size)` key.
2. `velato::Renderer::append` does not scale the composition's own
   coordinate space (100×100 units) to the target texture's pixel size —
   without an explicit scale transform, a composition rendered smaller than
   its native size just shows a cropped corner, not a scaled-down icon.

**Coverage**: only the four most common conditions have a hand-authored
Lottie composition (`assets/lottie/{sun,clouds,rain,snow}.json` — see
`ICON_MAPPING.md`); everything else still renders as a static SVG via
`iced::widget::svg`. `src/ui/icons.rs::view()` dispatches between the two per
symbol.

## CI / build

- `Cargo.toml`: `gtk4`/`glib`/`gio`/`gdk-pixbuf` replaced with `iced` (features
  `svg`, `image`, `tokio`), `velato` (feature `wgpu`), and `wgpu` itself
  (needed directly by `src/ui/lottie/widget.rs`'s low-level primitive code).
  `reqwest`/`tokio`/`serde`/`rust-embed`/`base64`/`dirs`/`async-trait`/`anyhow`
  are unchanged.
- `.github/workflows/ci.yml`: GTK `apt-get` steps replaced with
  `libxkbcommon-dev libwayland-dev libx11-dev libxrandr-dev libxi-dev` (a
  safe superset of what `wgpu`/`winit` need to *build* on Linux — no GPU or
  display is needed to `cargo build`/`test`/`clippy`, since none of those
  open a window). The redundant legacy `.github/workflows/rust.yml` was
  deleted (duplicate of `ci.yml`'s jobs).
- **macOS/Windows**: not yet re-added to the build matrix. The GTK
  pkg-config friction that caused their removal doesn't apply to iced+wgpu
  (Metal/DX12/Vulkan support is native), so this is a safe, low-risk
  follow-up once the Linux-only migration has run green for a while.

## Testing

All pre-existing tests in `src/lib.rs` (`config` roundtrip/serialization,
`WeatherProviderFactory`, `Arc<Mutex<AppConfig>>` thread-safety) were kept
unchanged — none depended on GTK. Added:

- `test_forecast_aggregation` — the one genuinely new nontrivial business
  logic (`aggregate_daily`'s daily bucketing/midday-condition selection),
  exercised against a hand-built fixture spanning two days of mixed
  conditions.
- `test_lottie_assets_parse` — confirms the four hand-authored Lottie JSON
  files under `assets/lottie/` are valid, non-empty compositions; catches
  malformed JSON before it reaches the animated-icon widget.

`GoogleWeatherProvider` (`src/weather_api/google_weather_api.rs`) is a real
network-backed provider now, not a mock — its `#[cfg(test)]` module tests
the pure/deterministic pieces only (condition-type mapping, unit
conversions, response deserialization against hand-built JSON fixtures, and
RFC 3339/timezone resolution), the same fixture-based philosophy as
`test_forecast_aggregation`, since `cargo test` has neither network access
nor a real API key. `examples/google_weather_test.rs` is the live smoke test
for the actual HTTP integration, run manually (see `docs/GOOGLE_WEATHER_API.md`).

`view()` functions are not unit-tested (no established snapshot-testing
tooling in this codebase's dependency budget, and asserting on `Element`
tree shape is brittle for a solo-maintained app) — verified manually via
`cargo run` and screenshots instead.
