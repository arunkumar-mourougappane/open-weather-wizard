use chrono::NaiveDateTime;
use core::fmt;
use log::error;
use std::{collections::HashMap, str::FromStr};

use chrono_tz::Tz;
use serde::Serialize;
use serde_json::{Map, Value};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum WeatherDataError {
    #[error("Cannot parse wether data")]
    ParseError,
    #[error("Cannot parse wether data point")]
    DatapointParseError,
    #[error("Cannot parse wether data point")]
    JsonSerializationError(#[from] serde_json::Error),
}

#[derive(Debug, Serialize)]
pub struct HourlyUnits {
    time: String,
    temperature_2m: String,
    relative_humidity_2m: String,
    apparent_temperature: String,
    precipitation_probability: String,
    precipitation: String,
    rain: String,
    showers: String,
    snowfall: String,
    weather_code: String,
    visibility: String,
}

impl HourlyUnits {
    const TIME: &'static str = "time";
    const TEMPERATURE_2M: &'static str = "temperature_2m";
    const RELATIVE_HUMIDITY_2M: &'static str = "relative_humidity_2m";
    const APPARENT_TEMPERATURE: &'static str = "apparent_temperature";
    const PRECIPITATION_PROBABILITY: &'static str = "precipitation_probability";
    const PRECIPITATION: &'static str = "precipitation";
    const RAIN: &'static str = "rain";
    const SHOWERS: &'static str = "showers";
    const SNOWFALL: &'static str = "snowfall";
    const WEATHER_CODE: &'static str = "weather_code";
    const VISIBILITY: &'static str = "visibility";

    #[allow(clippy::too_many_arguments)]
    pub fn new(
        time: String,
        temperature_2m: String,
        relative_humidity_2m: String,
        apparent_temperature: String,
        precipitation_probability: String,
        precipitation: String,
        rain: String,
        showers: String,
        snowfall: String,
        weather_code: String,
        visibility: String,
    ) -> Self {
        Self {
            time,
            temperature_2m,
            relative_humidity_2m,
            apparent_temperature,
            precipitation_probability,
            precipitation,
            rain,
            showers,
            snowfall,
            weather_code,
            visibility,
        }
    }
    pub fn parse_from(json_data: &Map<String, Value>) -> Result<HourlyUnits, WeatherDataError> {
        let timestamp_format = match json_data[Self::TIME].as_str() {
            Some(timestamp) => String::from(timestamp),
            None => return Err(WeatherDataError::ParseError),
        };

        let temperature_unit = match json_data[Self::TEMPERATURE_2M].as_str() {
            Some(temperature_unit) => String::from(temperature_unit),
            None => return Err(WeatherDataError::ParseError),
        };

        let relative_humidity_unit = match json_data[Self::RELATIVE_HUMIDITY_2M].as_str() {
            Some(relative_humidity_unit) => String::from(relative_humidity_unit),
            None => return Err(WeatherDataError::ParseError),
        };
        let apparent_temperature_unit = match json_data[Self::APPARENT_TEMPERATURE].as_str() {
            Some(apparent_temperature_unit) => String::from(apparent_temperature_unit),
            None => return Err(WeatherDataError::ParseError),
        };
        let precipitation_probability_unit = match json_data[Self::PRECIPITATION_PROBABILITY]
            .as_str()
        {
            Some(precipitation_probability_unit) => String::from(precipitation_probability_unit),
            None => return Err(WeatherDataError::ParseError),
        };
        let precipitation_unit = match json_data[Self::PRECIPITATION].as_str() {
            Some(precipitation_unit) => String::from(precipitation_unit),
            None => return Err(WeatherDataError::ParseError),
        };
        let rain_unit = match json_data[Self::RAIN].as_str() {
            Some(rain_unit) => String::from(rain_unit),
            None => return Err(WeatherDataError::ParseError),
        };
        let showers_unit = match json_data[Self::SHOWERS].as_str() {
            Some(showers_unit) => String::from(showers_unit),
            None => return Err(WeatherDataError::ParseError),
        };
        let snowfall_unit = match json_data[Self::SNOWFALL].as_str() {
            Some(snowfall_unit) => String::from(snowfall_unit),
            None => return Err(WeatherDataError::ParseError),
        };
        let weather_code_unit = match json_data[Self::WEATHER_CODE].as_str() {
            Some(weather_code_unit) => String::from(weather_code_unit),
            None => return Err(WeatherDataError::ParseError),
        };
        let visibility_unit = match json_data[Self::VISIBILITY].as_str() {
            Some(visibility_unit) => String::from(visibility_unit),
            None => return Err(WeatherDataError::ParseError),
        };

        Ok(HourlyUnits::new(
            timestamp_format,
            temperature_unit,
            relative_humidity_unit,
            apparent_temperature_unit,
            precipitation_probability_unit,
            precipitation_unit,
            rain_unit,
            showers_unit,
            snowfall_unit,
            weather_code_unit,
            visibility_unit,
        ))
    }

    pub fn get_time_unit(self) -> String {
        self.time
    }

    pub fn get_temperature_unit(self) -> String {
        self.temperature_2m
    }

    pub fn get_relative_humidity_unit(self) -> String {
        self.relative_humidity_2m
    }

    pub fn get_apparaent_temperature_unit(self) -> String {
        self.apparent_temperature
    }

    pub fn get_precipitation_probability_unit(self) -> String {
        self.precipitation_probability
    }

    pub fn get_precipitation_unit(self) -> String {
        self.precipitation
    }

    pub fn get_rain_unit(self) -> String {
        self.rain
    }

    pub fn get_showers_unit(self) -> String {
        self.showers
    }

    pub fn get_snowfall_unit(self) -> String {
        self.snowfall
    }

    pub fn get_weather_code_unit(self) -> String {
        self.weather_code
    }

    pub fn get_visibility_unit(self) -> String {
        self.visibility
    }

    pub fn to_json(self) -> Result<String, WeatherDataError> {
        match serde_json::to_string(&self) {
            Ok(json_string) => Ok(json_string),
            Err(error) => {
                log::error!("Failed to serializate data to JSON, error: {}", error);
                Err(WeatherDataError::JsonSerializationError(error))
            }
        }
    }
}

impl fmt::Display for HourlyUnits {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "|{:18}|{:18}|{:22}|{:26}|{:26}|{:19}|{:10}|{:15}|{:13}|{:12}|{:15}|",
            format!("Time ({})", self.time),
            format!("Temperature ({})", self.temperature_2m),
            format!("Rel. Humidity ({})", self.relative_humidity_2m),
            format!("Appr. Temperature ({})", self.apparent_temperature),
            format!("Preci. Probability ({})", self.precipitation_probability),
            format!("Precipitation ({})", self.precipitation),
            format!("Rain ({})", self.rain),
            format!("Showers ({})", self.showers),
            format!("Snowfall ({})", self.snowfall),
            self.weather_code.to_uppercase(),
            format!("Visibility ({})", self.visibility),
        )
    }
}
#[derive(Debug, Clone, Serialize)]
pub struct WeatherDataPoint {
    time: String,
    temperature: f32,
    relative_humidity_2m: i32,
    apparent_temperature: f32,
    precipitation_probability: f32,
    precipitation: f32,
    rain: f32,
    showers: f32,
    snowfall: f32,
    weather_code: i32,
    visibility: f64,
}

