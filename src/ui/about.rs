//! # About Dialog Module
//!
//! This module provides a function to create and display the application's "About"
//! dialog. The dialog presents essential information about the application, such
//! as its name, version, author, and license. It leverages the `gtk::AboutDialog`
//! widget, which is designed for this specific purpose.
//!
//! The information displayed in the dialog is retrieved from the `Cargo.toml`
//! file at compile time using the `env!` macro. This ensures that the details

use gtk::prelude::*;
use gtk::{AboutDialog, ApplicationWindow};

/// Creates and displays the "About" dialog.
///
/// This function constructs a `gtk::AboutDialog` and populates it with metadata
/// from the `Cargo.toml` file, such as the application's name, version, authors,
/// website, and license. The dialog is modal and transient for the parent window.
///
/// # Arguments
///
/// * `parent` - The parent `ApplicationWindow` to which this modal dialog is transient.
pub fn show_about_dialog(parent: &ApplicationWindow) {
    // Parse authors (Cargo's CARGO_PKG_AUTHORS is colon-separated)
    let authors_env = env!("CARGO_PKG_AUTHORS");
    let authors: Vec<String> = authors_env.split(':').map(|s| s.to_string()).collect();

    let about_dialog = AboutDialog::builder()
        .transient_for(parent)
        .modal(true)
        .program_name("Meteo Wizard")
        .version(env!("CARGO_PKG_VERSION"))
        .comments("Meteo Wizard is a lightweight desktop weather app written in Rust and GTK 4. It provides current weather, humidity, and temperature with animated iconography bundled with the application. Report issues or feature requests on the project's GitHub repository.")
        .copyright(format!("© 2024 {}", authors.first().unwrap_or(&"".to_string())).as_str())
        .license_type(gtk::License::MitX11)
        .website(env!("CARGO_PKG_HOMEPAGE"))
        .authors(authors)
        .build();

    // Try to load the bundled icon and set it in the About dialog if available.
    if let Ok(pixbuf) = crate::ui::build_elements::load_embedded_pixbuf("icon/icon.png", 128) {
        // Convert Pixbuf to a Gdk Texture (which implements Paintable) so the AboutDialog
        // can accept it as a logo.
        let texture = gtk::gdk::Texture::for_pixbuf(&pixbuf);
        about_dialog.set_logo(Some(&texture));
    }

    about_dialog.present();
}
