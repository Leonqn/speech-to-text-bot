use std::io;

use config::Config;
use config::ConfigError;
use log::info;
use serde::Deserialize;

mod media_converter;
mod bot;
mod recognizer;
mod storage;

#[derive(Deserialize, Debug)]
pub struct Settings {
    pub bot_apikey: String,
    pub recognizer_uri: String,
    pub db_file_path: String,
}

impl Settings {
    pub fn new() -> Result<Settings, ConfigError> {
        let mut settings = Config::default();
        settings.merge(config::File::new("config.ini", config::FileFormat::Ini).required(false))?;

        settings.merge(config::Environment::new())?;

        settings.try_into()
    }
}


fn main() -> Result<(), io::Error> {
    env_logger::init();
    info!("Started");

    let settings = Settings::new().expect("Wrong settings");

    let recognizer = recognizer::Recognizer::new(settings.recognizer_uri);
    let media_converter = media_converter::MediaConverter::new();
    let storage = storage::Storage::new(settings.db_file_path).unwrap();
    let bot_api_client = rutebot::client::Rutebot::new(settings.bot_apikey);

    let bot = bot::Bot::new(bot_api_client, media_converter, recognizer, storage);

    hyper::rt::run(bot.start_bot());

    Ok(())
}
