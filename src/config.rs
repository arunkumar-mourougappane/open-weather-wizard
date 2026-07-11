//! # Application Configuration Management
//!
//! This module handles loading, saving, and managing application configuration,
//! which is persisted to a JSON file in the user's config directory.
//!
//! ## Key Components
//!
//! - **`AppConfig`**: The main struct representing all user-configurable settings,
//!   including the chosen weather provider and location. The API token is *not*
//!   part of this struct's persisted data -- see API Token Handling below.
//! - **`ConfigManager`**: A utility struct responsible for handling the file I/O
//!   for loading from and saving to `config.json`.
//! - **API Token Handling**: The API token is stored in the OS's native secure
//!   credential store (macOS Keychain, Windows Credential Manager, Linux Secret
//!   Service) via the [`keyring`] crate, never written to disk in `config.json`.
//!   Config files saved by older versions of this app had the token
//!   base64-"encoded" (not encrypted) directly in the file; `ConfigManager::
//!   load_config` transparently migrates any such token into the OS keychain
//!   the first time an old config file is loaded.

use base64::{Engine as _, engine::general_purpose::STANDARD};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use std::sync::LazyLock;

/// Identifies this app's entries in the OS credential store (the `service`
/// half of a `keyring::Entry`).
const KEYRING_SERVICE: &str = "open-weather-wizard";
/// The `username` half of the OpenWeatherMap API token's `keyring::Entry`.
/// Not a real username -- `keyring::Entry` just needs *some* stable
/// (service, username) pair to identify an entry.
const KEYRING_API_TOKEN_KEY: &str = "openweathermap-api-key";

/// An enum representing the supported weather API providers.
#[derive(Debug, Default, Clone, Serialize, Deserialize, PartialEq)]
pub enum WeatherApiProvider {
    #[default]
    OpenWeather,
    GoogleWeather,
}

impl std::fmt::Display for WeatherApiProvider {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            WeatherApiProvider::OpenWeather => write!(f, "OpenWeather"),
            WeatherApiProvider::GoogleWeather => write!(f, "Google Weather"),
        }
    }
}

/// The user's chosen theme: an explicit choice, or follow the OS's current
/// light/dark preference. `app::theme()` resolves `System` using a value
/// cached from a periodic `dark_light::detect()` poll rather than calling
/// it directly -- see `app::detect_system_theme_task`'s docs for why.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ThemePreference {
    Light,
    Dark,
    #[default]
    System,
}

impl std::fmt::Display for ThemePreference {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ThemePreference::Light => write!(f, "Light"),
            ThemePreference::Dark => write!(f, "Dark"),
            ThemePreference::System => write!(f, "Follow System"),
        }
    }
}

/// A struct representing the user's configured location.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LocationConfig {
    pub city: String,
    pub state: String,
    pub country: String,
}

impl Default for LocationConfig {
    fn default() -> Self {
        Self {
            city: "Peoria".to_string(),
            state: "IL".to_string(),
            country: "US".to_string(),
        }
    }
}

