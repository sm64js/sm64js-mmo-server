mod chat;
mod date_format;

pub use chat::{ChatError, ChatHistory, ChatHistoryData, ChatMessage, ChatResult, GetChat};

use paperclip::actix::Apiv2Schema;
use serde::{Deserialize, Serialize};
use serde_with::skip_serializing_none;

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
pub struct PlayerInfo {
    pub account_id: i32,
    pub discord_id: Option<String>,
    pub google_id: Option<String>,
    pub socket_id: u32,
    pub ip: String,
    pub real_ip: Option<String>,
    pub level: u32,
    pub name: String,
}
