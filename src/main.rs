use curl::easy::{Easy2, Handler, WriteError};
use meteo_wizard::{settings::url_config::{HourlyTempFromGround, UrlConfig}, weather_data::weather_point::WeatherData};
use serde_json::Value;
use env_logger;

struct Collector(Vec<u8>);

impl Handler for Collector {
    fn write(&mut self, data: &[u8]) -> Result<usize, WriteError> {
        self.0.extend_from_slice(data);
        Ok(data.len())
    }
}

fn main() {

    env_logger::builder()
    .filter_level(log::LevelFilter::Info)
    .init();

    let url_config = UrlConfig::new(
        40.6936,
        89.5890,
        HourlyTempFromGround::TempAt2m,
        true,
        true,
        true,
        true,
        true,
        true,
        true,
        true,
        true,
        0,
        1,
    );

    let mut easy = Easy2::new(Collector(Vec::new()));
    println!("{}", url_config.to_string());
    // Set the URL
    easy.url(url_config.to_string().as_str()).unwrap();
    easy.get(true).unwrap();
    // Perform the request
    easy.perform().unwrap();

    assert_eq!(easy.response_code().unwrap(), 200);
    let contents = easy.get_ref();
    let weather_data_str = String::from_utf8(contents.0.clone()).unwrap();

    let weather_json: Value = serde_json::from_str(&weather_data_str).unwrap();

    let weather_data = WeatherData::parse_from(weather_json);
    match weather_data {
        Ok(weather_data) => {

            log::info!("\n{}\n",weather_data);
            log::info!("{}",weather_data.hourly_units);
            let data_points = weather_data.hourly;
            for weather_data_point in data_points{
                log::info!("{}", weather_data_point.1);
            }
        }
        Err(error) => println!("{}", error),
    }

}