impl WeatherDataPoint {
    const TIME: &'static str = "time";
    const TEMPERATURE_2M: &'static str = "temperature_2m";
    const RELATIVE_HUMIDITY_2M: &'static str = "relative_humidity_2m";
    const APPARENT_TEMPERATURE: &'static str = "apparent_temperature";
    const PRECIPITATION_PROBABILITY: &'static str = "precipitation_probability";
    const PRECIPITATION: &'static str = "precipitation";
    const RAIN: &'static str = "rain";
    const SHOWERS: &'static str = "showers";
    const SNOWFALL: &'static str = "snowfall";
    const WEATHER_CODE: &'static str = "weather_code";
    const VISIBILITY: &'static str = "visibility";

    #[allow(clippy::too_many_arguments)]
    pub fn new(
        time: String,
        temperature: f32,
        relative_humidity_2m: i32,
        apparent_temperature: f32,
        precipitation_probability: f32,
        precipitation: f32,
        rain: f32,
        showers: f32,
        snowfall: f32,
        weather_code: i32,
        visibility: f64,
    ) -> WeatherDataPoint {
        Self {
            time,
            temperature,
            relative_humidity_2m,
            apparent_temperature,
            precipitation_probability,
            precipitation,
            rain,
            showers,
            snowfall,
            weather_code,
            visibility,
        }
    }

