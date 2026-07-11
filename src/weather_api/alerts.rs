use serde::{Deserialize, Serialize};

/// Severity of a weather alert.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum AlertSeverity {
    UnknownSeverity,
    Minor,
    Moderate,
    Severe,
    Extreme,
}

impl Default for AlertSeverity {
    fn default() -> Self {
        Self::UnknownSeverity
    }
}

/// A weather alert (e.g., severe thunderstorm warning) for a specific location.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WeatherAlert {
    pub id: String,
    pub title: String,
    pub description: String,
    pub event_type: String,
    pub severity: AlertSeverity,
    /// Unix timestamp for when the alert becomes active.
    pub start_time: i64,
    /// Unix timestamp for when the alert expires.
    pub end_time: i64,
    pub urgency: String,
    pub certainty: String,
    pub instruction: Vec<String>,
}
