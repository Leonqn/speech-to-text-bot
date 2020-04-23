use std::io;

use config::Config;
use config::ConfigError;
use log::info;
use serde::Deserialize;

mod bot;
mod media_converter;
mod recognizer;

#[derive(Deserialize, Debug)]
pub struct Settings {
    pub bot_apikey: String,
    pub recognizer_uri: String,
}

impl Settings {
    pub fn new() -> Result<Settings, ConfigError> {
        let mut settings = Config::default();
        settings.merge(config::File::new("config.ini", config::FileFormat::Ini).required(false))?;

        settings.merge(config::Environment::new())?;

        settings.try_into()
    }
}

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    env_logger::init();
    info!("Started");

    let settings = Settings::new().expect("Wrong settings");

    let recognizer = recognizer::Recognizer::new(settings.recognizer_uri);
    let bot_api_client = rutebot::client::Rutebot::new(settings.bot_apikey);

    let bot = bot::Bot::new(bot_api_client, recognizer);
    bot.start_bot().await?;

    Ok(())
}
