use relegram::BotApiClient;
use relegram::responses::*;
use relegram::requests::*;
use relegram;
use std::sync::Arc;
use futures::{Future, Stream};
use crate::media_converter::*;
use crate::media_converter;
use crate::recognizer::*;
use crate::recognizer;
use std::time::Duration;
use log::error;
use crate::storage::Storage;
use crate::storage;
use std::fmt;

#[derive(Debug)]
pub enum Error {
    BotClientError(relegram::error::Error),
    ConvertError(media_converter::Error),
    RecognitionError(recognizer::Error),
    StorageError(storage::Error),
}

impl From<relegram::error::Error> for Error {
    fn from(err: relegram::error::Error) -> Self {
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
    bot_api_client: BotApiClient,
    media_converter: MediaConverter,
    recognizer: Recognizer,
    db: Storage,
}

enum VideoOrVoice {
    Video,
    Voice,
}

impl Bot {
    pub fn new(bot_api_client: BotApiClient, media_converter: MediaConverter, recognizer: Recognizer, db: Storage) -> Bot {
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
        let get_updates_request = GetUpdatesRequest {
            offset: None,
            limit: None,
            timeout: Some(100),
            allowed_updates: Some(vec![AllowedUpdate::Message, AllowedUpdate::CallbackQuery]),
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
                        self_1.handle_messages(update.kind);
                        Ok(())
                    }
                    Err(err) => {
                        error!("Error in getting updates {:?} ", err);
                        Ok(())
                    }
                }
            })
    }

    fn handle_messages(&self, update: UpdateKind) {
        match update {
            UpdateKind::Message(Message { id, from: MessageFrom::User { chat: Chat { id: chat_id, .. }, .. }, kind, .. }) => {
                match kind {
                    MessageKind::Text { entities: Some(entities), .. } => {
                        match entities.first() {
                            Some(MessageEntity::BotCommand(command)) => {
                                match command.as_str() {
                                    "/set_lang" => {
                                        hyper::rt::spawn(
                                            self
                                                .handle_set_lang(chat_id)
                                                .map_err(|err| error!("Error in setting lang: {:?}", err)));
                                    }

                                    "/help" => {
                                        hyper::rt::spawn(self.handle_help(chat_id)
                                            .map_err(|err| error!("Error in getting help: {:?}", err)));
                                    }
                                    _ => (),
                                }
                            }
                            _ => (),
                        }
                    }

                    MessageKind::Voice { voice, .. } => {
                        hyper::rt::spawn(self.handle_media_message(chat_id, id, VideoOrVoice::Voice, voice.file_id)
                            .map_err(|err| error!("Error in recognizing voice: {:?}", err)));
                    }
                    MessageKind::VideoNote { video_note } => {
                        hyper::rt::spawn(self.handle_media_message(chat_id, id, VideoOrVoice::Video, video_note.file_id)
                            .map_err(|err| error!("Error in recognizing video note: {:?}", err)));
                    }
                    _ => (),
                }
            }

            UpdateKind::CallbackQuery(CallbackQuery { data: Some(data), id, message: Some(Message { from: MessageFrom::User { chat, .. }, .. }), .. }) => {
                hyper::rt::spawn(self.handle_lang_has_set(chat.id, id, data)
                    .map_err(|err| error!("Error in response from setting lang: {:?}", err)));
            }
            _ => (),
        }
    }

    fn handle_set_lang(&self, chat_id: i64) -> impl Future<Item=(), Error=Error> {
        let keyboard =
            ReplyMarkup::InlineKeyboard(InlineKeyboard {
                inline_keyboard: self.inner.recognizer.get_supported_languages().chunks(2).map(|x| x.iter().map(|lang| InlineKeyboardButton {
                    text: lang.friendly_name.to_string(),
                    url: None,
                    callback_data: Some(lang.code.to_string()),
                    switch_inline_query: None,
                    switch_inline_query_current_chat: None,
                    pay: false,
                }).collect()).collect()
            });
        let request = SendMessageRequest {
            reply_markup: Some(keyboard),
            ..SendMessageRequest::new(
                ChatId::Id(chat_id),
                SendMessageKind::Text(
                    SendText::new("Choose language for recognition".to_string())))
        };

        self.inner.bot_api_client.send_message(&request, self.default_timeout)
            .map(|_| ())
            .map_err(From::from)
    }

    fn handle_lang_has_set(&self, chat_id: i64, callback_id: String, callback_data: String) -> impl Future<Item=(), Error=Error> {
        self.inner.db.put(chat_id, callback_data);
        self.inner.bot_api_client.answer_callback_query(
            &AnswerCallbackQuery {
                callback_query_id: callback_id,
                text: Some("Language has been changed".to_string()),
                show_alert: false,
                url: None,
                cache_time: None,
            }, self.default_timeout)
            .map(|_| ())
            .map_err(From::from)
    }

    fn handle_help(&self, chat_id: i64) -> impl Future<Item=(), Error=Error> {
        let help_msg =
            "Hello! I can convert voice and video note messages to text. \
             You can forward messages to me or add me to chat. \
             Please choose language by command /set_lang. \
             Default language is russian"
                .to_string();

        self.inner.bot_api_client.send_message(
            &SendMessageRequest::new(
                ChatId::Id(chat_id),
                SendMessageKind::Text(
                    SendText::new(help_msg))), self.default_timeout)
            .map_err(From::from)
            .map(|_| ())
    }

    fn handle_media_message(&self, chat_id: i64, msg_id: i64, video_or_voice: VideoOrVoice, file_id: String) -> impl Future<Item=(), Error=Error> {
        let self_1 = self.clone();
        let self_2 = self.clone();
        let self_3 = self.clone();

        hyper::rt::spawn(self.inner.bot_api_client
            .send_chat_action(&SendChatAction { chat_id: ChatId::Id(chat_id), action: ChatAction::Typing }, self.default_timeout)
            .map_err(|err| error!("Error in sending chat action: {}", err))
            .map(|_| ()));

        self.inner.bot_api_client.download_file(&GetFileRequest { file_id }, self.default_timeout)
            .map(move |file|
                match video_or_voice {
                    VideoOrVoice::Video => MediaKind::Mp4(file),
                    VideoOrVoice::Voice => MediaKind::Ogg(file)
                })
            .map_err(Error::from)
            .and_then(move |media_kind| self_1.inner.media_converter.convert(media_kind).map_err(From::from))
            .and_then(move |audio| self_2.inner.recognizer.recognize_audio(audio, &self_2.inner.db.get(chat_id).unwrap_or("ru-RU".to_string())).map_err(From::from))
            .then(move |result| {
                let reply_msg = match result {
                    Ok(recognized) => recognized,
                    Err(err) => {
                        error!("Got error while handling media message {:?}", err);
                        format!("Something went wrong. Please try again later.")
                    }
                };
                let request = SendMessageRequest {
                    reply_to_message_id: Some(msg_id),
                    ..SendMessageRequest::new(
                        ChatId::Id(chat_id),
                        SendMessageKind::Text(
                            SendText::new(reply_msg)))
                };
                self_3.inner.bot_api_client
                    .send_message(&request, self_3.default_timeout)
                    .map_err(From::from)
                    .map(|_| ())
            })
    }
}