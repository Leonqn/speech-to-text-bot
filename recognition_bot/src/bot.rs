use std::fmt;
use std::sync::Arc;
use std::time::Duration;

use futures::{Future, Stream};
use log::error;
use rutebot::client::Rutebot;
use rutebot::requests::answer_callback_query::AnswerCallbackQuery;
use rutebot::requests::ChatId;
use rutebot::requests::get_updates::{AllowedUpdate, GetUpdates};
use rutebot::requests::send_chat_action::{ChatAction, SendChatAction};
use rutebot::requests::{InlineKeyboard, InlineKeyboardButton, ReplyMarkup};
use rutebot::responses::{CallbackQuery, Message, Update};

use crate::media_converter::*;
use crate::media_converter;
use crate::recognizer::*;
use crate::recognizer;
use crate::storage;
use crate::storage::Storage;
use rutebot::requests::send_text::SendText;
use rutebot::requests::get_file::GetFile;

#[derive(Debug)]
pub enum Error {
    BotClientError(rutebot::error::Error),
    ConvertError(media_converter::Error),
    RecognitionError(recognizer::Error),
    StorageError(storage::Error),
}

impl From<rutebot::error::Error> for Error {
    fn from(err: rutebot::error::Error) -> Self {
        Error::BotClientError(err)
    }
}

impl From<media_converter::Error> for Error {
    fn from(err: media_converter::Error) -> Self {
        Error::ConvertError(err)
    }
}

impl From<recognizer::Error> for Error {
    fn from(err: recognizer::Error) -> Self {
        Error::RecognitionError(err)
    }
}

impl From<storage::Error> for Error {
    fn from(err: storage::Error) -> Self {
        Error::StorageError(err)
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Error::BotClientError(err) =>
                write!(f, "Error occurred in bot client library: {}", err),
            Error::ConvertError(err) =>
                write!(f, "Error occurred in conversion module: {}", err),
            Error::RecognitionError(err) =>
                write!(f, "Error occurred in speech to text module: {}", err),
            Error::StorageError(err) =>
                write!(f, "Error occurred in storage module: {}", err),
        }
    }
}


#[derive(Clone)]
pub struct Bot {
    inner: Arc<InnerBot>,
    default_timeout: Duration,
}

struct InnerBot {
    bot_api_client: Rutebot,
    media_converter: MediaConverter,
    recognizer: Recognizer,
    db: Storage,
}

enum VideoOrVoice {
    Video,
    Voice,
}

impl Bot {
    pub fn new(bot_api_client: Rutebot, media_converter: MediaConverter, recognizer: Recognizer, db: Storage) -> Bot {
        Bot {
            inner: Arc::new(InnerBot {
                bot_api_client,
                media_converter,
                recognizer,
                db,
            }),
            default_timeout: Duration::new(120, 0),
        }
    }


    pub fn start_bot(&self) -> impl Future<Item=(), Error=()> {
        let allowed_updates = [AllowedUpdate::Message, AllowedUpdate::CallbackQuery];
        let get_updates_request = GetUpdates {
            offset: None,
            limit: None,
            timeout: Some(100),
            allowed_updates: Some(&allowed_updates),
        };
        let self_1 = self.clone();
        self.inner.bot_api_client.incoming_updates(get_updates_request)
            .then(|res| {
                let ok: Result<_, ()> = Ok(res);
                ok
            })
            .for_each(move |update| {
                match update {
                    Ok(update) => {
                        self_1.handle_messages(update);
                        Ok(())
                    }
                    Err(err) => {
                        error!("Error in getting updates {:?} ", err);
                        Ok(())
                    }
                }
            })
    }