/// The main struct for the application's configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    pub weather_provider: WeatherApiProvider,
    pub location: LocationConfig,
    /// `#[serde(default)]` so config files saved before this field existed
    /// (or predating its introduction as a `ThemePreference` -- see
    /// `legacy_dark_mode` below) default to `ThemePreference::System`.
    #[serde(default)]
    pub theme_preference: ThemePreference,
    /// `#[serde(default)]` so config files saved before this field existed
    /// still load (missing -> `false`, i.e. Celsius). Conversion happens at
    /// display time in the UI layer -- the API is always fetched in metric,
    /// so toggling this doesn't trigger a re-fetch.
    #[serde(default)]
    pub use_fahrenheit: bool,
    /// Whether the app should automatically start when the user logs in.
    /// `#[serde(default)]` so older config files default to `false`.
    #[serde(default)]
    pub launch_at_login: bool,
    /// The user-configured auto-refresh interval in seconds.
    /// `#[serde(default)]` ensures missing values default to None, retaining
    /// default per-provider rates.
    #[serde(default)]
    pub refresh_interval_secs: Option<u64>,
    /// Present only to read config files saved by older versions of this
    /// app, which stored the API token base64-"encoded" (not encrypted)
    /// directly here. `#[serde(skip_serializing)]` means this is never
    /// written back out -- `ConfigManager::load_config` migrates it into
    /// the OS keychain and the field naturally disappears from
    /// `config.json` after the very next save. Not `pub`: nothing outside
    /// this module should ever read the *token* through this field again,
    /// only through `get_api_token`/`set_api_token`.
    #[serde(rename = "api_token_encoded", default, skip_serializing)]
    legacy_api_token_encoded: Option<String>,
    /// Present only to read config files saved by a version of this app
    /// before `dark_mode: bool` became `theme_preference: ThemePreference`.
    /// `#[serde(skip_serializing)]` means this is never written back out --
    /// `ConfigManager::load_config` migrates it into an explicit
    /// `ThemePreference::Light`/`Dark` (never `System`, since a file that
    /// had `dark_mode` at all always reflected an explicit choice, not "no
    /// preference") and it naturally disappears from `config.json` after
    /// the next save.
    #[serde(rename = "dark_mode", default, skip_serializing)]
    legacy_dark_mode: Option<bool>,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            weather_provider: WeatherApiProvider::OpenWeather,
            location: LocationConfig::default(),
            theme_preference: ThemePreference::default(),
            use_fahrenheit: false,
            launch_at_login: false,
            refresh_interval_secs: None,
            legacy_api_token_encoded: None,
            legacy_dark_mode: None,
        }
    }
}

impl AppConfig {
    /// Stores the API token in the OS's secure credential store (macOS
    /// Keychain, Windows Credential Manager, Linux Secret Service).
    ///
    /// # Arguments
    ///
    /// * `token` - The API token to set.
    pub fn set_api_token(&mut self, token: &str) -> Result<(), String> {
        keyring_entry()?
            .set_password(token)
            .map_err(|e| format!("Failed to store API token securely: {e}"))
    }

    /// Reads the API token back from the OS's secure credential store.
    /// No token having been set yet is not an error -- it returns an empty
    /// string, same as an unset field would have before.
    ///
    /// # Returns
    ///
    /// A `Result` containing the API token `String` (empty if unset) on
    /// success, or an error `String` if the credential store itself
    /// couldn't be accessed (e.g. a locked keychain, no Secret Service
    /// running).
    pub fn get_api_token(&self) -> Result<String, String> {
        match keyring_entry()?.get_password() {
            Ok(token) => Ok(token),
            Err(keyring::Error::NoEntry) => Ok(String::new()),
            Err(e) => Err(format!("Failed to read API token: {e}")),
        }
    }

    /// Removes the API token from the OS's secure credential store
    /// entirely, rather than overwriting it with an empty string --
    /// deleting the credential itself is what `examples/clear_credentials.rs`
    /// needs, and matches what a user uninstalling the app or switching
    /// machines would actually want. Not currently exposed anywhere in the
    /// app's own UI (Preferences only ever sets a new token); a missing
    /// entry is not an error, since that's already the desired end state.
    // The binary crate's own `mod config` (src/main.rs) never calls this --
    // only `examples/clear_credentials.rs` does, which links against the
    // library crate's copy, a separate compilation -- hence the `allow`
    // (same reasoning as `ConfigManager::for_path` above).
    #[allow(dead_code)]
    pub fn delete_api_token(&self) -> Result<(), String> {
        match keyring_entry()?.delete_credential() {
            Ok(()) => Ok(()),
            Err(keyring::Error::NoEntry) => Ok(()),
            Err(e) => Err(format!("Failed to delete API token: {e}")),
        }
    }

