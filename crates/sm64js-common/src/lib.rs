mod chat;
mod date_format;

pub use chat::{
    sanitize_chat, ChatError, ChatHistory, ChatHistoryData, ChatMessage, ChatResult, GetChat,
};

use awc::SendClientRequest;
use chrono::{NaiveDateTime, Utc};
use paperclip::actix::{web::HttpRequest, Apiv2Schema};
use prost::Message as ProstMessage;
use serde::{Deserialize, Serialize};
use serde_with::skip_serializing_none;
use sm64js_env::DISCORD_BOT_TOKEN;
use sm64js_proto::{root_msg, sm64_js_msg, RootMsg, Sm64JsMsg};
use std::time::Duration;

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
    pub level: u32,
    pub name: String,
    pub chat: Option<Vec<chat::ChatMessage>>,
}

#[skip_serializing_none]
#[derive(Apiv2Schema, Clone, Debug, Default, Serialize)]
pub struct AccountInfo {
    pub account: Account,
    pub discord: Option<DiscordAccount>,
    pub google: Option<GoogleAccount>,
}

#[skip_serializing_none]
#[derive(Apiv2Schema, Clone, Debug, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Account {
    pub id: i32,
    pub username: Option<String>,
    pub last_ip: Option<String>,
    pub is_banned: Option<bool>,
    pub banned_until: Option<NaiveDateTime>,
    pub ban_reason: Option<String>,
    pub is_muted: Option<bool>,
    pub muted_until: Option<NaiveDateTime>,
    pub mute_reason: Option<String>,
}

#[skip_serializing_none]
#[derive(Apiv2Schema, Clone, Debug, Serialize)]
pub struct DiscordAccount {
    pub id: String,
    pub username: String,
    pub discriminator: String,
    pub avatar: Option<String>,
    pub mfa_enabled: Option<bool>,
    pub locale: Option<String>,
    pub flags: Option<i32>,
    pub premium_type: Option<i16>,
    pub public_flags: Option<i32>,
    pub nick: Option<String>,
    pub roles: Vec<String>,
    pub joined_at: String,
    pub premium_since: Option<String>,
    pub deaf: bool,
    pub mute: bool,
}

#[derive(Apiv2Schema, Clone, Debug, Serialize)]
pub struct GoogleAccount {
    pub sub: String,
}

#[derive(Serialize)]
struct DiscordChatMessage {
    embed: DiscordRichEmbed,
}

#[derive(Serialize)]
struct DiscordRichEmbed {
    description: String,
    fields: Option<Vec<DiscordRichEmbedField>>,
    timestamp: NaiveDateTime,
    author: DiscordRichEmbedAuthor,
    footer: Option<DiscordRichEmbedFooter>,
}

#[derive(Serialize)]
pub struct DiscordRichEmbedField {
    pub name: String,
    pub value: String,
}

#[derive(Serialize)]
pub struct DiscordRichEmbedAuthor {
    pub name: String,
    pub url: Option<String>,
    pub icon_url: Option<String>,
}

#[derive(Serialize)]
pub struct DiscordRichEmbedFooter {
    pub text: String,
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

pub async fn send_discord_message(
    channel_id: &str,
    message_id: Option<&str>,
    description: String,
    fields: Option<Vec<DiscordRichEmbedField>>,
    author: DiscordRichEmbedAuthor,
    footer: Option<DiscordRichEmbedFooter>,
) {
    let client = awc::Client::builder()
        .timeout(Duration::from_secs(15))
        .finish();
    let request = if let Some(message_id) = message_id {
        client.patch(format!(
            "https://discord.com/api/channels/{}/messages/{}",
            channel_id, message_id
        ))
    } else {
        client.post(format!(
            "https://discord.com/api/channels/{}/messages",
            channel_id
        ))
    };
    let request: SendClientRequest = request
        .header(
            awc::http::header::AUTHORIZATION,
            format!("{} {}", "Bot", DISCORD_BOT_TOKEN.get().unwrap(),),
        )
        .send_json(&DiscordChatMessage {
            embed: DiscordRichEmbed {
                description,
                fields,
                timestamp: Utc::now().naive_utc(),
                author,
                footer,
            },
        });

    match request.await {
        Ok(res) => {
            if !res.status().is_success() {
                eprintln!("send_discord_message failed: {:?}", res);
            };
        }
        Err(err) => {
            eprintln!("{:?}", err)
        }
    };
}

pub fn get_ip_from_req(req: &HttpRequest) -> Option<String> {
    if let Some(x_real_ip) = req
        .headers()
        .get("x-real-ip")
        .and_then(|ip| ip.to_str().ok())
    {
        Some(x_real_ip.to_string())
    } else if let Some(x_forwarded_for) = req
        .headers()
        .get("x-forwarded-for")
        .and_then(|ip| ip.to_str().ok())
    {
        Some(x_forwarded_for.to_string())
    } else {
        req.peer_addr().map(|ip| ip.ip().to_string())
    }
}
