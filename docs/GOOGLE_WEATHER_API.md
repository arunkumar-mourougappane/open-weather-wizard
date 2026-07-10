# Google Weather API

Research notes on Google's real Weather API, originally written to plan
turning `src/weather_api/google_weather_api.rs` from a hardcoded mock into a
live `WeatherProvider` implementation. **That implementation has since
landed** (see `google_weather_api.rs` directly for the real code, and
`docs/ARCHITECTURE.md`'s "First-run setup and location detection" section for
how it's wired into the rest of the app) — this document is kept as
reference for the API itself (endpoints, auth, pricing, response shapes),
not as a to-do list. Sections describing gaps in "the current mock" below
are historical context for *why* certain design decisions were made, not a
description of the shipped code.

This is **not** part of the Google Maps JavaScript SDK — it's a standalone
REST API under Google Maps Platform's "Environment" product category
(alongside Air Quality, Pollen, and Solar).

Official docs: https://developers.google.com/maps/documentation/weather

## Setup and authentication

- Requires a Google Cloud project with billing enabled and the Weather API
  enabled on it, same as any other Maps Platform API.
- Auth is a plain API key passed as the `key` query parameter — no OAuth,
  no request signing. This matches how `WeatherProviderFactory::create_provider`
  already requires a token for every `WeatherApiProvider` variant, so a real
  implementation fits the same `keyring`-backed token storage `src/config.rs`
  already uses for OpenWeatherMap.
- A no-cost "Maps Demo Key" exists for trying the API without attaching
  billing, but it's rate-limited/watermarked and not meant for production use.
- Every real request needs a lat/lon pair (`location.latitude` /
  `location.longitude`), not a city name — there's no built-in geocoding-by-name
  step in this API. `location_config_to_location()` in
  `src/weather_api/weather_provider.rs` currently hardcodes `lat: 0.0, lon: 0.0`
  for exactly this reason (OpenWeatherMap does its own name→coords lookup
  server-side); a real Google integration would need geocoding done separately
  first, e.g. via the Geocoding API, or by asking the user for coordinates
  directly.

## Base URL and endpoints

Base URL: `https://weather.googleapis.com`

| Endpoint | Method + path | Returns |
|---|---|---|
| Current conditions | `GET /v1/currentConditions:lookup` | current weather at a point |
| Daily forecast | `GET /v1/forecast/days:lookup` | up to 10 days, starting today |
| Hourly forecast | `GET /v1/forecast/hours:lookup` | up to 240 hours, starting this hour |
| Hourly history | `GET /v1/history/hours:lookup` | up to 24 hours of past cached conditions |
| Public alerts | `GET /v1/publicAlerts:lookup` | active weather alerts for a location |

All are `GET` with query parameters — no request bodies.

## `currentConditions:lookup`

Maps most directly onto `WeatherProvider::get_weather` /
`openweather_api::ApiResponse`.

**Query parameters**

| Param | Required | Notes |
|---|---|---|
| `key` | yes | API key |
| `location.latitude` | yes | float |
| `location.longitude` | yes | float |
| `unitsSystem` | no | `METRIC` (default) or `IMPERIAL` |

**Example request**

```
GET https://weather.googleapis.com/v1/currentConditions:lookup
    ?key=YOUR_API_KEY
    &location.latitude=37.4220
    &location.longitude=-122.0841
    &unitsSystem=IMPERIAL
```

**Example response**

```json
{
  "currentTime": "2025-01-28T22:13:56.723468335Z",
  "timeZone": { "id": "America/Los_Angeles" },
  "isDaytime": true,
  "weatherCondition": {
    "iconBaseUri": "https://maps.gstatic.com/weather/v1/sunny",
    "description": { "text": "Sunny", "languageCode": "en" },
    "type": "CLEAR"
  },
  "temperature": { "degrees": 56.6, "unit": "FAHRENHEIT" },
  "feelsLikeTemperature": { "degrees": 55.7, "unit": "FAHRENHEIT" },
  "dewPoint": { "degrees": 33.9, "unit": "FAHRENHEIT" },
  "relativeHumidity": 42,
  "uvIndex": 1,
  "precipitation": {
    "probability": { "percent": 0, "type": "RAIN" },
    "qpf": { "quantity": 0, "unit": "INCHES" }
  },
  "thunderstormProbability": 0,
  "wind": {
    "direction": { "degrees": 335, "cardinal": "NORTH_NORTHWEST" },
    "speed": { "value": 5, "unit": "MILES_PER_HOUR" },
    "gust": { "value": 11, "unit": "MILES_PER_HOUR" }
  },
  "visibility": { "distance": 10, "unit": "MILES" },
  "cloudCover": 0
}
```

Notably absent compared to `ApiResponse`: no barometric `pressure` field, and
no `sunrise`/`sunset` timestamps in *current conditions* (those live in the
daily forecast's `sunEvents` instead) — a real mapping would need to pull
those from a separate `forecast/days:lookup` call rather than one response,
unlike OpenWeatherMap's single-endpoint `Sys{sunrise,sunset}`.

## `forecast/days:lookup`

Maps onto `WeatherProvider::get_forecast` / `forecast::ForecastResponse`.
Currently the mock returns `days: vec![]` unconditionally (see
`ARCHITECTURE.md`'s note on the intentional empty placeholder) — this is the
endpoint that would replace that.

**Query parameters**

| Param | Required | Notes |
|---|---|---|
| `key` | yes | API key |
| `location.latitude` / `location.longitude` | yes | floats |
| `days` | no | default 10, max 10 |
| `pageSize` | no | default 5; response is paginated |
| `pageToken` | no | from a previous response's `nextPageToken` |

**Example request**

```
GET https://weather.googleapis.com/v1/forecast/days:lookup
    ?key=YOUR_API_KEY
    &location.latitude=37.4220
    &location.longitude=-122.0841
    &days=5
```

**Response shape**

```json
{
  "forecastDays": [
    {
      "interval": { "startTime": "...", "endTime": "..." },
      "displayDate": { "year": 2025, "month": 1, "day": 29 },
      "maxTemperature": { "degrees": 58, "unit": "FAHRENHEIT" },
      "minTemperature": { "degrees": 44, "unit": "FAHRENHEIT" },
      "feelsLikeMaxTemperature": { "...": "..." },
      "feelsLikeMinTemperature": { "...": "..." },
      "sunEvents": { "sunriseTime": "...", "sunsetTime": "..." },
      "moonEvents": { "...": "..." },
      "daytimeForecast": {
        "weatherCondition": { "type": "...", "iconBaseUri": "..." },
        "relativeHumidity": 40,
        "uvIndex": 6,
        "precipitation": { "probability": {"...": "..."}, "qpf": {"...": "..."} },
        "thunderstormProbability": 5,
        "wind": { "direction": {"...": "..."}, "speed": {"...": "..."}, "gust": {"...": "..."} },
        "cloudCover": 20,
        "iceThickness": { "...": "..." }
      },
      "nighttimeForecast": { "...": "same shape as daytimeForecast" }
    }
  ],
  "timeZone": { "id": "America/Los_Angeles" },
  "nextPageToken": "..."
}
```

Key structural difference from `forecast::aggregate_daily()`'s current
OpenWeatherMap-driven design: Google's daily forecast is **natively daily**
with an explicit day/night split (`daytimeForecast` 7am–7pm /
`nighttimeForecast` 7pm–7am), not bucketed from 3-hour intervals. A Google
implementation of `get_forecast` would be simpler than
`aggregate_daily()` — no midday-condition-selection heuristic needed — but
would need its own mapping since the day/night split has no equivalent in
`ForecastResponse`'s current shape.

## Pricing

- $0.15 per 1,000 calls (each endpoint call is a separate billable event),
  after a free tier of 10,000 calls/month.
- Optional bundled subscription plans exist (Starter/Essentials/Pro,
  $100–$1,200/month) covering combined Maps Platform SKU usage, but pay-as-you-go
  is the relevant model for a single desktop app.
- The auto-refresh interval is user-configurable (defaults to 15 minutes for Google Weather,
  with a hardcoded floor of 15 minutes enforced in preferences validation and subscription tick setup).
  At 15 minutes, a single always-running instance generates 3 billable calls per refresh (current conditions + 2 forecast pages),
  totaling ~8,640 calls/month, which stays safely within the 10,000 free monthly calls. Lowering the refresh interval below
  15 minutes is disallowed for Google Weather to protect the free tier budget.

## What's not covered by the current mock

- Public alerts (`publicAlerts:lookup`) and hourly forecast/history have no
  equivalent in `WeatherProvider` at all currently — only current conditions
  and daily forecast are modeled.
- No geocoding: a real integration needs lat/lon resolved before calling
  either endpoint, unlike `OpenWeatherProvider` which resolves city names
  server-side.

## Sources

- [Weather API overview](https://developers.google.com/maps/documentation/weather/overview)
- [Get current conditions](https://developers.google.com/maps/documentation/weather/current-conditions)
- [Get daily forecast](https://developers.google.com/maps/documentation/weather/daily-forecast)
- [Get hourly forecast](https://developers.google.com/maps/documentation/weather/hourly-forecast)
- [Weather API REST reference](https://developers.google.com/maps/documentation/weather/reference/rest)
- [Weather API FAQ](https://developers.google.com/maps/documentation/weather/faq)
- [Weather API usage and billing](https://developers.google.com/maps/documentation/weather/usage-and-billing)
