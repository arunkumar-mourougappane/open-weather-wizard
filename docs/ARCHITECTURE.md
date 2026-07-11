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

## First-run setup and location detection

`ConfigManager::config_exists()` (checked in `app::boot()` *before*
`load_config()`) is the only signal distinguishing a fresh install from a
returning user — a missing or unparsable config file both silently fall back
to `AppConfig::default()`, so `load_config()` alone can't tell them apart.
When no config file exists yet, `boot()` opens the Preferences window
automatically alongside the main window, instead of firing a weather fetch
that's guaranteed to fail against an unset API token (every provider
requires one now — see `WeatherProviderFactory::create_provider`).
`AppState::is_first_run` swaps in the "Welcome to Weather Wizard" window
title and Preferences' welcome banner (`src/ui/preferences.rs`), clearing
back to ordinary "Preferences" copy the moment Save/Cancel/window-close
resolves it — a later manual reopen via the toolbar never shows first-run
copy again.

`src/geolocation.rs`'s `detect_location()` (wired to Preferences' "Detect my
location" button) is a two-tier best-effort lookup:

1. **OS-native positioning** — CoreLocation (macOS, via the low-level
   `objc2-core-location` bindings; no ergonomic wrapper crate exists), the
   WinRT `Geolocator` (Windows, via the `windows` crate), GeoClue2 (Linux,
   over D-Bus via `zbus`) — each gated behind a target-specific Cargo
   dependency (`[target.'cfg(target_os = "...")'.dependencies]`) so no
   platform pulls in another's bindings. Coordinates from any of these get
   reverse-geocoded into city/state/country via OpenStreetMap's free
   Nominatim API — deliberately country-independent (no US-specific
   state-abbreviation normalization, unlike Google Weather's own geocoding
   disambiguation in `google_weather_api.rs`), since this runs for any
   location on Earth.
2. **IP-based geolocation** (`ipwho.is`), used only when native location is
   unavailable, denied, or fails. Confirmed by an actual real-world test that
   IP geolocation alone is meaningfully inaccurate — several free providers
   all missed a real connection's city by tens of miles, an inherent
   limitation of resolving to wherever the ISP's routing infrastructure is
   registered rather than the physical location — so it's a last-resort
   fallback, never the primary path.

Every failure mode (unsupported platform, denied permission, no result, a
network error) degrades to the next tier rather than surfacing as an error:
detection is a convenience prefill the user can always override by typing,
never something that blocks Save or first-run setup. See
`src/geolocation.rs`'s own module doc for the full per-platform design
rationale, and `packaging/macos/Info.plist` /
`Cargo.toml`'s `package.metadata.packager.macos` for why the *packaged*
macOS release needs an `NSLocationWhenInUseUsageDescription` key to ever
show the location permission prompt (a plain `cargo run` dev binary never
will, regardless).

## Tray/menu bar icon

A persistent tray/menu bar icon (issue #56) sits alongside the existing
window-based app rather than replacing it -- this app never runs without its
main window existing, the icon is just an additional at-a-glance presence.

The obvious crate, [`tray-icon`](https://docs.rs/tray-icon) (used by Tauri),
raised the same conflict that drove the GTK4 → iced rewrite above: its Linux
backend needs `gtk3`/`libappindicator`, and its recommended integration
pattern pushes events through a `winit::event_loop::EventLoopProxy` -- which
`iced` doesn't expose, since it wraps `winit`'s event loop internally rather
than handing the app a reference to it.

The [`tray`](https://crates.io/crates/tray) crate (nobane/tray-rs) sidesteps
both problems:

- **No GTK on Linux.** Its system tray implementation is raw X11 (`x11rb`),
  not GTK -- consistent with this project's general "D-Bus/raw-protocol over
  GTK" preference (see `src/geolocation.rs`'s GeoClue2 D-Bus client). Caveat:
  this means a pure-Wayland session with no XWayland has no tray to draw
  into at all, since there's no GTK/StatusNotifierItem fallback either.
- **No event loop access needed.** Instead of a push-based `EventLoopProxy`
  integration, it exposes a polling API (`TrayIconEvent::receiver()`).
  `src/app.rs`'s `Message::AnimationTick` handler (already firing every
  `ANIMATION_TICK_INTERVAL`, 33ms, for the Lottie icons) drains it each tick
  instead of a dedicated timer.

`examples/tray_spike.rs` is the throwaway spike that proved this actually
works against this project's real `iced = "0.14"` (the crate's own bundled
example only declares `iced = "0.13"`) before adopting it for real in
`src/app.rs` (`build_tray_icon`, `sync_tray_display`) -- same
prove-it-before-committing approach as `examples/lottie_spike.rs` for the
GPU-shared Lottie rendering above. Verified manually on macOS only (icon
renders using the real embedded app icon, tooltip reflects live weather,
clicking it focuses the main window); Windows/Linux are unverified in this
environment -- the crate's own per-platform modules (`windows.rs` via
`Shell_NotifyIcon`, `linux.rs` via `x11rb`) have no `winit`/GTK dependency
either way, so the same integration should hold, but treat that as
unconfirmed until someone actually runs it there.

