mod ui;

use glib;
use gtk4::prelude::*; // Import necessary traits for GTK widgets
use gtk4::{Application, ApplicationWindow};
use gtk4::gio::MenuModel;
use gtk4::PopoverMenuBar; // For glib::ExitCode
use ui::build_elements::{build_button, build_main_menu, build_spinner, DEFAULT_WINDOW_WIDTH, DEFAULT_WINDOW_HEIGHT};

fn build_main_ui() -> Application {
    // Create a new GTK application
    let application = Application::builder()
        .application_id("com.example.FirstGtkApp") // Unique application ID
        .build();
    application.connect_activate(|app| {
        // Create a new application window
        let window = ApplicationWindow::builder()
            .application(app) // Associate the window with the application
            .title("Weahter Wizard") // Set the window title
            .default_width(DEFAULT_WINDOW_WIDTH)
            .default_height(DEFAULT_WINDOW_HEIGHT)
            .build();
        window.present();

        // Create root menu and add submenus
        let root_menu = build_main_menu();

        // Convert to MenuModel
        let menu_model: MenuModel = root_menu.into();

        // Create PopoverMenuBar
        let menubar = PopoverMenuBar::from_model(Some(&menu_model));

        // Add menubar to the window (e.g., within a Box)
        let vbox = gtk4::Box::builder()
            .orientation(gtk4::Orientation::Vertical)
            .build();
        vbox.append(&menubar);
        // Add other widgets to vbox as needed

        // Create a button
        let button = build_button("Click Me".to_string());
        // Add the button to the window
        vbox.append(&button);

        // Create and add a spinner
        let spinner = build_spinner();
        vbox.append(&spinner);
        spinner.start(); // Start the spinner animation
        spinner.set_visible(true); // Make the spinner visible
        window.set_child(Some(&vbox));
        // Present the window to the user
        window.present();
    });
    application
}

fn main() -> glib::ExitCode {
    let application: Application = build_main_ui();
    // Connect the "activate" signal to a closure that builds the UI

    // Run the application
    application.run()
}
