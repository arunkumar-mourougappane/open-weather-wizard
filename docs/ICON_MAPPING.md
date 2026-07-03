# Icon Mapping

How an OpenWeatherMap condition string becomes an on-screen icon. Every
`WeatherSymbol` now has a hand-authored Lottie animation; the static SVGs
remain only as a fallback if a composition somehow fails to load.

## Condition string → `WeatherSymbol`

`openweather_api::get_weather_symbol(&str) -> WeatherSymbol` (unchanged since
the GTK version) maps OpenWeatherMap's `weather[].main` string to one of:

| Condition string | `WeatherSymbol` |
|---|---|
| `Clear` | `Clear` |
| `Clouds` | `Clouds` |
| `Rain` | `Rain` |
| `Drizzle` | `Drizzle` |
| `Thunderstorm` | `Thunderstorm` |
| `Snow` | `Snow` |
| `Mist` | `Mist` |
| `Smoke` | `Smoke` |
| `Haze` | `Haze` |
| `Dust` | `Dust` |
| `Fog` | `Fog` |
| `Sand` | `Sand` |
| `Ash` | `Ash` |
| `Squall` | `Squall` |
| `Tornado` | `Tornado` |
| *(anything else)* | `Default` |

## `WeatherSymbol` → icon

`src/ui/icons.rs::view(symbol, size)` dispatches per symbol to its own
`assets/lottie/*.json` composition; conditions without a visually distinct
animation of their own share the closest match:

| `WeatherSymbol` | Lottie composition |
|---|---|
| `Clear` | `sun.json` |
| `Clouds`, `Default` | `clouds.json` |
| `Rain` | `rain.json` |
| `Drizzle` | `drizzle.json` |
| `Thunderstorm` | `thunderstorm.json` |
| `Snow` | `snow.json` |
| `Mist`, `Smoke`, `Fog` | `fog.json` |
| `Haze`, `Dust`, `Sand`, `Ash` | `haze.json` |
| `Squall` | `wind.json` |
| `Tornado` | `tornado.json` |

The static SVGs under `assets/static/` (mapped 1:1 per `WeatherSymbol` in
`icons::asset_path`) are only ever used as a fallback, if a composition
somehow fails to load at startup.

## Animated icon authoring

`assets/lottie/*.json` are hand-authored, not converted from the existing
SVGs — there's no reliable automated CSS-animation → Lottie converter, and
reverse-engineering the existing `@keyframes` blocks by hand was accurate
enough at this simple a level of animation. Each reuses the transform *type*
(rotate / translate / opacity) and rough magnitude from the corresponding
SVG in `assets/animated/`, not literal keyframe values.

Every cloud-based composition (`clouds`, `rain`, `drizzle`, `thunderstorm`)
builds its cloud from several overlapping ellipses that **must share one
fill color** — the first pass used a different shade per lobe, which read as
stacked flat ovals with visible seams instead of one fluffy shape. Rain and
drizzle drops are teardrop-shaped bezier paths (`"ty": "sh"`, straight
in/out tangents for the point, a rounded arc for the base) with staggered,
opacity-faded fall cycles per drop, rather than all drops falling in lockstep
and popping back to the top in sync.

`lottie::frame_at` (`src/ui/lottie/mod.rs`) computes the current frame as
`start + (elapsed * frame_rate) % duration` — a hard wrap back to frame 0,
not a bounce. Any shape whose position/opacity differs between its first and
last keyframe will visibly snap at that wrap; the fix used throughout this
set is to fade a shape's opacity to zero before the wrap point (e.g. each
rain/drizzle drop) rather than try to keep every property continuous across
the loop boundary.

These are simple, representative animations, not polished icon art — a
reasonable next step if the visual bar needs to be higher is either
hand-refining these in a proper Lottie/After Effects authoring tool, or
sourcing a licensed, professionally-produced Lottie weather icon pack (which
would also mean revisiting the licensing note below, since it covers the
current SVG-derived set specifically).

## Licensing / attribution

The icon set in `assets/animated/` and `assets/static/` (the *unanimated
counterparts*, still used as the fallback for every non-Lottie condition) is
sourced from [amCharts' free animated SVG weather
icons](https://www.amcharts.com/free-animated-svg-weather-icons/) via the
[Makin-Things/weather-icons](https://github.com/Makin-Things/weather-icons)
repository. Re-run `get_weather_icons.sh` to refresh from upstream.

The `assets/lottie/*.json` files are original work for this project
(hand-authored, not derived from or embedding the amCharts assets), licensed
under this repository's own license (see `LICENSE`).