    /// Applies the `launch_at_login` preference to the OS.
    pub fn update_auto_launch(&self) -> Result<(), String> {
        let current_exe = std::env::current_exe().map_err(|e| e.to_string())?;

        let auto = auto_launch::AutoLaunchBuilder::new()
            .set_app_name("open-weather-wizard")
            .set_app_path(&current_exe.to_string_lossy())
            .set_macos_launch_mode(auto_launch::MacOSLaunchMode::LaunchAgent)
            .build()
            .map_err(|e| e.to_string())?;

        if self.launch_at_login {
            auto.enable().map_err(|e| e.to_string())?;
        } else {
            auto.disable().map_err(|e| e.to_string())?;
        }

        Ok(())
    }
}

/// The single `keyring::Entry` this app ever uses for the OpenWeatherMap API
/// token, constructed once per process and reused for every read/write.
///
/// Constructing an `Entry` doesn't itself perform any OS I/O (that happens
/// in `get_password`/`set_password`), so this isn't primarily a performance
/// optimization -- it matters for testability. The crate's mock credential
/// backend (used by this module's tests) gives every *fresh*
/// `Entry::new(...)` call its own unrelated in-memory credential, since
/// mocks intentionally don't persist across sessions; a fresh `Entry` per
/// call would make a test's own set-then-get round-trip invisible to
/// itself. Caching one `Entry` for the process's lifetime sidesteps that
/// without changing anything about how real platform backends behave (they
/// persist by service+user at the OS level, independent of the `Entry`
/// object).
static API_TOKEN_ENTRY: LazyLock<Result<keyring::Entry, String>> = LazyLock::new(|| {
    keyring::Entry::new(KEYRING_SERVICE, KEYRING_API_TOKEN_KEY)
        .map_err(|e| format!("Failed to access the OS secure credential store: {e}"))
});

fn keyring_entry() -> Result<&'static keyring::Entry, String> {
    API_TOKEN_ENTRY.as_ref().map_err(Clone::clone)
}

/// Manages the loading and saving of the application's configuration.
///
/// This struct handles the logic for finding the correct configuration file path
/// and performing file I/O operations for serialization and deserialization.
pub struct ConfigManager {
    config_path: PathBuf,
}

impl ConfigManager {
    /// Creates a new `ConfigManager`.
    ///
    /// This function also ensures that the configuration directory exists.
    ///
    /// # Returns
    ///
    /// A `Result` containing the `ConfigManager` on success, or a boxed error on failure.
    pub fn new() -> Result<Self, Box<dyn std::error::Error>> {
        let config_dir = dirs::config_dir()
            .ok_or("Could not determine config directory")?
            .join("open-weather-wizard");

        // Create config directory if it doesn't exist
        fs::create_dir_all(&config_dir)?;

        let config_path = config_dir.join("config.json");

        Ok(Self { config_path })
    }

    /// Whether a config file already exists on disk -- the signal `app::boot`
    /// uses to distinguish a fresh install (no file yet, so nothing has ever
    /// been configured) from a returning user, since `load_config` itself
    /// can't tell the difference (a missing or unparsable file both silently
    /// fall back to `AppConfig::default()`). Must be checked *before*
    /// calling `load_config`, which doesn't create the file itself --  only
    /// `save_config` does.
    pub fn config_exists(&self) -> bool {
        self.config_path.exists()
    }

    /// Points a `ConfigManager` at an arbitrary file, bypassing the real OS
    /// config directory -- so tests can exercise `load_config`/`save_config`
    /// (in particular the legacy-token migration, which needs real file
    /// I/O) against a throwaway file instead of the user's actual config.
    // The binary crate's own test build (src/main.rs declares its own
    // `mod config;`, a separate compilation from the library's) has no
    // test that calls this -- only src/lib.rs's `test_legacy_token_migration`
    // does -- hence the `allow`.
    #[cfg(test)]
    #[allow(dead_code)]
    pub(crate) fn for_path(config_path: PathBuf) -> Self {
        Self { config_path }
    }

