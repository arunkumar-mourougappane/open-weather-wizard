use chrono::{DateTime, NaiveDateTime};
use log::log;
use core::fmt;
use std::{collections::HashMap, str::FromStr};

use chrono_tz::Tz;
use serde_json::{Map, Value};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum WeatherDataError {
    #[error("Cannot parse wether data point")]
    ParseError,
}

#[derive(Debug, Clone)]
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
}

#[derive(Debug)]
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
    visibility: i32,
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
        visibility: i32,
    ) -> Self {
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
            "{}|{:.2}|{:.2}|{:.2}|{:.2}|{:.2}|{:.2}|{:.2}|{}|{}",
            self.time,
            self.temperature,
            self.relative_humidity_2m,
            self.apparent_temperature,
            self.precipitation_probability,
            self.precipitation,
            self.rain,
            self.showers,
            self.weather_code,
            self.visibility
        )
    }
}

#[derive(Debug)]
pub struct WeatherData {
    latitude: f32,
    longitude: f32,
    generationtime_ms: f64,
    utc_offset_seconds: f64,
    timezone: String,
    timezone_abbrevation: String,
    elevation: f32,
    hourly_units: HourlyUnits,
    pub houlry: HashMap<i64, WeatherDataPoint>,
}

impl WeatherData {
    const LATITUDE: &'static str = "latitude";
    const LONGIUDE: &'static str = "longitude";
    const GENERATION_TIME_MS: &'static str = "generationtime_ms";
    const UTC_OFFSET_SECONDS: &'static str = "utc_offset_seconds";
    const TIMEZONE: &'static str = "timezone";
    const ELEVATION: &'static str = "elevation";
    const TIMEZONE_ABBREVATION: &'static str = "timezone_abbreviation";
    const TIMESTAMP: &'static str = "time";
    const HOURLY_DATA: &'static str = "hourly";
    const HOURLY_UNITS: &'static str = "hourly_units";

    pub fn new(
        latitude: f32,
        longitude: f32,
        generationtime_ms: f64,
        utc_offset_seconds: f64,
        timezone: String,
        timezone_abbrevation: String,
        elevation: f32,
        hourly_units: HourlyUnits,
        houlry: HashMap<i64, WeatherDataPoint>,
    ) -> WeatherData {
        WeatherData {
            latitude: latitude,
            longitude: longitude,
            generationtime_ms: generationtime_ms,
            utc_offset_seconds: utc_offset_seconds,
            timezone: timezone,
            timezone_abbrevation: timezone_abbrevation,
            elevation: elevation,
            hourly_units: hourly_units,
            houlry,
        }
    }

