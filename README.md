# Open Weather Wizard - A Rust-based GTK Weather Application

Open Weather Wizard is a simple and elegant desktop weather application written in Rust and GTK 4. It provides current weather, forecasts, and animated weather icons packaged with the app. Designed to be lightweight, extensible, and easy to localize or re-skin for different platforms and icon sets.

## Features

- **Current Weather:** Displays the current temperature, weather description, and humidity with real-time updates
- **Animated Weather Icons:** Shows beautiful animated SVG weather icons with smooth animations for rain, snow, sun, clouds, and more weather conditions
- **App Icons:** Features custom app icons in the About dialog and window titlebar for professional appearance
- **Multiple Providers:** Supports multiple weather data providers, including OpenWeatherMap and Google Weather
- **Auto-Refresh:** Automatically updates the weather data at configurable intervals (default: 30 seconds)
- **Preferences:** Configure location, weather provider, and API keys through an intuitive preferences window
- **Embedded Assets:** All weather icons and app icons are bundled with the application for offline operation

## Technology Stack

- **Rust:** The core application logic is written in Rust, providing performance and safety.
- **GTK4:** The graphical user interface is built using GTK4 with native Rust bindings.
- **Tokio:** Asynchronous operations, such as fetching weather data, are handled by Tokio.
- **Serde:** For serializing and deserializing configuration and API data.
- **Reqwest:** For making HTTP requests to weather API services.

## Animated Icons

The application features beautiful animated SVG weather icons provided by [amCharts](https://www.amcharts.com/free-animated-svg-weather-icons/) and sourced from the [Makin-Things/weather-icons](https://github.com/Makin-Things/weather-icons) repository. These icons provide smooth, dynamic animations for various weather conditions:

- **Rain:** Animated falling raindrops with realistic motion
- **Snow:** Gentle falling snowflakes with varying speeds
- **Sun:** Rotating sun with shimmering ray effects
- **Clouds:** Moving cloud formations with subtle animations
- **Thunderstorms:** Flickering lightning effects and storm activity
- **Wind:** Swirling wind patterns and motion effects

### Animation Technology

The app uses GTK4's native SVG rendering capabilities through librsvg to display animated SVGs. Weather icons are:

- Embedded as assets at compile-time for offline operation
- Written to temporary files at runtime to enable animation playbook
- Rendered using the system's librsvg library which supports CSS animations and SMIL

For complete technical details, see [docs_meta_data/ANIMATED_ICONS.md](docs_meta_data/ANIMATED_ICONS.md).

## Installation

### Quick Install (Recommended)

The easiest way to install Meteo Wizard is using the provided installation script that handles both binary installation and desktop integration:

```bash
git clone https://github.com/arunkumar-mourougappane/open-weather-wizard.git
cd open-weather-wizard
./install.sh
```

This script will:

- Install the `open-weather-wizard` binary using cargo
- Set up desktop integration (application menu entry, icons)
- Create an uninstall script for easy removal

### Manual Installation

#### Prerequisites

You need Rust installed on your system. If you don't have Rust installed, you can download it from [the official Rust installation guide](https://rustup.rs/).

#### Install Binary Only

To install just the binary without desktop integration:

```bash
# From source
git clone https://github.com/arunkumar-mourougappane/open-weather-wizard.git
cd open-weather-wizard
cargo install --path .

# Or when published to crates.io (future)
cargo install open-weather-wizard
```

#### From Source (Development)

To run from source without installing:

```bash
git clone https://github.com/arunkumar-mourougappane/open-weather-wizard.git
cd open-weather-wizard
cargo run
```

### Uninstallation

If you used the installation script, you can easily uninstall:

```bash
open-weather-wizard-uninstall
```

For manual installation:

```bash
# Remove binary
cargo uninstall open-weather-wizard

# Remove desktop integration (if manually added)
rm -f ~/.local/share/applications/open-weather-wizard.desktop
rm -f ~/.local/share/icons/hicolor/*/apps/open-weather-wizard.*

# Remove configuration (optional)
rm -rf ~/.config/open-weather-wizard/
rm -rf ~/.cache/open-weather-wizard/
```

## Configuration

The first time you run the application, it will create a configuration file at `~/.config/open-weather-wizard/config.json`. You can edit this file to set your location and API key.

Alternatively, you can use the preferences window within the application to configure these settings.

### First Run

When you run the application for the first time, it will use default configuration settings. You can view available command-line options with:

```bash
./target/release/open-wearther-wizard --help
```

### API Keys

To fetch weather data, you need an API key from one of the supported weather providers:

- **OpenWeatherMap:** You can get a free API key by signing up on their website.
- **Google Weather:** (This provider might require specific setup or might not be available for free.)

Once you have an API key, you can add it to the `config.json` file or enter it in the preferences window.

## Troubleshooting

### Common Build Issues

**GTK 4 not found:**

```bash
# Ubuntu/Debian
sudo apt-get install libgtk-4-dev pkg-config

# Fedora
sudo dnf install gtk4-devel pkgconf-devel

# Arch Linux
sudo pacman -S gtk4 pkgconf
```

**Missing dependencies:**

```bash
# Install additional development tools if needed
sudo apt-get install build-essential
```

**Display issues:**

- Ensure you have a desktop environment running (GNOME, KDE, etc.)
- For headless systems, you may need to set up X11 forwarding or use a virtual display

### Runtime Issues

**Config file location:**

- Configuration is stored at: `~/.config/open-weather-wizard/config.json`
- Delete this file to reset to defaults

**Weather data not loading:**

- Check your internet connection
- Verify your API key is valid
- Check the application logs for error messages

## Build Status

| Platform | Status | Notes |
|----------|--------|-------|
| Ubuntu 22.04 LTS | ✅ **Compiled Successfully** | Rust 1.89.0, GTK 4.x |
| Debug Build | ✅ 104 MB | With debug symbols |
| Release Build | ✅ 5.6 MB | Optimized binary |

## Project Structure

The project is organized into the following directories:

```text
.
├── assets/              # Contains static and animated weather icons + app icon
│   ├── animated/        # Animated SVG weather icons (used by app)
│   ├── static/          # Static SVG weather icons (fallback)
│   └── icon/            # App icons (PNG format for About dialog and titlebar)
├── docs_meta_data/      # Contains documentation-related files
├── examples/            # Contains example code and demos
├── src/                 # Contains the source code
│   ├── config.rs        # Handles application configuration
│   ├── lib.rs           # Main library file
│   ├── main.rs          # Main application entry point
│   ├── style.css        # CSS for styling the application
│   ├── ui/              # Contains UI-related modules
│   │   ├── about.rs     # About dialog with app icon
│   │   ├── build_elements.rs # UI helpers and animated icon loading
│   │   ├── mod.rs       # Main UI setup with titlebar icon
│   │   └── preferences.rs # Settings/preferences window
│   └── weather_api/     # Contains modules for different weather APIs
├── target/              # Compiled binaries (generated during build)
├── Cargo.toml           # The package manifest for Rust (includes [[bin]] section)
├── Cargo.lock           # Dependency lock file
├── get_weather_icons.sh # Script to download weather icons
├── install.sh           # Installation script with desktop integration
├── open-weather-wizard.desktop # Linux desktop entry file
├── LICENSE              # MIT License file
└── README.md            # This file
```

## Contributing

Contributions are welcome! If you have a feature request, bug report, or want to contribute to the code, please feel free to open an issue or a pull request.

## License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.
