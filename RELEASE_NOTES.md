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
