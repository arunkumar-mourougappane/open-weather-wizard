//! # Location Detection
//!
//! Best-effort location detection for prefilling the "Home" location during
//! first-run setup (see issue #38, and the broader geolocation tracking
//! issue #5). Two-tier, in order:
//!
//! 1. **OS-native location** (`os_location`) -- CoreLocation on macOS, the
//!    WinRT `Geolocator` on Windows, GeoClue2 on Linux -- reverse-geocoded
//!    (`reverse_geocode`, via OpenStreetMap's Nominatim) into a city/state/
//!    country. This is what real GPS/Wi-Fi/cell-tower-based positioning
//!    looks like; when available and permitted, it's far more accurate than
//!    IP geolocation.
//! 2. **IP-based geolocation** (`ip_location`, via `ipwho.is`), used only
//!    when native location is unavailable, denied, or fails. IP geolocation
//!    resolves to wherever the ISP's routing infrastructure is registered,
//!    not the physical location -- confirmed inaccurate by a real-world test
//!    (three different free providers all missed by tens of miles) -- so
//!    it's a last-resort fallback, not the primary path.
//!
//! Every failure mode -- no native API on this platform, permission denied,
//! no result, a network error -- degrades to the next tier rather than
//! surfacing as an error, since this whole feature is a convenience prefill
//! the user can always override by typing. Only if *every* tier fails does
//! `detect_location` return an `Err` at all.

use crate::config::LocationConfig;

/// Detects an approximate "Home" location: OS-native positioning
/// (reverse-geocoded) if available, otherwise an IP-based lookup.
pub async fn detect_location() -> Result<LocationConfig, String> {
    if let Some((lat, lon)) = os_location::coordinates().await {
        match reverse_geocode::reverse_geocode(lat, lon).await {
            Ok(location) => return Ok(location),
            Err(e) => log::warn!("Reverse geocoding failed, falling back to IP lookup: {e}"),
        }
    }
    ip_location::detect().await
}

/// IP-based geolocation -- the fallback tier. `ipwho.is` (free, keyless,
/// HTTPS) was chosen after comparing it against two other free providers on
/// a real connection; all three were inaccurate to varying degrees (an
/// inherent limitation of IP geolocation, not something a provider swap
/// fixes outright), but this one was closest.
mod ip_location {
    use super::LocationConfig;
    use serde::Deserialize;

    const IP_GEOLOCATION_API: &str = "https://ipwho.is/";

    #[derive(Deserialize, Debug)]
    pub(super) struct IpGeolocationResponse {
        pub(super) success: bool,
        pub(super) city: String,
        pub(super) region_code: String,
        pub(super) country_code: String,
    }

    pub async fn detect() -> Result<LocationConfig, String> {
        let response = reqwest::get(IP_GEOLOCATION_API)
            .await
            .map_err(|e| format!("Request failed: {e}"))?;

        let parsed = response
            .json::<IpGeolocationResponse>()
            .await
            .map_err(|e| format!("Failed to parse response: {e}"))?;

        if !parsed.success {
            return Err("ipwho.is reported a lookup failure".to_string());
        }

        Ok(LocationConfig {
            city: parsed.city,
            state: parsed.region_code,
            country: parsed.country_code,
        })
    }
}

/// Turns OS-native coordinates into a city/state/country, via OpenStreetMap's
/// free, keyless Nominatim reverse-geocoding API -- shared by all three
/// platforms in `os_location`, since CoreLocation/WinRT/GeoClue2 all return
/// only latitude/longitude, never a place name.
mod reverse_geocode {
    use super::LocationConfig;
    use serde::Deserialize;

    const REVERSE_GEOCODE_API: &str = "https://nominatim.openstreetmap.org/reverse";
    /// Nominatim's usage policy requires a descriptive User-Agent identifying
    /// the calling application -- an unidentified/generic one risks being
    /// rate-limited or blocked.
    const USER_AGENT: &str = concat!(
        "open-weather-wizard/",
        env!("CARGO_PKG_VERSION"),
        " (github.com/arunkumar-mourougappane/open-weather-wizard)"
    );