    fn handle_messages(&self, update: Update) {
        match update {
            Update {
                message: Some(Message { chat, text: Some(text), .. }),
                ..
            } => {
                match text.as_str() {
                    "/set_lang" => {
                        hyper::rt::spawn(
                            self
                                .handle_set_lang(chat.id)
                                .map_err(|err| error!("Error in setting lang: {:?}", err)));
                    }

                    "/help" => {
                        hyper::rt::spawn(self.handle_help(chat.id)
                            .map_err(|err| error!("Error in getting help: {:?}", err)));
                    }
                    _ => (),
                }
            }

            Update {
                message: Some(Message { message_id, chat, voice: Some(voice), .. }),
                ..
            } => {
                hyper::rt::spawn(self.handle_media_message(chat.id, message_id, VideoOrVoice::Voice, voice.file_id)
                    .map_err(|err| error!("Error in recognizing voice: {:?}", err)));
            }

            Update {
                message: Some(Message { message_id, chat, video_note: Some(video_note), .. }),
                ..
            } => {
                hyper::rt::spawn(self.handle_media_message(chat.id, message_id, VideoOrVoice::Video, video_note.file_id)
                    .map_err(|err| error!("Error in recognizing video note: {:?}", err)));
            }

            Update {
                callback_query: Some(CallbackQuery { data: Some(data), id, message: Some(Message { chat, .. }), .. }),
                ..
            } => {
                hyper::rt::spawn(self.handle_lang_has_set(chat.id, id, data)
                    .map_err(|err| error!("Error in response from setting lang: {:?}", err)));
            }

            _ => (),
        }
    }

    fn handle_set_lang(&self, chat_id: i64) -> impl Future<Item=(), Error=Error> {
        let keyboard = self.inner.recognizer.get_supported_languages().chunks(2).map(|x| {
            x.iter().map(|lang| InlineKeyboardButton::CallbackData {
                text: &lang.friendly_name,
                callback_data: &lang.code,
            }).collect()
        }).collect::<Vec<_>>();

        let keyboard =
            ReplyMarkup::InlineKeyboard(InlineKeyboard {
                inline_keyboard: keyboard.as_slice()
            });
        let request = SendText {
            reply_markup: Some(keyboard),
            ..SendText::new(chat_id, "Choose language for recognition")
        };

        self.inner.bot_api_client.prepare_api_request(request).send()
            .map(|_| ())
            .map_err(From::from)
    }

    fn handle_lang_has_set(&self, chat_id: i64, callback_id: String, callback_data: String) -> impl Future<Item=(), Error=Error> {
        self.inner.db.put(chat_id, callback_data);
        self.inner.bot_api_client.prepare_api_request(AnswerCallbackQuery {
            callback_query_id: &callback_id,
            text: Some("Language has been changed"),
            show_alert: false,
            url: None,
            cache_time: None,
        }).send()
            .map(|_| ())
            .map_err(From::from)
    }

    fn handle_help(&self, chat_id: i64) -> impl Future<Item=(), Error=Error> {
        let help_msg =
            "Hello! I can convert voice and video note messages to text. \
             You can forward messages to me or add me to chat. \
             Please choose language by command /set_lang. \
             Default language is russian";

        self.inner.bot_api_client.prepare_api_request(SendText::new(chat_id, help_msg)).send()
            .map_err(From::from)
            .map(|_| ())
    }

    fn handle_media_message(&self, chat_id: i64, msg_id: i64, video_or_voice: VideoOrVoice, file_id: String) -> impl Future<Item=(), Error=Error> {
        let self_1 = self.clone();
        let self_2 = self.clone();
        let self_3 = self.clone();
        let self_4 = self.clone();

        hyper::rt::spawn(self.inner.bot_api_client
            .prepare_api_request(SendChatAction { chat_id: ChatId::Id(chat_id), action: ChatAction::Typing }).send()
            .map_err(|err| error!("Error in sending chat action: {}", err))
            .map(|_| ()));

        self.inner.bot_api_client.prepare_api_request(GetFile::new(&file_id)).send()
            .and_then(move |file| self_4.inner.bot_api_client.download_file(&file.file_path.as_ref().map_or("", String::as_str)))
            .map(move |file|
                match video_or_voice {
                    VideoOrVoice::Video => MediaKind::Mp4(file),
                    VideoOrVoice::Voice => MediaKind::Ogg(file)
                })
            .map_err(Error::from)
            .and_then(move |media_kind| self_1.inner.media_converter.convert(media_kind).map_err(From::from))
            .and_then(move |audio| self_2.inner.recognizer.recognize_audio(audio, &self_2.inner.db.get(chat_id).as_ref().map_or("ru-RU", String::as_str)).map_err(From::from))
            .then(move |result| {
                let reply_msg = match result {
                    Ok(ref recognized) => recognized,
                    Err(err) => {
                        error!("Got error while handling media message {:?}", err);
                        "Something went wrong. Please try again later."
                    }
                };
                let request = SendText::new_reply(chat_id, reply_msg, msg_id);
                self_3.inner.bot_api_client
                    .prepare_api_request(request).send()
                    .map_err(From::from)
                    .map(|_| ())
            })
    }
}