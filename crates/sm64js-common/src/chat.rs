use crate::AccountInfo;
use chrono::{DateTime, Duration, NaiveDateTime, Utc};
use indexmap::IndexMap;
use paperclip::actix::{web, Apiv2Schema};
use parking_lot::RwLock;
use rustrict::CensorStr;
use serde::{Deserialize, Serialize};
use serde_with::skip_serializing_none;
use sm64js_env::REDIRECT_URI;

#[derive(Apiv2Schema, Debug, Default, Deserialize)]
pub struct GetChat {
    /// Format must be given as %Y-%m-%d %H:%M:%S
    #[serde(
        deserialize_with = "crate::date_format::deserialize_opt",
        default = "crate::date_format::empty"
    )]
    pub from: Option<DateTime<Utc>>,
    /// Format must be given as %Y-%m-%d %H:%M:%S
    #[serde(
        deserialize_with = "crate::date_format::deserialize_opt",
        default = "crate::date_format::empty"
    )]
    pub to: Option<DateTime<Utc>>,
    pub limit: Option<u32>,
    pub player_name: Option<String>,
    pub discord_id: Option<String>,
    pub google_id: Option<String>,
}

pub type ChatHistoryData = web::Data<RwLock<ChatHistory>>;

#[derive(Debug)]
pub struct ChatHistory(IndexMap<DateTime<Utc>, ChatMessage>);

impl Default for ChatHistory {
    fn default() -> Self {
        Self(IndexMap::new())
    }
}

const ALLOWED_CHARACTERS: &str = r#"
abcdefghijklmnopoqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789 ?!@#$%^&*(){}[];:'"\|/,.<>-_=+`
😂🤣🤔🤨🙄😭😎🥶😤👍👎💀🗿🔥🎄🎃🔺🔻🤡🎪🎶🎵
"#;

pub fn sanitize_chat(s: &str) -> String {
    let mut escaped_message = s.to_string();
    escaped_message.retain(|c| ALLOWED_CHARACTERS.contains(c));
    escaped_message
}

impl ChatHistory {
    pub fn add_message(
        &mut self,
        message: &str,
        account_info: AccountInfo,
        player_name: String,
        level_name: String,
        ip: String,
    ) -> ChatResult {
        let escaped_message = sanitize_chat(message);
        let is_escaped = escaped_message != message;
        let censored_message = escaped_message.censor();
        let is_censored = censored_message != escaped_message;

        let date = Utc::now() - Duration::seconds(15);
        let account_id = account_info.account.id;
        let is_spam = self
            .0
            .iter()
            .skip_while(|(k, _)| *k < &date)
            .filter(|(k, v)| {
                v.account_id == account_id && !self.0.get(*k).unwrap().is_spam.unwrap_or_default()
            })
            .count()
            >= 3;

        let date = Utc::now() - Duration::seconds(60);
        let is_excessive_spam = self
            .0
            .iter()
            .skip_while(|(k, _)| *k < &date)
            .filter(|(_, v)| v.account_id == account_id)
            .count()
            >= 30;

        let is_screaming = if message.len() > 5 {
            let alphabetic_count = message.chars().filter(|c| c.is_ascii_alphabetic()).count();
            let screaming_count = message
                .chars()
                .filter(|c| c.is_ascii_alphabetic() && c.is_ascii_uppercase())
                .count();
            (screaming_count as f32 / alphabetic_count as f32) > 0.7
        } else {
            false
        };

        let now = Utc::now();
        let discord_id = account_info.discord.clone().map(|d| d.id);

        self.0.insert(
            now,
            ChatMessage {
                message: message.to_string(),
                timestamp: now.timestamp(),
                date_time: now.naive_utc(),
                player_name: Some(player_name.clone()),
                account_id,
                discord_id,
                google_id: account_info.google.clone().map(|d| d.sub),
                ip: Some(ip),
                is_escaped: if is_escaped { Some(is_escaped) } else { None },
                is_censored: if is_censored { Some(is_censored) } else { None },
                is_spam: if is_spam { Some(is_spam) } else { None },
                is_excessive_spam: if is_excessive_spam {
                    Some(is_excessive_spam)
                } else {
                    None
                },
                is_screaming: if is_screaming {
                    Some(is_screaming)
                } else {
                    None
                },
            },
        );

        if is_excessive_spam {
            return ChatResult::Err(ChatError::ExcessiveSpam);
        } else if is_spam {
            return ChatResult::Err(ChatError::Spam);
        } else if is_screaming {
            return ChatResult::Err(ChatError::Screaming);
        }

        let message = message.to_string();
        if !is_spam && !message.is_empty() {
            let censored_message = censored_message.clone();
            actix::spawn(async move {
                Self::send_discord_chat_message(
                    censored_message,
                    player_name,
                    level_name,
                    account_info,
                )
                .await;
            });
        }

        ChatResult::Ok((censored_message, is_spam))
    }

