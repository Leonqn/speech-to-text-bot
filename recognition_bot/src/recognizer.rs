use hyper::Body;
use hyper::client::HttpConnector;
use hyper::Client;
use hyper;
use futures::Future;
use hyper::Request;
use futures::stream::Stream;
use hyper::StatusCode;
use std::str;
use std::fmt;

#[derive(Debug)]
pub enum Error {
    HyperError(hyper::Error),
    BadResponse(StatusCode, String),
}

impl From<hyper::Error> for Error {
    fn from(err: hyper::Error) -> Self {
        Error::HyperError(err)
    }
}


impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Error::HyperError(hyper) =>
                write!(f, "Http error: {}", hyper),
            Error::BadResponse(code, message) =>
                write!(f, "Bad response. Code: {}, message: {}", code, message),
        }
    }
}

pub struct Recognizer {
    http_client: Client<HttpConnector, Body>,
    uri: String,
    supported_languages: Vec<Language>,
}

pub struct Language {
    pub friendly_name: String,
    pub code: String,
}

impl Language {
    pub fn new<S: Into<String>>(friendly_name: S, code: S) -> Self {
        Language {
            friendly_name: friendly_name.into(),
            code: code.into(),
        }
    }
}

impl Recognizer {
    pub fn new<S: Into<String>>(recognizer_uri: S) -> Recognizer {
        Recognizer {
            http_client: hyper::Client::new(),
            uri: recognizer_uri.into(),
            supported_languages: vec![
                Language::new("Russian (Russia)", "ru-RU"),
                Language::new("English (USA)", "en-us"),
                Language::new("German (Germany)", "de-DE"),
                Language::new("Spanish (Spain)", "es-ES"),
            ],
        }
    }

    pub fn recognize_audio(&self, bytes: Vec<u8>, lang: &str) -> impl Future<Item=String, Error=Error> {
        let request =
            Request::post(format!("{}?lang={}", &self.uri, lang))
                .body(Body::from(bytes))
                .expect("While creating request an error has occurred");

        self.http_client.request(request)
            .and_then(|r| {
                let status = r.status();
                r.into_body().concat2().map(move |body| (status, body))
            })
            .map_err(Error::from)
            .then(|result| {
                let (status, body) = result?;
                let string_body = str::from_utf8(&body)
                    .map_err(|_| Error::BadResponse(status, "utf8 error".to_string()))
                    ?.to_string();

                if status.is_success() {
                    Ok(string_body)
                } else {
                    Err(Error::BadResponse(status, string_body))
                }
            })
    }

    pub fn get_supported_languages(&self) -> &Vec<Language> {
        &self.supported_languages
    }
}
