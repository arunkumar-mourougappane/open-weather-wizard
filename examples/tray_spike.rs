//! Tray-icon/iced integration spike for issue #56 (throwaway prototype, see
//! its own doc comment -- excluded from the published crate, see
//! `Cargo.toml`).
//!
//! Answers the issue's central open question: can a persistent system tray
//! icon coexist with this app's actual `iced::daemon` (multi-window,
//! `tokio` executor), given `iced` doesn't expose the `winit` event loop it
//! wraps internally, and `tray-icon`'s own docs warn the icon should be
//! created only once that event loop is already running?
//!
//! The `tray` crate (nobane/tray-rs) turns out to sidestep that requirement
//! entirely: it exposes a **polling** API (`TrayIconEvent::receiver()`)
//! instead of requiring a `winit::event_loop::EventLoopProxy` push
//! integration. That means the tray icon doesn't need access to iced's
//! internal event loop at all -- it only needs *some* iced `Subscription`
//! polling its channel periodically, which this spike does on the same
//! 33ms cadence `src/app.rs`'s `AnimationTick` already uses (so a real
//! integration adds no new timer, just piggybacks on an existing one).
//!
//! Findings:
//! - Builds and runs correctly against this project's real `iced = "0.14"`
//!   (the crate's own bundled example only declares `iced = "0.13"; this
//!   spike is what actually proves 0.14 compatibility).
//! - Creating the `TrayIcon` synchronously in `boot()` (this project's
//!   equivalent of the crate's own example's `App::new()`) works fine on
//!   macOS -- `NSStatusItem` doesn't need the run loop already spinning,
//!   just an `NSApplication` instance, which exists by the time `boot()`
//!   runs.
//! - Windows: not tested here (no Windows machine in this environment) --
//!   the crate's own `windows.rs` uses `Shell_NotifyIcon`/`windows-sys`
//!   directly, no `winit`/GTK dependency, so it should work the same way
//!   this spike proves for macOS, but treat that as unverified until
//!   someone actually runs it there.
//! - Linux: the crate implements the system tray via **raw X11**
//!   (`x11rb`), not GTK -- resolves this issue's headline GTK-vs-iced
//!   conflict without needing this project's own from-scratch
//!   `StatusNotifierItem`/D-Bus implementation (Option C). Not tested here
//!   either, and worth flagging explicitly: X11 tray icons need an X11 (or
//!   XWayland) session -- a pure-Wayland compositor with no XWayland has no
//!   system tray for this crate to draw into at all.
//!
//! Run with: `cargo run --example tray_spike`

use std::time::Duration;

use iced::widget::{button, center, column, text};
use iced::window;
use iced::{Element, Size, Subscription, Task};

use tray::{Icon, MouseButton, MouseButtonState, TrayIcon, TrayIconBuilder, TrayIconEvent};

pub fn main() -> iced::Result {
    iced::daemon(App::boot, App::update, App::view)
        .title(App::title)
        .subscription(App::subscription)
        .run()
}

struct App {
    _tray_icon: TrayIcon,
    click_count: u32,
}

#[derive(Debug, Clone)]
enum Message {
    TrayPoll,
    Increment,
}

impl App {
    fn boot() -> (Self, Task<Message>) {
        let icon_path = concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/assets/icon/iconset/icon-32.png"
        );
        let icon = load_icon(std::path::Path::new(icon_path));

        let tray_icon = TrayIconBuilder::new()
            .with_tooltip("Weather Wizard -- 72\u{b0}F, clear sky (spike)")
            .with_icon(icon)
            .build()
            .expect("Failed to create tray icon");

        let (_window, open_task) = window::open(window::Settings {
            size: Size::new(360.0, 200.0),
            ..window::Settings::default()
        });

        (
            Self {
                _tray_icon: tray_icon,
                click_count: 0,
            },
            open_task.discard(),
        )
    }

    fn title(&self, _window_id: window::Id) -> String {
        "Tray Spike".to_string()
    }

    fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::TrayPoll => {
                // Drains whatever the platform's tray implementation
                // enqueued since the last poll -- see this file's top-level
                // doc comment for why polling sidesteps needing iced's
                // internal winit event loop at all.
                while let Ok(event) = TrayIconEvent::receiver().try_recv() {
                    if let TrayIconEvent::Click {
                        button: MouseButton::Left,
                        button_state: MouseButtonState::Up,
                        ..
                    } = event
                    {
                        self.click_count += 1;
                    }
                }
                Task::none()
            }
            Message::Increment => {
                self.click_count += 1;
                Task::none()
            }
        }
    }

    fn view(&self, _window_id: window::Id) -> Element<'_, Message> {
        center(
            column![
                text("Tray icon spike").size(20),
                text("Click the menu bar icon, or the button below.").size(12),
                text(format!("Clicks registered: {}", self.click_count)).size(18),
                button("Increment").on_press(Message::Increment),
            ]
            .spacing(10)
            .padding(20),
        )
        .into()
    }

    fn subscription(&self) -> Subscription<Message> {
        // Matches `src/app.rs`'s existing `ANIMATION_TICK_INTERVAL` cadence
        // -- a real integration would piggyback on that subscription
        // rather than add a second one.
        iced::time::every(Duration::from_millis(33)).map(|_| Message::TrayPoll)
    }
}

fn load_icon(path: &std::path::Path) -> Icon {
    let image = image::open(path)
        .expect("failed to open icon path")
        .into_rgba8();
    let (width, height) = image.dimensions();
    Icon::from_rgba(image.into_raw(), width, height).expect("failed to build tray icon")
}