    pub fn get_time(self) -> String {
        self.time
    }

    pub fn get_temperature(self) -> f32 {
        self.temperature
    }

    pub fn get_relative_humidity_2m(self) -> i32 {
        self.relative_humidity_2m
    }
    pub fn get_apparaent_temperature(self) -> f32 {
        self.apparent_temperature
    }

    pub fn get_precipitation_probability(self) -> f32 {
        self.precipitation_probability
    }

    pub fn get_precipitation(self) -> f32 {
        self.precipitation
    }

    pub fn get_rain(self) -> f32 {
        self.rain
    }

    pub fn get_showers(self) -> f32 {
        self.showers
    }

    pub fn get_snowfall(self) -> f32 {
        self.snowfall
    }

    pub fn get_weather_code(self) -> i32 {
        self.weather_code
    }

    pub fn get_visibility(self) -> f64 {
        self.visibility
    }

    pub fn to_json(self) -> Result<String, WeatherDataError> {
        match serde_json::to_string(&self) {
            Ok(json_string) => Ok(json_string),
            Err(error) => {
                log::error!("Failed to serializate data to JSON, error: {}", error);
                Err(WeatherDataError::JsonSerializationError(error))
            }
        }
    }
}

impl PartialEq for WeatherDataPoint {
    fn eq(&self, other: &WeatherDataPoint) -> bool {
        self.time == other.time
            && self.temperature == other.temperature
            && self.relative_humidity_2m == other.relative_humidity_2m
            && self.apparent_temperature == other.apparent_temperature
            && self.precipitation_probability == other.precipitation_probability
            && self.precipitation == other.precipitation
            && self.rain == other.rain
            && self.showers == other.showers
            && self.snowfall == other.snowfall
            && self.weather_code == other.weather_code
            && self.visibility == other.visibility
    }
}

impl fmt::Display for WeatherDataPoint {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "|{:18}|{:18.2}|{:22.2}|{:26.2}|{:26.2}|{:19.2}|{:10.2}|{:15.2}|{:13.2}|{:12}|{:15.2}|",
            self.time,
            self.temperature,
            self.relative_humidity_2m,
            self.apparent_temperature,
            self.precipitation_probability,
            self.precipitation,
            self.rain,
            self.showers,
            self.snowfall,
            self.weather_code,
            self.visibility
        )
    }
}

#[derive(Debug, Serialize)]
pub struct WeatherData {
    latitude: f64,
    longitude: f64,
    generationtime_ms: f64,
    utc_offset_seconds: f64,
    timezone: String,
    timezone_abbreviation: String,
    elevation: f32,
    pub hourly_units: HourlyUnits,
    pub hourly: HashMap<i64, WeatherDataPoint>,
}

impl fmt::Display for WeatherData {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Latitude: {:4.3}°\tLongitude: {:4.3}°\t Generation Time: {:5.3}ms\nUTC Offset: {:15.5}\tTimezone: {:30}\nTimezone Abbreviation: {:10}\tElevation: {:10.2}m",
            self.latitude,
            self.longitude,
            self.generationtime_ms,
            self.utc_offset_seconds,
            self.timezone,
            self.timezone_abbreviation,
            self.elevation
        )
    }
}

