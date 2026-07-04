//! # IP-Based Location Detection
//!
//! Best-effort convenience for prefilling the "Home" location during
//! first-run setup (see issue #38, and the broader geolocation tracking
//! issue #5): resolves an approximate city/state/country from the caller's
//! public IP address via [freeipapi.com](https://freeipapi.com/)'s free,
//! keyless HTTPS endpoint.
//!
//! No cross-platform OS geolocation API (CoreLocation on macOS, the WinRT
//! `Geolocator` on Windows, GeoClue on Linux) is used here -- bridging three
//! separate platform APIs for the same one-time convenience prefill would be
//! a lot of dependency weight, and none of them work headlessly in CI
//! anyway. IP geolocation is ISP-routing-based, not GPS-accurate, so this is
//! explicitly a starting point the user can (and often should) correct, not
//! a precise location -- callers must treat a failure as "leave the
//! existing fields alone," never as an error worth surfacing loudly.

use crate::config::LocationConfig;
use serde::Deserialize;

const IP_GEOLOCATION_API: &str = "https://freeipapi.com/api/json/";

/// The subset of freeipapi.com's response this app actually uses --
/// `regionCode` is the two-letter code (e.g. `"IL"`), matching the format
/// `LocationConfig.state`/the Google Weather provider's own
/// `US_STATE_ABBREVIATIONS` table already assume.
#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
struct IpGeolocationResponse {
    city_name: String,
    region_code: String,
    country_code: String,
}

/// Detects an approximate `LocationConfig` from the caller's public IP
/// address. Returns a plain `String` error (network/parse failure) rather
/// than a richer error type -- the only caller (`app.rs`) just logs it and
/// leaves whatever location fields were already there untouched, since this
/// is a convenience, not a fetch the UI needs to react to differently by
/// failure mode.
pub async fn detect_location() -> Result<LocationConfig, String> {
    let response = reqwest::get(IP_GEOLOCATION_API)
        .await
        .map_err(|e| format!("Request failed: {e}"))?;

    let parsed = response
        .json::<IpGeolocationResponse>()
        .await
        .map_err(|e| format!("Failed to parse response: {e}"))?;

    Ok(LocationConfig {
        city: parsed.city_name,
        state: parsed.region_code,
        country: parsed.country_code,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ip_geolocation_response_deserializes_and_maps() {
        let json = r#"{
            "ipVersion": 4,
            "ipAddress": "98.227.181.31",
            "latitude": 41.885,
            "longitude": -87.7845,
            "countryName": "United States",
            "countryCode": "US",
            "cityName": "Oak Park",
            "regionName": "Illinois",
            "regionCode": "IL"
        }"#;
        let parsed: IpGeolocationResponse = serde_json::from_str(json).unwrap();
        let location = LocationConfig {
            city: parsed.city_name,
            state: parsed.region_code,
            country: parsed.country_code,
        };
        assert_eq!(location.city, "Oak Park");
        assert_eq!(location.state, "IL");
        assert_eq!(location.country, "US");
    }
}
