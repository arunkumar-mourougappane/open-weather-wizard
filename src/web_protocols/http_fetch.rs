use std::string::FromUtf8Error;

use curl::easy::{Easy2, Handler, WriteError};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum HttpError {
    #[error("unknown data store error")]
    Unknown,
    #[error("Curl Error:")]
    CurlError(#[from] curl::Error),
    #[error("curl form error")]
    CurlFormError(#[from] curl::FormError),
    #[error("curl Multi error")]
    CurlMError(#[from] curl::MultiError),
    #[error("Parse error")]
    ParseError(#[from] FromUtf8Error),
    #[error("HTTP Response was not 200, Observed HTTP Error: `{0}`")]
    NotOKResponse(u32),
}

struct Collector(Vec<u8>);

impl Handler for Collector {
    fn write(&mut self, data: &[u8]) -> Result<usize, WriteError> {
        self.0.extend_from_slice(data);
        Ok(data.len())
    }
}

pub fn perform_http_get(url: String) -> Result<String, HttpError> {
    let mut easy = Easy2::new(Collector(Vec::new()));
    // Set the URL
    match easy.url(url.to_string().as_str()) {
        Ok(_) => match easy.get(true) {
            Ok(_) => {
                log::debug!("Performing HTTP GET for: {}", url.to_string());
                // Perform the request
                match easy.perform() {
                    Ok(_) => match easy.response_code() {
                        Ok(http_response_code) => {
                            if http_response_code == 200 {
                                let contents = easy.get_ref();
                                match String::from_utf8(contents.0.clone()) {
                                    Ok(payload) => Ok(payload),
                                    Err(error) => {
                                        log::error!(
                                            "Cannot parse the http reponse contents to string."
                                        );
                                        Err(HttpError::ParseError(error))
                                    }
                                }
                            } else {
                                log::error!(
                                    "HTTP Operationl, Response code: {}",
                                    http_response_code
                                );
                                Err(HttpError::NotOKResponse(http_response_code))
                            }
                        }
                        Err(error) => {
                            log::error!("Failed to parse response code, error code: {:?}", error);
                            Err(HttpError::CurlError(error))
                        }
                    },
                    Err(error) => {
                        log::error!("Failed to perform HTTP GET, error code: {:?}", error);
                        Err(HttpError::CurlError(error))
                    }
                }
            }
            Err(error) => {
                log::error!("Http Operation failed with error code: {:?}", error);
                Err(HttpError::CurlError(error))
            }
        },
        Err(error) => {
            log::error!("Cannot create url, error code: {:?}", error);
            Err(HttpError::CurlError(error))
        }
    }
}
