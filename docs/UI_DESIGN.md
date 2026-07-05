# UI Design

Screen layouts and the CSS → iced styling translation for the iced rewrite.
See `ARCHITECTURE.md` for the state/message machinery behind these views.

## Windowing

Three independent OS windows (`iced::daemon`, not an in-app overlay):

| Window | Opened by | Size | Notes |
|---|---|---|---|
| Main | app boot | 720×480, resizable | never closes without exiting the app |
| Preferences | `Message::OpenPreferences` | 500×420 | modal-*feeling*, not modal-*enforced*: a real second window |
| About | `Message::OpenAbout` | 420×360, fixed | static content |

Only one instance of Preferences/About can be open at a time (`update()`
no-ops `OpenPreferences`/`OpenAbout` if the corresponding `Option<window::Id>`
is already `Some`).

## Main screen (`src/ui/main_screen.rs`)

```
column [padding 12, spacing 12]
├── row "toolbar" [spacing 8, centered]
│   ├── text "Weather Wizard" (20px, bold)
│   ├── space::horizontal()               <- pushes buttons to the right
│   ├── button "Preferences" -> OpenPreferences
│   └── button "About" -> OpenAbout
├── container [centered, padding 20]
│   └── (state-dependent, see below)
└── forecast_row::view(&state.forecast)   <- omitted entirely if None (see below)
```

Content depends on `WeatherStatus`:

- **Loading** → `text("Fetching weather...")`, 18px.
- **Error(message)** → `text("Error: {message}")`, 16px, red (`Color::from_rgb(0.8, 0.1, 0.1)`).
- **Loaded(data)** →
  ```
  column [spacing 6, centered]
  ├── icons::view(symbol, 128.0)              <- animated Lottie or static SVG
  ├── text(location) (24px, bold)
  ├── text(temp, "{:.1}°C") (30px, bold)
  ├── text(description) (18px, italic)
  └── text("Humidity: {}%") (14px)
  ```

## Forecast row (`src/ui/forecast_row.rs`)

A horizontally-scrollable row of day cards, appended below the current
conditions. **Omitted from the layout entirely** — not shown as an empty
placeholder — whenever `ForecastStatus` is `Loading`, `Error`, or `Loaded`
with an empty `days` list. Both current providers (OpenWeatherMap, Google
Weather) return real forecasts today; this path exists for a future
provider that might not (see `src/ui/forecast_row.rs`'s own doc comment).

```
scrollable [horizontal, width Fill]
└── row [spacing 12]
    └── day_card(day) for each ForecastDay   <- up to 5, oldest first
```

`day_card`:

```
container [padding 8]
└── column [spacing 4, centered, width 96]
    ├── text(date) (14px, bold)              <- "YYYY-MM-DD"
    ├── icons::view(day.symbol, 48.0)
    ├── text("{max:.0}° / {min:.0}°") (14px)
    └── text(description) (12px)
```

## Preferences (`src/ui/preferences.rs`)

```
container
└── column [spacing 20, padding 20, width Fill]
    ├── column [spacing 12]  "form"
    │   ├── labeled_row "Weather Provider:" -> pick_list(OpenWeather | GoogleWeather)
    │   ├── labeled_row "API Token:"        -> text_input (secure/masked)
    │   ├── text "Default Location" (16px)
    │   ├── labeled_row "City:"             -> text_input
    │   ├── labeled_row "State/Province:"   -> text_input
    │   └── labeled_row "Country:"          -> text_input
    └── row [spacing 8, centered]  "buttons"
        ├── button "Cancel" -> Cancel
        └── button "Save"   -> Save
```

`labeled_row(label, field)` = `row![text(label).width(160), field]`, 12px
spacing, centered vertically — the fixed 160px label column keeps all five
fields aligned.

Field state (`preferences::State`) is populated from `AppConfig` when the
window opens (`State::from_config`) and only committed back
(`State::apply_to`) on `Save`; `Cancel` discards it. `Save` in the parent
`AppState::update` also persists via the existing `ConfigManager::save_config`
and triggers `RefreshRequested`.

## About (`src/ui/about.rs`)

```
container [width Fill, centered]
└── column [spacing 8, centered]
    ├── image(icon.png) (64×64)             <- if the embedded asset loads
    ├── text "Weather Wizard" (20px)
    ├── text "v{CARGO_PKG_VERSION}"
    ├── text(authors, joined ", ")
    ├── text "MIT License"
    └── text(CARGO_PKG_HOMEPAGE) (12px)
```

Same `env!(CARGO_PKG_*)` macros as the previous GTK `AboutDialog`.

## CSS → iced style translation

`src/style.css` is gone; iced has no external stylesheet, so every value
that used to be a CSS rule is now an inline `.size()`/`.font()`/`.color()`
call at the call site. Nothing from the original visual design was
intentionally dropped — this table is the migration record:

| CSS (old) | iced (new) | Used in |
|---|---|---|
| `.location-label { font-size: 24px; font-weight: bold; }` | `.size(24).font(BOLD)` | `main_screen::view` |
| `.weather-temp { font-size: 30px; font-weight: bold; }` | `.size(30).font(BOLD)` | `main_screen::view` |
| `.weather-description { font-size: 18px; font-style: italic; }` | `.size(18).font(ITALIC)` | `main_screen::view` |
| `.weather-humidity { font-size: 14px; }` | `.size(14)` | `main_screen::view` |
| `.weather-symbol { font-size: 80px; }` (fallback only; real size was `set_pixel_size`) | `icons::view(symbol, 128.0)` | `main_screen::view` |

`BOLD`/`ITALIC` are small `const Font` values (`Font { weight: Weight::Bold,
..Font::DEFAULT }` etc.) defined once per module that needs them, rather than
a shared theme — there being only two-and-a-half distinct text styles in the
whole app didn't justify a theme abstraction.
