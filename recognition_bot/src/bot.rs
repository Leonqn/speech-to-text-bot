use futures::StreamExt;
use log::error;
use rutebot::client::Rutebot;
use rutebot::requests::ChatId;
use rutebot::requests::GetFile;
use rutebot::requests::SendMessage;
use rutebot::requests::UpdateKind;
use rutebot::requests::{ChatAction, SendChatAction};

use rutebot::responses::MessageEntityValue;
use rutebot::responses::{Message, Update};

use crate::media_converter::*;

use crate::recognizer::*;
use anyhow::Context;

#[derive(Clone)]
pub struct Bot {
    bot_api_client: Rutebot,
    recognizer: Recognizer,
}

enum VideoOrVoice {
    Video,
    Voice,
}

impl Bot {
    pub fn new(bot_api_client: Rutebot, recognizer: Recognizer) -> Self {
        Self {
            bot_api_client,
            recognizer,
        }
    }

    pub async fn start_bot(&self) -> anyhow::Result<()> {
        let allowed_updates = Some(vec![UpdateKind::Message]);
        let mut updates_stream = self
            .bot_api_client
            .incoming_updates(None, allowed_updates)
            .boxed();

        while let Some(update) = updates_stream.next().await {
            match update {
                Ok(update) => {
                    let bot = self.clone();
                    tokio::task::spawn(async move {
                        if let Err(err) = bot.handle_messages(update).await {
                            error!("Error while handling message {:?}", err)
                        }
                    });
                }
                Err(err) => {
                    error!("Error in getting updates {:?} ", err);
                }
            }
        }
        Ok(())
    }

    async fn handle_messages(&self, update: Update) -> anyhow::Result<()> {
        match update {
            Update {
                message:
                    Some(Message {
                        chat,
                        text: Some(text),
                        entities: Some(entities),
                        ..
                    }),
                ..
            } => {
                if let Some(bot_command) = entities
                    .first()
                    .and_then(|x| x.extract_value(&text))
                    .and_then(|x| match x {
                        MessageEntityValue::BotCommand(x) => Some(x),
                        _ => None,
                    })
                {
                    if bot_command.starts_with("/help") {
                        self.handle_help(chat.id).await.context("help")?;
                    }
                }
            }

            Update {
                message:
                    Some(Message {
                        message_id,
                        chat,
                        voice: Some(voice),
                        ..
                    }),
                ..
            } => {
                self.handle_media_message(chat.id, message_id, VideoOrVoice::Voice, voice.file_id)
                    .await
                    .context("voice")?;
            }

            Update {
                message:
                    Some(Message {
                        message_id,
                        chat,
                        video_note: Some(video_note),
                        ..
                    }),
                ..
            } => {
                self.handle_media_message(
                    chat.id,
                    message_id,
                    VideoOrVoice::Video,
                    video_note.file_id,
                )
                .await
                .context("video_note")?;
            }
            _ => {}
        }
        Ok(())
    }

    async fn handle_help(&self, chat_id: i64) -> anyhow::Result<()> {
        let help_msg = "Hello! I can convert voice and video note messages to text. \
                        You can forward messages to me or add me to chat. \
                        Default and the only language is russian";

        self.bot_api_client
            .prepare_api_request(SendMessage::new(chat_id, help_msg))
            .send()
            .await?;
        Ok(())
    }

    async fn handle_media_message(
        &self,
        chat_id: i64,
        msg_id: i64,
        video_or_voice: VideoOrVoice,
        file_id: String,
    ) -> anyhow::Result<()> {
        let recognized = async {
            self.bot_api_client
                .prepare_api_request(SendChatAction {
                    chat_id: ChatId::Id(chat_id),
                    action: ChatAction::Typing,
                })
                .send()
                .await?;

            let file_handle = self
                .bot_api_client
                .prepare_api_request(GetFile::new(&file_id))
                .send()
                .await?;

            let file_bytes = self
                .bot_api_client
                .download_file(file_handle.file_path.as_ref().map_or("", String::as_str))
                .await?;
            let media_kind = match video_or_voice {
                VideoOrVoice::Video => MediaKind::Mp4(file_bytes),
                VideoOrVoice::Voice => MediaKind::Ogg(file_bytes),
            };
            let converted = tokio::task::spawn_blocking(move || convert(media_kind)).await??;
            self.recognizer.recognize_audio(converted).await
        }
        .await;

        let reply_msg = match recognized {
            Ok(ref recognized) => recognized,
            Err(err) => {
                error!("Got error while handling media message {:?}", err);
                "Something went wrong. Please try again later."
            }
        };
        let request = SendMessage::new_reply(chat_id, reply_msg, msg_id);
        self.bot_api_client
            .prepare_api_request(request)
            .send()
            .await?;
        Ok(())
    }
}
