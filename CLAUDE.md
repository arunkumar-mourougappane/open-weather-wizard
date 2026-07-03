# CLAUDE.md

Guidance for Claude Code when working in this repository.

## Project

`open-weather-wizard` — a desktop weather app written in Rust, built on
[iced](https://iced.rs) (Elm-architecture GUI, `iced::daemon` multi-window
pattern). Shows current conditions and a 5-day forecast via a pluggable
weather-provider abstraction (OpenWeatherMap live, Google Weather mock),
with GPU-rendered animated Lottie weather icons and a per-value cross-fade
on background refresh.

The app was originally built on GTK4 and rewritten onto iced — see
`docs/ARCHITECTURE.md` for why, and for the full state/message/rendering
design. `docs/UI_DESIGN.md` and `docs/ICON_MAPPING.md` cover visual design
and icon-to-condition mapping specifically.

## Build, test, lint

```sh
cargo build --locked
cargo test --locked --all
cargo clippy --all-targets --all-features -- -D warnings
cargo fmt -- --check          # cargo fmt to fix
```

These four are exactly what CI runs (`.github/workflows/ci.yml`) — run them
before considering a change done. On Linux, building needs
`libxkbcommon-dev libwayland-dev libx11-dev libxrandr-dev libxi-dev
libdbus-1-dev` (wgpu/iced windowing + keyring's Secret Service backend).

Examples (`examples/demo.rs`, `examples/openweather_test.rs`) are runnable
manual smoke tests, not part of `cargo test`; `openweather_test.rs` needs a
real `OPENWEATHER_API_KEY` env var. `examples/lottie_spike.rs` is a
throwaway prototype excluded from the published crate.

## Key architecture points

- **`src/app.rs`**: the `iced::daemon` — `AppState`/`Message`/`update()`/
  `view()`/`subscription()`. Preferences and About are separate OS windows
  (`Option<window::Id>`), not modals. `WeatherStatus`/`ForecastStatus` have
  a `Refreshing(T)` variant that keeps last-known-good data visible during
  background refresh instead of reverting to a loading state; use `.data()`
  to get at the inner value regardless of `Loaded` vs `Refreshing`.
- **`src/config.rs`**: `AppConfig` + `ConfigManager`. The API token is
  **never** stored in `config.json` — it lives in the OS's native secure
  credential store via the `keyring` crate (macOS Keychain / Windows
  Credential Manager / Linux Secret Service), accessed through a single
  process-cached `keyring::Entry` (`API_TOKEN_ENTRY`, a `LazyLock`). Old
  config files with a base64 `api_token_encoded` field are migrated
  automatically on load (see `migrate_legacy_token`).
- **`src/ui/transition.rs`**: `ValueTracker` drives per-value cross-fade
  animation on background-refreshed fields (300ms). `src/ui/skeleton.rs`
  drives the pulsing-opacity first-load placeholder. Both are pure
  functions of elapsed wall-clock time, redrawn on the existing
  `AnimationTick` (~33ms) subscription — same pattern as
  `src/ui/lottie/widget.rs`'s frame selection.
- **`src/ui/lottie/`**: GPU-shared Lottie rendering — `velato::Composition`
  frames rendered directly into iced's own `wgpu` render target (no CPU
  pixel readback). Only depend on `velato`'s re-exported `velato::vello`,
  never add `vello` as a separate top-level dependency — it pins a
  different `wgpu` major version than iced and the two `wgpu::Device` types
  won't unify. See `docs/ARCHITECTURE.md` for the two non-obvious bugs this
  surfaced (shared-texture-slot overwrite across icons, missing coordinate
  scale) and why it needed a spike (`examples/lottie_spike.rs`) before
  committing to the approach.
- **`src/weather_api/`**: `WeatherProvider` trait + `WeatherProviderFactory`.
  `forecast.rs::aggregate_daily()` buckets the 3-hour-interval 5-day
  forecast into daily cards; first/last day have partial interval coverage
  since the window starts from "now", not midnight.

## Testing keyring-backed code

Tests that touch `AppConfig::set_api_token`/`get_api_token` **must** swap in
the mock backend (`keyring::set_default_credential_builder
(keyring::mock::default_credential_builder())`) or they'll write to the
real OS keychain. Because the cached `API_TOKEN_ENTRY` is process-global and
`cargo test` runs tests in parallel threads within one process, token tests
serialize through a shared `Mutex` (`TOKEN_TEST_LOCK` / `lock_mock_keyring()`
in `src/lib.rs`) — follow that pattern for any new token-touching test
rather than swapping the backend ad hoc.

Also note: `keyring::mock`'s in-memory store is keyed to each `Entry`
*object*, not to the (service, username) pair — a fresh `Entry::new(...)`
call gets its own unrelated mock credential. Production code must reuse one
cached `Entry` for a set-then-get round trip to be observable at all, mock
or real.

## Conventions

- Commits are GPG-signed (see the `smart-commit`/`gpg-commit-sign` skills);
  don't skip signing or use `--no-verify`.
- Never hardcode a real API key in source or examples — the project has
  already had one leaked and scrubbed from a branch's history once (see git
  log around the `keyring` migration); read from an env var instead
  (`OPENWEATHER_API_KEY` in `examples/openweather_test.rs` is the pattern).
- `RELEASE_NOTES.md` at the repo root is the body of the GitHub Release
  created by `.github/workflows/release.yml` — update its `## Unreleased`
  section as part of any user-visible change, not just at tag time.