    /// Deliberately country-independent: `state` and `country_code` are
    /// stored exactly as Nominatim returns them for *this* address, not
    /// normalized against any single country's convention (e.g. no US
    /// state-abbreviation lookup) -- this runs for any location on Earth.
    #[derive(Deserialize, Debug, Default)]
    pub(super) struct Address {
        pub(super) city: Option<String>,
        pub(super) town: Option<String>,
        pub(super) village: Option<String>,
        #[serde(default)]
        pub(super) state: String,
        #[serde(default)]
        pub(super) country_code: String,
    }

    #[derive(Deserialize, Debug, Default)]
    pub(super) struct ReverseGeocodeResponse {
        #[serde(default)]
        pub(super) address: Address,
    }

    pub async fn reverse_geocode(lat: f64, lon: f64) -> Result<LocationConfig, String> {
        let client = reqwest::Client::new();
        let response = client
            .get(REVERSE_GEOCODE_API)
            .query(&[
                ("lat", lat.to_string()),
                ("lon", lon.to_string()),
                ("format", "jsonv2".to_string()),
                ("addressdetails", "1".to_string()),
            ])
            .header(reqwest::header::USER_AGENT, USER_AGENT)
            .send()
            .await
            .map_err(|e| format!("Request failed: {e}"))?;

        let parsed = response
            .json::<ReverseGeocodeResponse>()
            .await
            .map_err(|e| format!("Failed to parse response: {e}"))?;

        // Nominatim varies which of these three keys it returns depending on
        // settlement size -- try each in turn rather than picking just one.
        let city = parsed
            .address
            .city
            .or(parsed.address.town)
            .or(parsed.address.village)
            .ok_or_else(|| "No city/town/village in reverse-geocode response".to_string())?;

        Ok(LocationConfig {
            city,
            state: parsed.address.state,
            // Nominatim returns a lowercase ISO code (e.g. "us"); this app's
            // convention elsewhere (defaults, Google Weather's country param)
            // is uppercase.
            country: parsed.address.country_code.to_uppercase(),
        })
    }
}

/// OS-native positioning coordinates, dispatched per-platform. `None` on
/// any failure/denial/unsupported-platform -- this layer never returns an
/// `Err`, since a real error is only meaningful once the IP-based fallback
/// (the last tier) also fails.
mod os_location {
    pub async fn coordinates() -> Option<(f64, f64)> {
        #[cfg(target_os = "macos")]
        return macos::coordinates().await;
        #[cfg(target_os = "windows")]
        return windows::coordinates().await;
        #[cfg(target_os = "linux")]
        return linux::coordinates().await;
        #[cfg(not(any(target_os = "macos", target_os = "windows", target_os = "linux")))]
        None
    }

    /// CoreLocation, via the low-level `objc2-core-location` bindings (no
    /// ergonomic wrapper crate exists). Requires the process to be a signed
    /// `.app` bundle with an `NSLocationWhenInUseUsageDescription` Info.plist
    /// key to ever show the permission prompt (see `packaging/macos/
    /// Info.plist` and `Cargo.toml`'s `package.metadata.packager.macos`) --
    /// a plain `cargo run` dev binary gets an immediate `denied`/
    /// `notDetermined`-with-no-prompt and falls through to IP, which is
    /// expected and was confirmed by actually running this code during
    /// development.
    #[cfg(target_os = "macos")]
    mod macos {
        use std::cell::RefCell;
        use std::sync::mpsc;
        use std::time::{Duration, Instant};

        use objc2::rc::Retained;
        use objc2::runtime::ProtocolObject;
        use objc2::{AnyThread, DefinedClass, define_class, msg_send};
        use objc2_core_foundation::CFRunLoop;
        use objc2_core_location::{
            CLAuthorizationStatus, CLLocation, CLLocationManager, CLLocationManagerDelegate,
        };
        use objc2_foundation::{NSArray, NSError, NSObject, NSObjectProtocol};

        type CoordinatesSender = mpsc::Sender<Option<(f64, f64)>>;

        struct Ivars {
            tx: RefCell<Option<CoordinatesSender>>,
        }