    pub fn get_messages(
        &self,
        query: GetChat,
        with_player_info: bool,
        with_ip: bool,
    ) -> Vec<ChatMessage> {
        let max_messages = query.limit.unwrap_or(100) as usize;
        let mut res: Vec<ChatMessage> = vec![];
        let mut reached = false;
        let mut keys = self.0.keys().clone();
        while let Some(key) = keys.next_back() {
            if !reached {
                if let Some(to) = query.to {
                    if *key >= to {
                        continue;
                    }
                } else {
                    reached = true
                }
            }
            if reached {
                if let Some(from) = query.from {
                    if *key <= from {
                        break;
                    }
                }
            }
            let mut msg = self.0.get(key).unwrap().clone();
            if let Some(player_name) = &query.player_name {
                if msg.player_name.as_ref() != Some(player_name) {
                    continue;
                }
            }
            if let (Some(_), Some(_)) = (&query.discord_id, &query.google_id) {
                //// a query should never have a discord id and a google id.
                //// throw an error here maybe
                continue;
            }
            if let Some(id1) = &query.discord_id {
                match &msg.discord_id {
                    Some(id2) if id1 == id2 => {}
                    _ => continue,
                };
            }
            if let Some(id1) = &query.google_id {
                match &msg.google_id {
                    Some(id2) if id1 == id2 => {}
                    _ => continue,
                };
            }
            if !with_player_info {
                msg.player_name = None;
                msg.discord_id = None;
                msg.google_id = None;
            }
            if !with_ip {
                msg.ip = None;
            }
            res.push(msg);
            if res.len() >= max_messages {
                break;
            }
        }
        res.reverse();
        res
    }

    async fn send_discord_chat_message(
        mut message: String,
        player_name: String,
        level_name: String,
        account_info: AccountInfo,
    ) {
        let author = super::DiscordRichEmbedAuthor {
            name: player_name,
            url: Some(format!(
                "{}/api/account?account_id={}",
                REDIRECT_URI.get().unwrap(),
                account_info.account.id
            )),
            icon_url: Some(if let Some(discord) = account_info.discord {
                if let Some(avatar) = discord.avatar {
                    format!(
                        "https://cdn.discordapp.com/avatars/{}/{}.png?size=64",
                        discord.id, avatar
                    )
                } else {
                    "https://discord.com/assets/2c21aeda16de354ba5334551a883b481.png".to_string()
                }
            } else {
                "https://developers.google.com/identity/images/g-logo.png".to_string()
            }),
        };
        let footer = Some(super::DiscordRichEmbedFooter {
            text: format!("#{} - {}", account_info.account.id, level_name),
        });
        message = message.replace('*', r"\*").replace('_', r"\_");
        let is_code = message != "1337";
        if is_code {
            super::send_discord_message("824145108047101974", None, message, None, author, footer)
                .await;
        }
    }
}

#[skip_serializing_none]
#[derive(Apiv2Schema, Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ChatMessage {
    message: String,
    timestamp: i64,
    date_time: NaiveDateTime,
    player_name: Option<String>,
    account_id: i32,
    discord_id: Option<String>,
    google_id: Option<String>,
    ip: Option<String>,
    is_escaped: Option<bool>,
    is_censored: Option<bool>,
    is_spam: Option<bool>,
    is_excessive_spam: Option<bool>,
    is_screaming: Option<bool>,
}

pub enum ChatResult {
    Ok((String, bool)),
    Err(ChatError),
    NotFound,
}

pub enum ChatError {
    Spam,
    ExcessiveSpam,
    Screaming,
}
