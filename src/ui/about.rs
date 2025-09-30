
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
    let about_dialog = AboutDialog::builder()
        .transient_for(parent)
        .modal(true)
        .program_name("Meteo Wizard")
        .version(env!("CARGO_PKG_VERSION"))
        .copyright("© 2024 Amouroug")
        .license_type(gtk::License::MitX11)
        .website(env!("CARGO_PKG_HOMEPAGE"))
        .authors(vec![env!("CARGO_PKG_AUTHORS").to_string()])
        .build();

    about_dialog.present();
}
