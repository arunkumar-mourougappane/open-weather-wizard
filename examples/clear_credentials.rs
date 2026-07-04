//!
//! Deletes this app's stored API token from the OS's secure credential store
//! (macOS Keychain / Windows Credential Manager / Linux Secret Service) --
//! useful for resetting the app back to a fresh-install state (see the
//! first-run setup flow, issue #38) or clearing credentials before
//! uninstalling.
//!
//! Unlike the other examples in this crate, this one touches the *real* OS
//! credential store, not a mock or a remote API -- running it has an actual,
//! irreversible effect (whichever provider is currently configured will need
//! its token re-entered in Preferences afterward). It asks for an explicit
//! "yes" confirmation on stdin before deleting anything.
//!
//! ```sh
//! cargo run --example clear_credentials
//! ```
use open_weather_wizard::config::AppConfig;
use std::io::{self, Write};

fn main() {
    println!("Clear Stored Credentials");
    println!("=========================\n");
    println!(
        "This will permanently delete the Weather Wizard API token stored in \
         this OS's secure credential store (macOS Keychain / Windows Credential \
         Manager / Linux Secret Service). You'll need to re-enter it in \
         Preferences afterward.\n"
    );

    print!("Type \"yes\" to continue: ");
    if io::stdout().flush().is_err() {
        // Nothing meaningful to do if stdout itself is broken -- fall
        // through and let the read_line below fail/abort the same way.
    }

    let mut confirmation = String::new();
    if io::stdin().read_line(&mut confirmation).is_err() || confirmation.trim() != "yes" {
        println!("Aborted -- no changes made.");
        return;
    }

    // Deleting the token doesn't depend on any other config field --
    // AppConfig::default() is just a handle to call the method through.
    let config = AppConfig::default();
    match config.delete_api_token() {
        Ok(()) => println!("\n✅ Stored API token deleted."),
        Err(e) => println!("\n⚠️  Failed to delete API token: {e}"),
    }
}