        define_class!(
            #[unsafe(super(NSObject))]
            #[ivars = Ivars]
            struct LocationDelegate;

            impl LocationDelegate {}

            unsafe impl NSObjectProtocol for LocationDelegate {}

            unsafe impl CLLocationManagerDelegate for LocationDelegate {
                #[unsafe(method(locationManager:didUpdateLocations:))]
                fn location_manager_did_update_locations(
                    &self,
                    _manager: &CLLocationManager,
                    locations: &NSArray<CLLocation>,
                ) {
                    if let Some(tx) = self.ivars().tx.borrow_mut().take() {
                        let coords = locations.lastObject().map(|loc| {
                            let c = unsafe { loc.coordinate() };
                            (c.latitude, c.longitude)
                        });
                        let _ = tx.send(coords);
                    }
                }

                #[unsafe(method(locationManager:didFailWithError:))]
                fn location_manager_did_fail_with_error(
                    &self,
                    _manager: &CLLocationManager,
                    _error: &NSError,
                ) {
                    if let Some(tx) = self.ivars().tx.borrow_mut().take() {
                        let _ = tx.send(None);
                    }
                }
            }
        );

        impl LocationDelegate {
            fn new(tx: CoordinatesSender) -> Retained<Self> {
                let this = Self::alloc().set_ivars(Ivars {
                    tx: RefCell::new(Some(tx)),
                });
                unsafe { msg_send![super(this), init] }
            }
        }

        pub async fn coordinates() -> Option<(f64, f64)> {
            // CLLocationManager's delegate callbacks require an active run
            // loop on the thread that owns it, which tokio's async
            // executor doesn't provide -- run the whole interaction on a
            // dedicated blocking thread that pumps its own short-lived
            // CFRunLoop instead.
            tokio::task::spawn_blocking(coordinates_blocking)
                .await
                .unwrap_or(None)
        }

        fn coordinates_blocking() -> Option<(f64, f64)> {
            let manager = unsafe { CLLocationManager::new() };
            let status = unsafe { manager.authorizationStatus() };

            if status == CLAuthorizationStatus::Denied
                || status == CLAuthorizationStatus::Restricted
            {
                return None;
            }

            let (tx, rx) = mpsc::channel();
            let delegate = LocationDelegate::new(tx);
            unsafe { manager.setDelegate(Some(ProtocolObject::from_ref(&*delegate))) };

            if status == CLAuthorizationStatus::NotDetermined {
                unsafe { manager.requestWhenInUseAuthorization() };
            }

            unsafe { manager.requestLocation() };

            let deadline = Instant::now() + Duration::from_secs(5);
            loop {
                if let Ok(result) = rx.try_recv() {
                    return result;
                }
                if Instant::now() >= deadline {
                    return None;
                }
                CFRunLoop::run_in_mode(
                    unsafe { objc2_core_foundation::kCFRunLoopDefaultMode },
                    0.1,
                    false,
                );
            }
        }
    }

    /// The WinRT `Geolocator` API, via the `windows` crate. `RequestAccessAsync`
    /// shows the system permission prompt; `GetGeopositionAsync` then performs
    /// a one-shot fetch. Any denial or `windows::core::Error` maps to `None`.
    #[cfg(target_os = "windows")]
    mod windows {
        use windows::Devices::Geolocation::{GeolocationAccessStatus, Geolocator};

        pub async fn coordinates() -> Option<(f64, f64)> {
            let access = Geolocator::RequestAccessAsync().ok()?.await.ok()?;
            if access != GeolocationAccessStatus::Allowed {
                return None;
            }

            let geolocator = Geolocator::new().ok()?;
            let position = geolocator.GetGeopositionAsync().ok()?.await.ok()?;
            let coordinate = position.Coordinate().ok()?;
            let point = coordinate.Point().ok()?;
            let basic = point.Position().ok()?;

            Some((basic.Latitude, basic.Longitude))
        }
    }

    /// GeoClue2 over D-Bus (`org.freedesktop.GeoClue2`), via `zbus`. Requires
    /// a running `geoclue2` daemon (absent on plenty of minimal/non-GNOME
    /// distros) and registers with the `DesktopId` matching this app's
    /// `open-weather-wizard.desktop` file, per GeoClue2's per-app permission
    /// model. No session bus, no daemon, or denied permission all map to
    /// `None`.
    #[cfg(target_os = "linux")]
    mod linux {
        use futures_util::StreamExt;
        use std::time::Duration;
        use zbus::Connection;
        use zbus::proxy;
        use zbus::zvariant::OwnedObjectPath;

