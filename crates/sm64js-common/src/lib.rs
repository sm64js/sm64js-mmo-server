mod chat;
mod date_format;

pub use chat::{
    sanitize_chat, ChatError, ChatHistory, ChatHistoryData, ChatMessage, ChatResult, GetChat,
};

use paperclip::actix::Apiv2Schema;
use prost::Message as ProstMessage;
use serde::{Deserialize, Serialize};
use serde_with::skip_serializing_none;
use sm64js_proto::{root_msg, sm64_js_msg, RootMsg, Sm64JsMsg};

#[derive(Clone, Debug, Deserialize)]
pub struct DiscordUser {
    pub id: String,
    pub username: String,
    pub discriminator: String,
    pub avatar: Option<String>,
    pub mfa_enabled: Option<bool>,
    pub locale: Option<String>,
    pub flags: Option<i32>,
    pub premium_type: Option<i16>,
    pub public_flags: Option<i32>,
}

#[derive(Clone, Debug, Deserialize)]
pub struct DiscordGuildMember {
    pub nick: Option<String>,
    pub roles: Vec<String>,
    pub joined_at: String,
    pub premium_since: Option<String>,
    pub deaf: bool,
    pub mute: bool,
}

#[skip_serializing_none]
#[derive(Apiv2Schema, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PlayerInfo {
    pub account_id: i32,
    pub discord_id: Option<String>,
    pub google_id: Option<String>,
    pub ip: String,
    pub real_ip: Option<String>,
    pub level: u32,
    pub name: String,
    pub chat: Option<Vec<chat::ChatMessage>>,
}

pub fn create_uncompressed_msg(msg: sm64_js_msg::Message) -> Vec<u8> {
    let root_msg = RootMsg {
        message: Some(root_msg::Message::UncompressedSm64jsMsg(Sm64JsMsg {
            message: Some(msg),
        })),
    };
    let mut msg = vec![];
    root_msg.encode(&mut msg).unwrap();

    msg
}
