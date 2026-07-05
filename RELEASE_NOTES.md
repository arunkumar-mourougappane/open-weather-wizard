# Release Notes

Update this file before pushing a release tag (`vX.Y.Z`) — its contents
become the body of the GitHub Release created by
`.github/workflows/release.yml`.

## Unreleased

**CLI**

- New `--headless` mode: fetches weather (and forecast) once and prints it to
  stdout without opening the GUI — useful for scripting, status-bar widgets,
  or a display-less machine. `--json` for machine-readable output;
  `--city`/`--state`/`--country`/`--provider` for a one-off query without
  touching the saved config. Still needs an API token, either the one saved
  via the GUI's Preferences (OS keychain) or, for a machine with no keychain
  available, the `OPEN_WEATHER_WIZARD_API_TOKEN` environment variable.

**Interface**

- First-run setup: if no config file exists yet, Preferences now opens
  automatically alongside the main window with a welcome banner and a "Get
  an API key" link for whichever provider is selected, instead of silently
  attempting a fetch that's guaranteed to fail with no token configured.
- The location section in Preferences is now labeled "Home" and has a
  "Detect my location" button. It now tries your OS's native location
  service first (macOS/Windows/Linux) for real GPS/Wi-Fi-based accuracy,
  only falling back to an (inherently less accurate) IP-based lookup if
  native location isn't available or permitted — either way, it's a
  starting point you can still edit by hand.

**Providers**

- Google Weather is now a real, live provider (Google Maps Platform's
  Weather API) instead of a hardcoded mock — current conditions and a real
  5-day forecast, both requiring your own Google Cloud API key (entered the
  same way as the OpenWeatherMap token, in Preferences). Because Google's
  free tier is capped at 10,000 calls/month, this provider auto-refreshes
  every 15 minutes instead of OpenWeatherMap's 30 seconds. See
  `docs/GOOGLE_WEATHER_API.md` for the full integration details.

## v0.2.0

The GTK4 UI from v0.1.0 is gone — the app is rebuilt from scratch on
[iced](https://iced.rs), with a redesigned interface and a lot of new
functionality. See `docs/ARCHITECTURE.md` for the rewrite rationale.

**Interface**

- Redesigned main view: current-conditions card with a color-coded stat
  grid (feels-like, humidity, wind, pressure, visibility, sunrise/sunset).
- 5-day forecast as a centered carousel; tap any day to see its detail
  (hi/lo, feels-like, humidity, wind, pressure, visibility, chance of rain)
  right in the main card.
- Every weather condition gets its own animated, GPU-composited Lottie
  icon (sun, rain, snow, clouds, thunderstorms, drizzle, fog, haze, wind,
  tornado, and more) rendered directly through iced's `wgpu` surface.
- Background refresh no longer blanks the screen back to a loading state;
  changed values cross-fade in place. A shimmer skeleton placeholder is
  shown only on first load, replacing the old spinner.
- Dark mode and °C/°F, both live-previewed in Preferences before saving.
- Preferences and About are now separate windows with validated fields,
  grouped sections, and a manual Refresh button in the toolbar.

**Providers**

- Two weather providers: live data from OpenWeatherMap, or a built-in
  Google Weather mock provider that needs no API key.
- Forecast parsing now includes wind, chance of precipitation, and
  visibility per day.

**Security**

- The OpenWeatherMap API token is no longer stored as base64 in
  `config.json` — it now lives in the OS's native credential store
  (macOS Keychain / Windows Credential Manager / Linux Secret Service).
  Existing config files are migrated automatically on first load.

**Packaging**

- Releases now include installable app packages alongside the raw
  binaries: a `.dmg` for macOS, a `.deb` for Linux, and an `.exe` (NSIS)
  installer for Windows.
- New app icon.