impl WeatherData {
    const LATITUDE: &'static str = "latitude";
    const LONGITUDE: &'static str = "longitude";
    const GENERATION_TIME_MS: &'static str = "generationtime_ms";
    const UTC_OFFSET_SECONDS: &'static str = "utc_offset_seconds";
    const TIMEZONE: &'static str = "timezone";
    const ELEVATION: &'static str = "elevation";
    const TIMEZONE_ABBREVIATION: &'static str = "timezone_abbreviation";
    const HOURLY_DATA: &'static str = "hourly";
    const HOURLY_UNITS: &'static str = "hourly_units";

    #[allow(clippy::too_many_arguments)]
    pub fn new(
        latitude: f64,
        longitude: f64,
        generationtime_ms: f64,
        utc_offset_seconds: f64,
        timezone: String,
        timezone_abbreviation: String,
        elevation: f32,
        hourly_units: HourlyUnits,
        hourly: HashMap<i64, WeatherDataPoint>,
    ) -> WeatherData {
        Self {
            latitude,
            longitude,
            generationtime_ms,
            utc_offset_seconds,
            timezone,
            timezone_abbreviation,
            elevation,
            hourly_units,
            hourly,
        }
    }

    pub fn parse_from(json_obj: Value) -> Result<WeatherData, WeatherDataError> {
        let latitude = match json_obj[Self::LATITUDE].as_f64() {
            Some(latitude) => latitude,
            None => return Err(WeatherDataError::ParseError),
        };
        log::debug!("Latitude: {}", latitude);

        let longitude = match json_obj[Self::LONGITUDE].as_f64() {
            Some(longitude) => longitude,
            None => return Err(WeatherDataError::ParseError),
        };
        log::debug!("Longitude: {}", longitude);

        let generationtime_ms = match json_obj[Self::GENERATION_TIME_MS].as_f64() {
            Some(generationtime_ms) => generationtime_ms,
            None => return Err(WeatherDataError::ParseError),
        };
        log::debug!("generationtime_ms: {}", generationtime_ms);

        let utc_offset_seconds = match json_obj[Self::UTC_OFFSET_SECONDS].as_f64() {
            Some(utc_offset_seconds) => utc_offset_seconds,
            None => return Err(WeatherDataError::ParseError),
        };
        log::debug!("utc_offset_seconds: {}", utc_offset_seconds);

        let timezone = match json_obj[Self::TIMEZONE].as_str() {
            Some(timezone) => timezone,
            None => return Err(WeatherDataError::ParseError),
        };
        log::debug!("timezone: {}", timezone);

        let timezone_abbr = match json_obj[Self::TIMEZONE_ABBREVIATION].as_str() {
            Some(timezone_abbr) => timezone_abbr.to_string(),
            None => return Err(WeatherDataError::ParseError),
        };

        log::debug!("Timezone Abbreviation: {}", timezone_abbr);

        let elevation = match json_obj[Self::ELEVATION].as_f64() {
            Some(elevation) => elevation as f32,
            None => 0.0,
        };
        log::debug!("elevation: {}", elevation);

        let binding = json_obj[Self::HOURLY_DATA].clone();
        let hourly_data = match binding.as_object() {
            Some(hourly_data) => hourly_data,
            None => return Err(WeatherDataError::ParseError),
        };

        let hourly_units_val = json_obj[Self::HOURLY_UNITS].clone().take();
        let hourly_units = match hourly_units_val.as_object() {
            Some(hourly_units) => hourly_units,
            None => return Err(WeatherDataError::ParseError),
        };

        let hourly_data = hourly_data.clone();
        let timestamps: &Vec<Value> = match hourly_data[WeatherDataPoint::TIME].as_array() {
            Some(timestamps) => timestamps,
            None => return Err(WeatherDataError::ParseError),
        };

        let temperatures: &Vec<Value> =
            match hourly_data[WeatherDataPoint::TEMPERATURE_2M].as_array() {
                Some(temperatures) => temperatures,
                None => return Err(WeatherDataError::ParseError),
            };

        let relative_humidities: &Vec<Value> =
            match hourly_data[WeatherDataPoint::RELATIVE_HUMIDITY_2M].as_array() {
                Some(relative_humidities) => relative_humidities,
                None => return Err(WeatherDataError::ParseError),
            };

        let apparent_temperatures =
            match hourly_data[WeatherDataPoint::APPARENT_TEMPERATURE].as_array() {
                Some(apparent_temperatures) => apparent_temperatures,
                None => return Err(WeatherDataError::ParseError),
            };

        let precipitation_probabilities =
            match hourly_data[WeatherDataPoint::PRECIPITATION_PROBABILITY].as_array() {
                Some(precipitation_probabilities) => precipitation_probabilities,
                None => return Err(WeatherDataError::ParseError),
            };

        let precipitations = match hourly_data[WeatherDataPoint::PRECIPITATION].as_array() {
            Some(precipitations) => precipitations,
            None => return Err(WeatherDataError::ParseError),
        };
        let rains = match hourly_data[WeatherDataPoint::RAIN].as_array() {
            Some(rains) => rains,
            None => return Err(WeatherDataError::ParseError),
        };

        let showers_s = match hourly_data[WeatherDataPoint::SHOWERS].as_array() {
            Some(showers_s) => showers_s,
            None => return Err(WeatherDataError::ParseError),
        };

        let snowfalls = match hourly_data[WeatherDataPoint::SNOWFALL].as_array() {
            Some(snowfalls) => snowfalls,
            None => return Err(WeatherDataError::ParseError),
        };

        let weather_codes = match hourly_data[WeatherDataPoint::WEATHER_CODE].as_array() {
            Some(weather_codes) => weather_codes,
            None => return Err(WeatherDataError::ParseError),
        };

        let visibilities = match hourly_data[WeatherDataPoint::VISIBILITY].as_array() {
            Some(visibilities) => visibilities,
            None => return Err(WeatherDataError::ParseError),
        };

        let hourly_data_units = match HourlyUnits::parse_from(hourly_units) {
            Ok(hourly_units) => hourly_units,
            Err(_) => return Err(WeatherDataError::ParseError),
        };

        let timezone_tz: chrono_tz::Tz = match Tz::from_str(&timezone_abbr) {
            Ok(timezone_tz) => timezone_tz.into(),
            Err(_) => return Err(WeatherDataError::DatapointParseError),
        };

        let mut weather_data_points: HashMap<i64, WeatherDataPoint> = HashMap::new();
        for (pos, timestamp) in timestamps.iter().enumerate() {
            let timestamp_str = match timestamp.as_str() {
                Some(time_stamp) => time_stamp,
                None => return Err(WeatherDataError::DatapointParseError),
            };

            // Parse datetime without timezone
            let timestamp_epoch =
                match NaiveDateTime::parse_from_str(timestamp_str, "%Y-%m-%dT%H:%M") {
                    Ok(naive_datetime) => match naive_datetime.and_local_timezone(timezone_tz) {
                        chrono::offset::LocalResult::Single(localtime) => localtime.timestamp(),
                        chrono::offset::LocalResult::Ambiguous(_earlier, latest) => {
                            log::error!("Time stamp may not be accurate for {} ", timestamp_str);
                            latest.timestamp()
                        }
                        chrono::offset::LocalResult::None => {
                            return Err(WeatherDataError::DatapointParseError)
                        }
                    },
                    Err(error) => {
                        log::error!("Cannot parse time to epoch, error: {:?}", error);
                        return Err(WeatherDataError::DatapointParseError);
                    }
                };

            let temperature = match temperatures.get(pos) {
                Some(temperature_unparsed) => match temperature_unparsed.as_f64() {
                    Some(temperature_unparsed) => temperature_unparsed as f32,
                    None => return Err(WeatherDataError::DatapointParseError),
                },
                None => return Err(WeatherDataError::DatapointParseError),
            };

            let relative_humidity_2m = match relative_humidities.get(pos) {
                Some(relative_humidity) => match relative_humidity.as_f64() {
                    Some(relative_humidity) => relative_humidity as i32,
                    None => return Err(WeatherDataError::DatapointParseError),
                },
                None => return Err(WeatherDataError::DatapointParseError),
            };

            let apparent_temperature = match apparent_temperatures.get(pos) {
                Some(app_temp) => match app_temp.as_f64() {
                    Some(app_temp) => app_temp as f32,
                    None => return Err(WeatherDataError::DatapointParseError),
                },
                None => return Err(WeatherDataError::DatapointParseError),
            };

            let precipitation_probability = match precipitation_probabilities.get(pos) {
                Some(precip_prob) => match precip_prob.as_f64() {
                    Some(precip_prob) => precip_prob as f32,
                    None => return Err(WeatherDataError::DatapointParseError),
                },
                None => return Err(WeatherDataError::DatapointParseError),
            };

            let precipitation = match precipitations.get(pos) {
                Some(prepcip) => match prepcip.as_f64() {
                    Some(prepcip) => prepcip as f32,
                    None => return Err(WeatherDataError::DatapointParseError),
                },
                None => return Err(WeatherDataError::DatapointParseError),
            };

            let rain = match rains.get(pos) {
                Some(rains_data) => match rains_data.as_f64() {
                    Some(rains_data) => rains_data as f32,
                    None => return Err(WeatherDataError::DatapointParseError),
                },
                None => return Err(WeatherDataError::DatapointParseError),
            };

            let showers = match showers_s.get(pos) {
                Some(shower_data) => match shower_data.as_f64() {
                    Some(shower_data) => shower_data as f32,
                    None => return Err(WeatherDataError::DatapointParseError),
                },
                None => return Err(WeatherDataError::DatapointParseError),
            };

            let snowfall = match snowfalls.get(pos) {
                Some(snowfall_data) => match snowfall_data.as_f64() {
                    Some(snowfall_data) => snowfall_data as f32,
                    None => return Err(WeatherDataError::DatapointParseError),
                },
                None => return Err(WeatherDataError::DatapointParseError),
            };

            let weather_code = match weather_codes.get(pos) {
                Some(weather_code_data) => match weather_code_data.as_i64() {
                    Some(weather_code_data) => weather_code_data as i32,
                    None => return Err(WeatherDataError::DatapointParseError),
                },
                None => return Err(WeatherDataError::DatapointParseError),
            };

            let visibility = match visibilities.get(pos) {
                Some(visibility) => match visibility.as_f64() {
                    Some(visibility) => visibility,
                    None => return Err(WeatherDataError::DatapointParseError),
                },
                None => return Err(WeatherDataError::DatapointParseError),
            };
            weather_data_points.insert(
                timestamp_epoch,
                WeatherDataPoint::new(
                    timestamp_str.to_string(),
                    temperature,
                    relative_humidity_2m,
                    apparent_temperature,
                    precipitation_probability,
                    precipitation,
                    rain,
                    showers,
                    snowfall,
                    weather_code,
                    visibility,
                ),
            );
        }

        Ok(Self {
            latitude,
            longitude,
            generationtime_ms,
            utc_offset_seconds,
            timezone: timezone.to_string(),
            timezone_abbreviation: timezone_abbr,
            elevation,
            hourly_units: hourly_data_units,
            hourly: weather_data_points,
        })
    }

    pub fn to_json(self) -> Result<String, WeatherDataError> {
        match serde_json::to_string(&self) {
            Ok(json_string) => Ok(json_string),
            Err(error) => {
                log::error!("Failed to serializate data to JSON, error: {}", error);
                Err(WeatherDataError::JsonSerializationError(error))
            }
        }
    }
}