    pub fn parse_from(json_obj: Value) -> Result<WeatherData, WeatherDataError> {
        let latitude = match json_obj[Self::LATITUDE].as_f64() {
            Some(latitude) => latitude as f32,
            None => 0.0,
        };
        log::info!("Latitude: {}", latitude);

        let longitude = match json_obj[Self::LONGIUDE].as_f64() {
            Some(longitude) => longitude as f32,
            None => 0.0,
        };
        log::info!("Longitude: {}", longitude);

        let generationtime_ms = match json_obj[Self::GENERATION_TIME_MS].as_f64() {
            Some(generationtime_ms) => generationtime_ms,
            None => 0.0,
        };
        log::info!("generationtime_ms: {}", generationtime_ms);

        let utc_offset_seconds = match json_obj[Self::UTC_OFFSET_SECONDS].as_f64() {
            Some(utc_offset_seconds) => utc_offset_seconds,
            None => 0.0,
        };
        log::info!("utc_offset_seconds: {}", utc_offset_seconds);

        let timezone = match json_obj[Self::TIMEZONE].as_str() {
            Some(timezone) => timezone,
            None => return Err(WeatherDataError::ParseError),
        };
        log::info!("timezone: {}", timezone);

        let timezone_abbr = match json_obj[Self::TIMEZONE_ABBREVATION].as_str() {
            Some(timezone_abbr) => timezone_abbr.to_string(),
            None => return Err(WeatherDataError::ParseError),
        };

        log::info!("Timezone Abbreviation: {}", timezone_abbr);

        let elevation = match json_obj[Self::ELEVATION].as_f64() {
            Some(elevation) => elevation as f32,
            None => 0.0,
        };
        log::info!("elevation: {}", elevation);

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
        let timestamps: &Vec<Value> = hourly_data[WeatherDataPoint::TIME].as_array().unwrap();
        let temperatures: &Vec<Value> = hourly_data[WeatherDataPoint::TEMPERATURE_2M].as_array().unwrap();
        println!("{:?}",hourly_data);
        let relative_humidities: &Vec<Value> = hourly_data[WeatherDataPoint::RELATIVE_HUMIDITY_2M].as_array().unwrap();
        log::info!("{:?}", relative_humidities);
        let apparent_temperatures = hourly_data[WeatherDataPoint::APPARENT_TEMPERATURE].as_array().unwrap();
        let precipitation_probabilities =
            hourly_data[WeatherDataPoint::PRECIPITATION_PROBABILITY].as_array().unwrap();
        let precipitations = hourly_data[WeatherDataPoint::PRECIPITATION].as_array().unwrap();
        let rains = hourly_data[WeatherDataPoint::RAIN].as_array().unwrap();
        let showers_s = hourly_data[WeatherDataPoint::SHOWERS].as_array().unwrap();
        let snowfalls = hourly_data[WeatherDataPoint::SNOWFALL].as_array().unwrap();
        let weather_codes = hourly_data[WeatherDataPoint::WEATHER_CODE].as_array().unwrap();
        let visibilities = hourly_data[WeatherDataPoint::VISIBILITY].as_array().unwrap();

        let hourly_data_uints = match HourlyUnits::parse_from(hourly_units) {
            Ok(hourly_units) => hourly_units,
            Err(_) => return Err(WeatherDataError::ParseError),
        };

        let timezone_tz = Tz::from_str(&timezone_abbr).ok().unwrap();

        let mut weather_data_points: HashMap<i64, WeatherDataPoint> = HashMap::new();
        for (pos, timestamp) in timestamps.iter().enumerate() {
            let timestamp_str = timestamp.as_str().unwrap();
            // Parse datetime without timezone
            let naive_datetime =
                NaiveDateTime::parse_from_str(timestamp_str, "%Y-%m-%dT%H:%M").unwrap();
            let localtime = naive_datetime.and_local_timezone(timezone_tz).unwrap();
            let timestamp_epoch = localtime.timestamp();

            let temperature = temperatures.get(pos).unwrap().as_f64().unwrap() as f32;
            let relative_humidity_2m =
                relative_humidities.get(pos).unwrap().as_f64().unwrap() as i32;
            let apparent_temperature =
                apparent_temperatures.get(pos).unwrap().as_f64().unwrap() as f32;
            let precipitation_probability = precipitation_probabilities
                .get(pos)
                .unwrap()
                .as_u64()
                .unwrap() as f32;
            println!("{}", precipitations.len());
            let precipitation = precipitations.get(pos).unwrap().as_u64().unwrap() as f32;

            let rain = rains.get(pos).unwrap().as_i64().unwrap() as f32;
            let showers = showers_s.get(pos).unwrap().as_f64().unwrap() as f32;
            let snowfall = snowfalls.get(pos).unwrap().as_f64().unwrap() as f32;
            let weather_code = weather_codes.get(pos).unwrap().as_i64().unwrap() as i32;
            let visibility = visibilities.get(pos).unwrap().as_i64().unwrap() as i32;
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
            timezone_abbrevation: timezone_abbr,
            elevation,
            hourly_units: hourly_data_uints,
            houlry: weather_data_points,
        })
    }
}