    /// Loads the application's configuration from a file.
    ///
    /// If the configuration file does not exist, cannot be read, or contains invalid
    /// JSON, this function logs a warning and returns the default configuration.
    ///
    /// # Returns
    ///
    /// The loaded `AppConfig`.
    pub fn load_config(&self) -> AppConfig {
        match fs::read_to_string(&self.config_path) {
            Ok(contents) => match serde_json::from_str::<AppConfig>(&contents) {
                Ok(mut config) => {
                    log::info!("Loaded configuration from {:?}", self.config_path);
                    self.migrate_legacy_token(&mut config);
                    migrate_legacy_dark_mode(&mut config);
                    config
                }
                Err(e) => {
                    log::warn!("Failed to parse config file, using defaults: {}", e);
                    AppConfig::default()
                }
            },
            Err(_) => {
                log::info!("Config file not found, using defaults");
                AppConfig::default()
            }
        }
    }

    /// Saves the application's configuration to a file.
    ///
    /// # Arguments
    ///
    /// * `config` - A reference to the `AppConfig` to save.
    ///
    /// # Returns
    ///
    /// A `Result` which is `Ok` on success or a boxed error on failure.
    pub fn save_config(&self, config: &AppConfig) -> Result<(), Box<dyn std::error::Error>> {
        let json = serde_json::to_string_pretty(config)?;
        fs::write(&self.config_path, json)?;
        log::info!("Saved configuration to {:?}", self.config_path);
        Ok(())
    }

    /// One-time migration for config files saved by older versions of this
    /// app: if a base64-"encoded" token is present, decode it into the OS
    /// keychain and immediately re-save the config so the plaintext-ish
    /// token is gone from disk on the very next write, not just in memory.
    /// A no-op (and cheap: one `Option` check) for every config file saved
    /// since this migration was added.
    fn migrate_legacy_token(&self, config: &mut AppConfig) {
        let Some(encoded) = config.legacy_api_token_encoded.take() else {
            return;
        };

        let decoded = STANDARD
            .decode(&encoded)
            .map_err(|e| format!("invalid base64: {e}"))
            .and_then(|bytes| String::from_utf8(bytes).map_err(|e| format!("invalid UTF-8: {e}")));

        match decoded {
            Ok(token) if !token.is_empty() => match config.set_api_token(&token) {
                Ok(()) => {
                    log::info!("Migrated API token from config file into the OS keychain");
                    if let Err(e) = self.save_config(config) {
                        log::warn!(
                            "Migrated token to keychain but failed to rewrite config file (it will be retried next launch): {e}"
                        );
                    }
                }
                Err(e) => log::warn!(
                    "Found a legacy API token in config file but failed to migrate it into the OS keychain: {e}"
                ),
            },
            Ok(_) => {} // empty token, nothing to migrate
            Err(e) => log::warn!(
                "Found an api_token_encoded field in config file but couldn't decode it, ignoring: {e}"
            ),
        }
    }
}

/// One-time migration for config files saved before `dark_mode: bool`
/// became `theme_preference: ThemePreference` -- maps the old explicit
/// boolean onto `Light`/`Dark`. Unlike `migrate_legacy_token`, this doesn't
/// force an immediate re-save: there's no sensitive data to scrub from
/// disk, so the harmless `dark_mode` key just lingers in the file until the
/// next natural Save in Preferences rewrites it with `theme_preference`
/// instead. A no-op (one `Option` check) for every config file saved since
/// this migration was added.
fn migrate_legacy_dark_mode(config: &mut AppConfig) {
    if let Some(dark_mode) = config.legacy_dark_mode.take() {
        config.theme_preference = if dark_mode {
            ThemePreference::Dark
        } else {
            ThemePreference::Light
        };
    }
}