        #[proxy(
            interface = "org.freedesktop.GeoClue2.Manager",
            default_service = "org.freedesktop.GeoClue2",
            default_path = "/org/freedesktop/GeoClue2/Manager"
        )]
        trait Manager {
            fn get_client(&self) -> zbus::Result<OwnedObjectPath>;
        }

        #[proxy(
            interface = "org.freedesktop.GeoClue2.Client",
            default_service = "org.freedesktop.GeoClue2"
        )]
        trait Client {
            fn start(&self) -> zbus::Result<()>;
            fn stop(&self) -> zbus::Result<()>;

            #[zbus(property)]
            fn set_desktop_id(&self, id: &str) -> zbus::Result<()>;

            #[zbus(signal)]
            fn location_updated(
                &self,
                old: OwnedObjectPath,
                new: OwnedObjectPath,
            ) -> zbus::Result<()>;
        }

        #[proxy(
            interface = "org.freedesktop.GeoClue2.Location",
            default_service = "org.freedesktop.GeoClue2"
        )]
        trait Location {
            #[zbus(property)]
            fn latitude(&self) -> zbus::Result<f64>;
            #[zbus(property)]
            fn longitude(&self) -> zbus::Result<f64>;
        }

        pub async fn coordinates() -> Option<(f64, f64)> {
            let connection = Connection::session().await.ok()?;

            let manager = ManagerProxy::new(&connection).await.ok()?;
            let client_path = manager.get_client().await.ok()?;

            let client = ClientProxy::builder(&connection)
                .path(client_path)
                .ok()?
                .build()
                .await
                .ok()?;

            client.set_desktop_id("open-weather-wizard").await.ok()?;

            let mut updates = client.receive_location_updated().await.ok()?;
            client.start().await.ok()?;

            let signal = tokio::time::timeout(Duration::from_secs(5), updates.next())
                .await
                .ok()??;
            let args = signal.args().ok()?;
            let location_path = args.new.clone();

            let location = LocationProxy::builder(&connection)
                .path(location_path)
                .ok()?
                .build()
                .await
                .ok()?;

            let lat = location.latitude().await.ok()?;
            let lon = location.longitude().await.ok()?;

            let _ = client.stop().await;

            Some((lat, lon))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ip_geolocation_response_deserializes_and_maps() {
        let json = r#"{
            "ip": "98.227.181.31",
            "success": true,
            "city": "Peoria Heights",
            "region": "Illinois",
            "region_code": "IL",
            "country_code": "US"
        }"#;
        let parsed: ip_location::IpGeolocationResponse = serde_json::from_str(json).unwrap();
        assert!(parsed.success);
        assert_eq!(parsed.city, "Peoria Heights");
        assert_eq!(parsed.region_code, "IL");
        assert_eq!(parsed.country_code, "US");
    }

    #[test]
    fn test_reverse_geocode_response_deserializes_and_maps() {
        let json = r#"{
            "address": {
                "city": "Dunlap",
                "state": "Illinois",
                "country_code": "us"
            }
        }"#;
        let parsed: reverse_geocode::ReverseGeocodeResponse = serde_json::from_str(json).unwrap();
        assert_eq!(parsed.address.city.as_deref(), Some("Dunlap"));
        assert_eq!(parsed.address.state, "Illinois");
        assert_eq!(parsed.address.country_code, "us");
    }

    #[test]
    fn test_reverse_geocode_response_falls_back_to_town() {
        // Smaller settlements come back under `town` or `village` instead
        // of `city`.
        let json = r#"{
            "address": {
                "town": "Dunlap",
                "state": "Illinois",
                "country_code": "us"
            }
        }"#;
        let parsed: reverse_geocode::ReverseGeocodeResponse = serde_json::from_str(json).unwrap();
        assert!(parsed.address.city.is_none());
        assert_eq!(parsed.address.town.as_deref(), Some("Dunlap"));
    }
}
