# <img src="assets/icon/icon.png" alt="" width="40" height="40"> Open Weather Wizard

[![CI](https://github.com/arunkumar-mourougappane/open-weather-wizard/actions/workflows/ci.yml/badge.svg)](https://github.com/arunkumar-mourougappane/open-weather-wizard/actions/workflows/ci.yml)
[![Crates.io](https://img.shields.io/crates/v/open-weather-wizard.svg)](https://crates.io/crates/open-weather-wizard)
[![License: MIT](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)

A simple, elegant desktop weather app built in Rust with [iced](https://github.com/iced-rs/iced). Current conditions, a 5-day forecast you can tap into for detail, and every weather condition rendered as its own hand-authored, GPU-composited [Lottie](https://lottiefiles.com/) animation.

<p align="center">
  <img src="docs/screenshots/main-view.png" alt="Open Weather Wizard main window showing current conditions and the 5-day forecast carousel" width="600">
</p>

## Features

- **Current conditions at a glance** — icon, temperature, and a color-coded stat grid (feels-like, humidity, wind, pressure, visibility, sunrise/sunset).
- **5-day forecast carousel** — centered when it fits, an invisible-scroll carousel when it doesn't. Tap any day to see its full detail (hi/lo, feels-like, humidity, wind, pressure, visibility, chance of rain) right in the main card, no popup or extra window.
- **Animated weather icons for every condition** — sun, rain, snow, clouds, thunderstorms, drizzle, fog, haze, wind, tornado, and more, each a small Lottie composition rendered through [`velato`](https://github.com/linebender/velato) straight onto iced's own `wgpu` surface.
- **Silent, non-blocking refresh** — data updates automatically every 30 seconds (or on demand) without ever blanking the screen back to a spinner; changed values cross-fade in place. A shimmer skeleton placeholder is shown only for the very first load.
- **Dark mode and °C/°F**, both live-previewed in Preferences before you save.
- **Two weather providers** — live data from [OpenWeatherMap](https://openweathermap.org/) or [Google Maps Platform's Weather API](https://mapsplatform.google.com/maps-products/weather/), both free-tier, both requiring your own API key.
- **Guided first-run setup** — on first launch, Preferences opens automatically with a welcome banner walking you through picking a provider, adding its API key, and setting your Home location (typed in, or detected automatically — see below).
- **Location detection** — "Detect my location" in Preferences tries your OS's native location service first (macOS/Windows/Linux) for real GPS/Wi-Fi-based accuracy, falling back to an IP-based lookup only if that's unavailable or denied.
- **Headless/CLI mode** — `--headless` fetches and prints the weather (optionally as JSON) without opening the GUI, for scripting or status-bar widgets.
- **Cross-platform** — Linux, macOS, and Windows, with no system GUI toolkit dependency.

## Technology Stack

- [**Rust**](https://www.rust-lang.org/) — application logic.
- [**iced**](https://github.com/iced-rs/iced) — the GUI, in the Elm-architecture style (state / message / update / view).
- [**velato**](https://github.com/linebender/velato) + [**vello**](https://github.com/linebender/vello) — Lottie-to-GPU animation, sharing iced's own `wgpu` device for direct compositing.
- [**tokio**](https://tokio.rs/) — async weather/forecast fetches.
- [**reqwest**](https://github.com/seanmonstar/reqwest) — HTTP client for the weather APIs.
- [**serde**](https://serde.rs/) — configuration and API response (de)serialization.
- [**rust-embed**](https://github.com/pyrossh/rust-embed) — bundles icons into the binary for offline startup.

See [`docs/ARCHITECTURE.md`](docs/ARCHITECTURE.md) for the module map and design rationale, and [`docs/ICON_MAPPING.md`](docs/ICON_MAPPING.md) for how a weather condition becomes an animated icon.

## Installation

### Prebuilt app packages (recommended)

Each [GitHub Release](https://github.com/arunkumar-mourougappane/open-weather-wizard/releases) includes installable packages alongside raw binaries:

- **macOS** — `.dmg` (drag `Open Weather Wizard.app` into `/Applications`)
- **Linux** — `.deb` (`sudo apt install ./open-weather-wizard_*.deb` or `sudo dpkg -i`)
- **Windows** — `.exe` NSIS installer (Start Menu shortcut, uninstaller)

### From crates.io

```bash
cargo install open-weather-wizard
```

### Quick Install (Linux, from source)

Clones the repo, builds and installs the binary, and sets up desktop integration (application menu entry, icons):

```bash
git clone https://github.com/arunkumar-mourougappane/open-weather-wizard.git
cd open-weather-wizard
./install.sh
```

Uninstall with `open-weather-wizard-uninstall` (installed alongside the binary), or manually:

```bash
cargo uninstall open-weather-wizard
rm -f ~/.local/share/applications/open-weather-wizard.desktop
rm -f ~/.local/share/icons/hicolor/*/apps/open-weather-wizard.*
```

### From Source

```bash
git clone https://github.com/arunkumar-mourougappane/open-weather-wizard.git
cd open-weather-wizard
cargo run          # run without installing
cargo install --path .   # or install the binary
```

#### Linux build dependencies

iced's `wgpu`/`winit` backend needs a few windowing headers to *build* (no GTK or other desktop toolkit required):

```bash
# Debian/Ubuntu
sudo apt-get install libxkbcommon-dev libwayland-dev libx11-dev libxrandr-dev libxi-dev
```

macOS and Windows builds have no extra system dependencies beyond a working Rust toolchain.

## Configuration

On first run (no config file yet), Preferences opens automatically alongside the main window with a short welcome banner, since there's nothing to fetch weather for until a provider, API key, and Home location are set:

<p align="center">
  <img src="docs/screenshots/first-run-setup.png" alt="Preferences window on first run, showing the welcome banner, provider/API key fields, and Home location section with a Detect my location button" width="520">
</p>

Everything shown there can be changed again at any time from the toolbar's **Preferences** button (weather provider, API token, Home location, dark mode, units) — changes are previewed live before you save.

The config file lives at (via the [`dirs`](https://github.com/dirs-dev/dirs-rs) crate's platform conventions):

| Platform | Path |
|---|---|
| Linux | `~/.config/open-weather-wizard/config.json` |
| macOS | `~/Library/Application Support/open-weather-wizard/config.json` |
| Windows | `%APPDATA%\open-weather-wizard\config.json` |

Delete the file to reset to defaults.

### API Keys

Both providers need your own API key, entered in Preferences (never stored in the config file — see below):

- **OpenWeatherMap**: sign up for a free API key at [openweathermap.org](https://openweathermap.org/api).
- **Google Weather**: enable the Weather API on a Google Cloud project and create an API key — see [Google's Weather API documentation](https://developers.google.com/maps/documentation/weather/overview) and this repo's own [`docs/GOOGLE_WEATHER_API.md`](docs/GOOGLE_WEATHER_API.md) for the full setup notes and pricing.

## Headless / CLI Mode

Fetch and print the weather once, without opening the GUI — useful for scripting or a status-bar widget:

```bash
open-weather-wizard --headless              # human-readable text
open-weather-wizard --headless --json       # machine-readable JSON

# One-off query, ignoring the saved config:
open-weather-wizard --headless --city Chicago --state IL --country US --provider google
```

Needs an API token the same way the GUI does — either already saved via Preferences (read from the OS keychain), or, for a machine without one available (e.g. a headless Linux server with no D-Bus session), set `OPEN_WEATHER_WIZARD_API_TOKEN`. Exits `0` on success, `1` on failure, for use in scripts/cron.

## Troubleshooting

**Weather data not loading:**

- Check your internet connection and that your API key for the selected provider is valid.
- Run with verbose logging to see request/response details: `RUST_LOG=debug cargo run`.

**Reset configuration:**

- Delete the config file at the path listed above and restart the app.

## Contributing

Contributions are welcome! If you have a feature request, bug report, or want to contribute to the code, please feel free to open an issue or a pull request.

## License

This project is licensed under the MIT License — see the [LICENSE](LICENSE) file for details.
