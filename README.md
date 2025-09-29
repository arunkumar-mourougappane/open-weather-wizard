# Weather Wizard - A Rust-based GTK Weather Application

Weather Wizard is a GTK4-based desktop weather application written in Rust. It fetches weather data from various providers and displays it in a simple and elegant user interface.

## Features

- **Current Weather:** Displays the current temperature, a description of the weather, and humidity.
- **Weather Icons:** Shows animated weather icons that correspond to the current weather conditions.
- **Multiple Providers:** Supports multiple weather data providers, including OpenWeatherMap and Google Weather.
- **Auto-Refresh:** Automatically updates the weather data at regular intervals.
- **Configuration:** Allows users to configure their location and select their preferred weather provider through a preferences window.

## Technology Stack

- **Rust:** The core application logic is written in Rust, providing performance and safety.
- **GTK4 & Relm4:** The graphical user interface is built using GTK4 and the Relm4 framework, following The Elm Architecture.
- **Tokio:** Asynchronous operations, such as fetching weather data, are handled by Tokio.
- **Serde:** For serializing and deserializing configuration and API data.

## Animated Icons

The application uses a beautiful set of animated SVG weather icons provided by [amCharts](https://www.amcharts.com/free-animated-svg-weather-icons/) and sourced from the [Makin-Things/weather-icons](https://github.com/Makin-Things/weather-icons) repository. These icons provide dynamic animations for various weather conditions like rain, sun, clouds, and snow.

For more details on the icon integration, see [docs_meta_data/ANIMATED_ICONS.md](docs_meta_data/ANIMATED_ICONS.md).

## Getting Started

These instructions will get you a copy of the project up and running on your local machine for development and testing purposes.

### Prerequisites

- **Rust:** Ensure you have a recent version of Rust and Cargo installed. You can find installation instructions at [rust-lang.org](https://www.rust-lang.org/).
- **GTK4:** You need to have the GTK4 development libraries installed on your system. The installation process varies depending on your operating system.

  - **Ubuntu/Debian:**

```bash
sudo apt-get install libgtk-4-dev
```

  - **Fedora:**

```bash
sudo dnf install gtk4-devel
```

  - **Arch Linux:**

```bash
sudo pacman -S gtk4
```

### Building

1. Clone the repository:

```bash
git clone https://github.com/arunkumar-mourougappane/meteo-wizard.git
cd meteo-wizard
```

1. Download the weather icon assets:

```bash
./get_weather_icons.sh
```

1. Build the project:

```bash
cargo build
```

### Running

To run the application, use the following command:

```bash
cargo run
```

## Configuration

The first time you run the application, it will create a configuration file at `~/.config/open-weather-wizard/config.json`. You can edit this file to set your location and API key.

Alternatively, you can use the preferences window within the application to configure these settings.

### API Keys

To fetch weather data, you need an API key from one of the supported weather providers:

- **OpenWeatherMap:** You can get a free API key by signing up on their website.
- **Google Weather:** (This provider might require specific setup or might not be available for free.)

Once you have an API key, you can add it to the `config.json` file or enter it in the preferences window.

## Project Structure

The project is organized into the following directories:

```
.
├── assets/         # Contains static assets like weather icons.
├── docs_meta_data/ # Contains documentation-related files.
├── examples/       # Contains example code.
├── src/            # Contains the source code.
│   ├── config.rs   # Handles application configuration.
│   ├── lib.rs      # Main library file.
│   ├── main.rs     # Main application entry point.
│   ├── style.css   # CSS for styling the application.
│   ├── ui/         # Contains UI-related modules.
│   └── weather_api/ # Contains modules for different weather APIs.
├── Cargo.toml      # The package manifest for Rust.
└── README.md       # This file.
```

## Contributing

Contributions are welcome! If you have a feature request, bug report, or want to contribute to the code, please feel free to open an issue or a pull request.

## License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.
