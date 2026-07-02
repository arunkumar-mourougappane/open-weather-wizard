# Icon Mapping

How an OpenWeatherMap condition string becomes an on-screen icon, and which
subset of conditions get real animation vs. a static image.

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

`src/ui/icons.rs::view(symbol, size)` dispatches per symbol:

| `WeatherSymbol` | Animated (Lottie)? | Static SVG (`assets/static/`) |
|---|---|---|
| `Clear` | ✅ `lottie/sun.json` | `clear-day.svg` (fallback only) |
| `Clouds` | ✅ `lottie/clouds.json` | `cloudy-2-day.svg` (fallback only) |
| `Rain` | ✅ `lottie/rain.json` | `rainy-3.svg` (fallback only) |
| `Snow` | ✅ `lottie/snow.json` | `snowy-2.svg` (fallback only) |
| `Drizzle` | — | `rainy-1.svg` |
| `Thunderstorm` | — | `thunderstorms.svg` |
| `Mist` | — | `fog.svg` |
| `Smoke` | — | `fog.svg` |
| `Haze` | — | `haze.svg` |
| `Dust` | — | `dust.svg` |
| `Fog` | — | `fog.svg` |
| `Sand` | — | `dust.svg` |
| `Ash` | — | `dust.svg` |
| `Squall` | — | `wind.svg` |
| `Tornado` | — | `tornado.svg` |
| `Default` | — | `cloudy.svg` |

The four animated conditions cover the most common real-world weather, on
the theory that they're worth the authoring effort first; the rest render as
a static SVG (`iced::widget::svg`, no CSS-animation support, same limitation
the GTK version had via `librsvg`).

## Animated icon authoring

`assets/lottie/{sun,clouds,rain,snow}.json` are hand-authored, not converted
from the existing SVGs — there's no reliable automated CSS-animation →
Lottie converter, and reverse-engineering the existing `@keyframes` blocks by
hand was accurate enough at this simple a level of animation. Each reuses the
transform *type* (rotate / translate / opacity) and rough magnitude from the
corresponding SVG in `assets/animated/`, not literal keyframe values:

| Icon | Technique | Reference `@keyframes` (`assets/animated/`) |
|---|---|---|
| `sun.json` | Whole-layer rotation, 0°→360°, linear, looping | `am-weather-sun` in `clear-day.svg` (0°→360° over 9s) |
| `clouds.json` | Position ping-pong (3 keyframes: start → +16px → start) | `am-weather-cloud-2` in `cloudy-2-day.svg` (`translate(0,0)` → `translate(2px,0)` → back) |
| `rain.json` | Per-drop vertical fall (linear, wraps via the player's modulo loop — see below) | `am-weather-rain` in `rainy-3.svg` (`stroke-dashoffset` animation; a different SVG-specific technique, approximated here as falling shapes) |
| `snow.json` | Per-flake zigzag fall (3 intermediate keyframes) | `am-weather-snow` in `snowy-2.svg` (33%/66%/100% `translateX`/`translateY` zigzag) |

`lottie::frame_at` (`src/ui/lottie/mod.rs`) computes the current frame as
`start + (elapsed * frame_rate) % duration` — a hard wrap back to frame 0,
not a bounce. That's why `rain.json`/`snow.json` are authored as a single
linear fall from top to bottom rather than a there-and-back loop: the wrap
itself produces the "restart from the top" effect a real rain/snow loop
wants, with no extra keyframes needed. `sun.json`'s 0°→360° rotation wraps
seamlessly for the same reason (0° and 360° are the same orientation).
`clouds.json` is the one animation that *needs* an explicit return-to-start
keyframe, since a directional sway would visibly jump-cut on wrap otherwise.

These are simple, representative animations, not polished icon art — a
reasonable next step if the visual bar needs to be higher is either
hand-refining these four in a proper Lottie/After Effects authoring tool, or
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

The four `assets/lottie/*.json` files are original work for this project
(hand-authored, not derived from or embedding the amCharts assets), licensed
under this repository's own license (see `LICENSE`).