**Closing the main window hides it to the tray instead of quitting**, when a
tray icon exists -- the point of "a lightweight, persistent way to see
current conditions without the full main window open" (the issue's own
framing) is that closing the window is a normal thing to do, not the same as
quitting. `window::close` would destroy the window outright (no way to
reopen it under the same `window::Id`), so `WindowCloseRequested`'s handler
minimizes instead (`window::minimize(id, true)`) when `state.tray_icon.
is_some()`, falling back to actually quitting if the tray icon failed to
create -- with no tray, there'd be no way to ever get the window back
otherwise.

Left-clicking the tray icon brings the main window back via
`bring_window_to_front` (also used by the `OpenPreferences`/`OpenAbout`
handlers, to recover an already-open window that's hidden or minimized
rather than silently doing nothing). It queries `window::is_minimized(id)`
first and only issues one action: `window::minimize(id, false)` if actually
minimized, or `window::gain_focus(id)` otherwise -- `winit`'s
`focus_window()` on macOS checks `isMiniaturized()` and no-ops if true, and
`deminiaturize()` is animated/asynchronous, so batching both actions in the
same `Task::batch` races and silently fails almost every time; querying
first and issuing only the relevant one avoids the race entirely. The
`tray` crate has no context-menu API (only icon/tooltip/click events), so
**right-clicking the tray icon is the only remaining way to quit** once
closing the window no longer does -- what a menu's "Quit" item would have
done anyway, absent an actual menu to put it in.

A separate, unrelated fix bundled with this work: `winit::window::Window::
set_window_icon` (what `iced::window::Settings::icon` maps to) is documented
as unsupported on macOS entirely, so a bare `cargo run`/`cargo build` dev
binary (not packaged into a proper `.app` bundle) showed a generic
executable icon in the Dock. `icons::set_dock_icon_macos` bypasses `winit`
and sets it directly via `NSApplication.setApplicationIconImage` (`objc2`,
consistent with this project's other macOS-native code), called once early
in `boot()`. Packaged release builds are unaffected either way -- they
already got their Dock icon from `packaging/macos/Info.plist`'s `.icns`.

Also note that `window::Settings::exit_on_close_request` defaults to
`true`, which makes `iced_winit` destroy a window the instant
`WindowEvent::CloseRequested` fires, bypassing `WindowCloseRequested`'s
handler entirely regardless of what it decides to do. The main window's
`Settings` in `boot()` sets it to `false` so the app can actually intercept
the close and choose to minimize instead.

### Follow-up polish (icon variants, title text, alert badge)

Four incremental improvements landed after the initial feature, each its
own commit:

1. **Template icon.** `build_tray_icon` sets `.with_icon_as_template(true)`
   so macOS renders the icon as a monochrome template image that adapts to
   light/dark menu bars, matching the system's own status icons. Later,
   testing phase 3 (below) surfaced that `TrayIcon::set_icon()` (the
   `tray` crate's macOS implementation) hardcodes the template flag to
   `false` internally, silently discarding this setting the first time the
   icon image changes -- fixed by calling `set_icon_with_as_template(icon,
   true)` instead wherever the icon is updated after creation. That method
   is a documented no-op on non-macOS platforms, so it's only used there,
   gated `#[cfg(target_os = "macos")]`/`#[cfg(not(target_os = "macos"))]`.
2. **Compact title text.** `tray_title_text` renders the current
   temperature ("68°F") via `TrayIcon::set_title` (macOS-only), so the
   number is visible without hovering for the tooltip. `None` while
   loading or after a fetch error, so there's no blank/stale text next to
   the icon when there's nothing useful to say.
3. **Per-condition icon.** `tray_icon_symbol` maps the current
   `WeatherStatus` to a `WeatherSymbol`, and `icons::tray_icon_for` loads a
   pre-rendered 64x64 PNG for it from `assets/tray/`. Those PNGs are
   generated offline by `examples/generate_tray_icons.rs` (a dev-only
   tool, excluded from the published crate like the other `examples/*_
   spike.rs` files) by rasterizing the same `assets/static/*.svg` files
   the main window already uses via `usvg`/`resvg`/`tiny-skia` -- kept as
   dev-dependencies only, so the shipped binary never links an SVG
   rasterizer just to redraw a tray icon that only changes when the
   weather condition does. `icons::tray_asset_path` derives each PNG's
   path directly from the existing SVG mapping (stripping `static/`/`.svg`
   for `tray/`/`.png`) instead of a second hand-kept symbol table, so the
   two can't silently drift apart.
4. **Severe alert badge.** `has_severe_alert` checks `state.alerts` for the
   same `Severe`/`Extreme` threshold `ui/main_screen.rs::alerts_view`
   already uses for its own danger styling. When true, `tray_tooltip_text`
   and `tray_title_text` both gain a `⚠` prefix, so an active severe alert
   is visible without opening the app -- including while the title would
   otherwise show nothing (loading/error), since that's exactly when a
   user might have no other reason to notice. `sync_tray_display` is
   called from the `AlertsFetched(Ok(_))` handler too (not just weather
   fetches), so the badge appears as soon as alerts arrive rather than
   waiting for the next weather refresh.

All four are pure, unit-tested functions (`tray_tooltip_text`,
`tray_title_text`, `tray_icon_symbol`, `has_severe_alert`) separate from
`sync_tray_display`'s actual OS-level side effects, consistent with this
project's general pattern of keeping side-effecting glue thin and
everything else independently testable.

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
