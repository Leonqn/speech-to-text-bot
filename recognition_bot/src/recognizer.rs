use futures::stream::Stream;
use futures::Future;
use hyper;
use hyper::client::HttpConnector;
use hyper::Body;
use hyper::Client;
use hyper::Request;
use hyper::StatusCode;
use std::fmt;
use std::str;
use hyper::body;
use anyhow::anyhow;

#[derive(Clone)]
pub struct Recognizer {
    http_client: Client<HttpConnector, Body>,
    uri: String,
}

impl Recognizer {
    pub fn new<S: Into<String>>(recognizer_uri: S) -> Recognizer {
        Recognizer {
            http_client: hyper::Client::new(),
            uri: recognizer_uri.into(),
        }
    }

    pub async fn recognize_audio(
        &self,
        bytes: Vec<u8>,
    ) -> anyhow::Result<String> {
        let request = Request::post(format!("{}?lang=ru-RU", &self.uri))
            .body(Body::from(bytes))
            .expect("While creating request an error has occurred");
        let response = self.http_client.request(request).await?;
        let status = response.status();
        let body = body::to_bytes(response.into_body()).await?;
        let body = str::from_utf8(&body)?.to_string();
        if status.is_success() {
            Ok(body)
        } else {
            Err(anyhow!("Api responded with status {} and body {}", status, body))
        }
    }
}
