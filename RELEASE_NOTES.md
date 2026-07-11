# Release Notes

Update this file before pushing a release tag (`vX.Y.Z`) — its contents
become the body of the GitHub Release created by
`.github/workflows/release.yml` (the whole file, verbatim — there's no
section-extraction, so this should only ever describe the release about to
ship). Past releases' notes live under
[`docs/previous_releases/`](docs/previous_releases/) instead of
accumulating here.

## Unreleased

**Interface**

- A persistent tray/menu bar icon now shows current conditions at a glance (macOS menu bar tested; Windows/Linux implemented but unverified in this environment — see `docs/ARCHITECTURE.md`). On macOS it renders as a template image, so it adapts to light/dark menu bars like the system's own icons, and shows the current temperature as compact text next to it ("68°F"); the icon itself now changes to match the current condition (sun, cloud, rain, etc.) instead of staying generic, and gets a "⚠" badge on the tooltip and title whenever a severe or extreme weather alert is active. Its tooltip reflects the same live weather data as the main window. With the tray icon present, closing the main window now tucks it away instead of quitting — left-clicking the tray icon un-minimizes and focuses it back (and Preferences/About windows can be recovered the same way if they're hidden or minimized); right-clicking quits (the tray library has no context-menu support, so this is the only quit path once closing no longer does). If the tray icon fails to create, closing the window still quits as before, since there'd be no way to get it back otherwise. Also fixes the Dock icon not appearing for a plain `cargo run`/`cargo build` binary on macOS (`winit`'s window-icon API is a documented no-op there; packaged release builds already got theirs from `Info.plist`, this now also sets it directly via AppKit for dev builds). (#56)
- The app now supports multiple saved locations instead of a single "Home". Preferences' Locations section lets you add, rename, remove, and reorder saved places (each with its own city/state/country and "Detect my location" prefill); the main window gets a small switcher strip to flip between them without opening Preferences, and `--headless` mode gets a matching `--location <name>` flag. Existing single-location config files are migrated automatically into a one-entry "Home" list. All locations still share the currently-configured provider/API key/language/theme -- per-location overrides for those are out of scope. (#55)
- Weather *descriptions* ("clear sky", "light rain", etc.) can now be requested in one of 12 languages via a new "Language" picker in Preferences, next to the Provider picker. This only affects the text the weather API returns, not the app's own UI chrome (buttons, labels, etc.), which remains English-only. Existing config files default to English, matching both providers' own API default. (#48)
- Dark mode is now a three-way "Theme" choice in Preferences (Light / Dark / Follow System) instead of a single toggle. "Follow System" matches the OS's current light/dark preference at launch and re-checks it on the same cadence as the weather refresh. Existing `dark_mode` settings are migrated automatically to an explicit Light/Dark choice, never silently switched to "Follow System". (#47)
- Active weather alerts (severe thunderstorm, flood, etc.) now surface as a banner above the current-conditions card when using the Google Weather provider, fetched via `publicAlerts:lookup` on the same refresh cycle as current conditions. OpenWeatherMap has no free-tier alerts equivalent, so it always shows none. (#45)
- The auto-refresh interval is now user-configurable in Preferences (presets: 30s / 1m / 5m / 15m / 30m). To protect Google Maps Platform rate limits, a 15-minute floor is strictly enforced when using the Google Weather provider. (#44)
- Preferences now has a "Verify API" button next to the API Token field:
  fires a single live request against the currently-typed
  provider/token/location (not the saved config) and shows an inline
  "✔ Connected" / "✗ <error>" result, so a bad token surfaces right next to
  the field instead of only after Save closes the window. (#43)

**Bug fixes**

- Pressure now respects the °F/°C unit toggle like every other stat: hPa in
  metric mode, inHg in imperial mode. Previously it was hardcoded to hPa
  regardless of the preference. (#42)

## v0.3.0

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
