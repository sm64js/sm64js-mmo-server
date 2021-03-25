use crate::AccountInfo;
use awc::SendClientRequest;
use censor::Censor;
use chrono::{DateTime, Duration, NaiveDateTime, Utc};
use indexmap::IndexMap;
use paperclip::actix::{web, Apiv2Schema};
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use serde_with::skip_serializing_none;
use std::env;

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
abcdefghijklmnopoqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789 ?!@#$%^&*(){}[];:'"\|/,.<>-_=+
🙄😫🤔🔥😌😍🤣❤️😭😂⭐✨🎄🎃🔺🔻🍬🍭🍫
"#;

pub fn sanitize_chat(s: &str) -> String {
    let mut escaped_message = s.to_string();
    escaped_message.retain(|c| ALLOWED_CHARACTERS.contains(c));
    escaped_message
}

#[derive(Serialize)]
struct DiscordChatMessage {
    embed: DiscordRichEmbed,
}

#[derive(Serialize)]
struct DiscordRichEmbed {
    description: String,
    timestamp: NaiveDateTime,
    author: DiscordRichEmbedAuthor,
    footer: DiscordRichEmbedFooter,
}

#[derive(Serialize)]
struct DiscordRichEmbedAuthor {
    name: String,
    url: String,
    icon_url: String,
}

#[derive(Serialize)]
struct DiscordRichEmbedFooter {
    text: String,
}

impl ChatHistory {
    pub fn add_message(
        &mut self,
        message: &str,
        account_info: AccountInfo,
        player_name: String,
        level_name: String,
        ip: String,
        real_ip: Option<String>,
    ) -> ChatResult {
        let escaped_message = sanitize_chat(message);
        let is_escaped = escaped_message != message;

        let censor = Censor::Standard;
        let censored_message = censor.censor(&escaped_message);
        let is_censored = censored_message != escaped_message;

        let date = Utc::now() - Duration::seconds(15);
        let is_spam = self
            .0
            .keys()
            .skip_while(|k| *k < &date)
            .filter(|k| !self.0.get(*k).unwrap().is_spam.unwrap_or_default())
            .count()
            > 5;

        let now = Utc::now();
        let discord_id = account_info.discord.clone().map(|d| d.id);

        self.0.insert(
            now,
            ChatMessage {
                message: message.to_string(),
                timestamp: now.timestamp(),
                date_time: now.naive_utc(),
                player_name: Some(player_name.clone()),
                discord_id,
                google_id: account_info.google.clone().map(|d| d.sub),
                ip: Some(ip),
                real_ip,
                is_escaped: if is_escaped { Some(is_escaped) } else { None },
                is_censored: if is_censored { Some(is_censored) } else { None },
                is_spam: if is_spam { Some(is_spam) } else { None },
            },
        );
        let message = message.to_string();

        if !is_spam && !message.is_empty() {
            actix::spawn(async move {
                Self::send_discord_message(message, player_name, level_name, account_info).await;
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
                if msg.player_name.as_ref() != Some(&player_name) {
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
                msg.real_ip = None;
            }
            res.push(msg);
            if res.len() >= max_messages {
                break;
            }
        }
        res.reverse();
        res
    }

    async fn send_discord_message(
        message: String,
        player_name: String,
        level_name: String,
        account_info: AccountInfo,
    ) {
        let author = DiscordRichEmbedAuthor {
            name: player_name,
            url: format!(
                "{}/api/account?account_id={}",
                env::var("REDIRECT_URI").unwrap(),
                account_info.account.id
            ),
            icon_url: if let Some(discord) = account_info.discord {
                if let Some(avatar) = discord.avatar {
                    let a = format!(
                        "https://cdn.discordapp.com/avatars/{}/{}.png?size=64",
                        discord.id, avatar
                    );
                    dbg!(&a);
                    a
                } else {
                    "https://discord.com/assets/2c21aeda16de354ba5334551a883b481.png".to_string()
                }
            } else {
                "https://developers.google.com/identity/images/g-logo.png".to_string()
            },
        };
        let request: SendClientRequest = awc::Client::default()
            .post(format!(
                "https://discord.com/api/channels/{}/messages",
                "824145108047101974"
            ))
            .header(
                awc::http::header::AUTHORIZATION,
                format!("{} {}", "Bot", env::var("DISCORD_BOT_TOKEN").unwrap(),),
            )
            .send_json(&DiscordChatMessage {
                embed: DiscordRichEmbed {
                    description: message,
                    timestamp: Utc::now().naive_utc(),
                    author,
                    footer: DiscordRichEmbedFooter {
                        text: format!("#{} - {}", account_info.account.id, level_name),
                    },
                },
            });

        let response = request.await.unwrap();
        if !response.status().is_success() {
            eprintln!("send_discord_message failed: {:?}", response);
        };
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
    discord_id: Option<String>,
    google_id: Option<String>,
    ip: Option<String>,
    real_ip: Option<String>,
    is_escaped: Option<bool>,
    is_censored: Option<bool>,
    is_spam: Option<bool>,
}

pub enum ChatResult {
    Ok((String, bool)),
    Err(ChatError),
    NotFound,
}

pub enum ChatError {
    Spam,
}
