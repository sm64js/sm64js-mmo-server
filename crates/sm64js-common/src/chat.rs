use censor::Censor;
use chrono::{DateTime, Duration, Utc};
use indexmap::IndexMap;
use paperclip::actix::{web, Apiv2Schema};
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use serde_with::skip_serializing_none;
use v_htmlescape::escape;

#[derive(Apiv2Schema, Debug, Deserialize)]
pub struct GetChat {
    /// Format must be given as %Y-%m-%d %H:%M:%S
    #[serde(
        deserialize_with = "crate::date_format::deserialize_opt",
        default = "crate::date_format::empty"
    )]
    from: Option<DateTime<Utc>>,
    /// Format must be given as %Y-%m-%d %H:%M:%S
    #[serde(
        deserialize_with = "crate::date_format::deserialize_opt",
        default = "crate::date_format::empty"
    )]
    to: Option<DateTime<Utc>>,
    player_name: Option<String>,
    discord_id: Option<String>,
    google_id: Option<String>,
}

pub type ChatHistoryData = web::Data<RwLock<ChatHistory>>;

#[derive(Debug)]
pub struct ChatHistory(IndexMap<DateTime<Utc>, ChatMessage>);

impl Default for ChatHistory {
    fn default() -> Self {
        Self(IndexMap::new())
    }
}

impl ChatHistory {
    pub fn add_message(
        &mut self,
        message: &str,
        player_name: String,
        discord_id: Option<String>,
        google_id: Option<String>,
        ip: Option<String>,
        real_ip: Option<String>,
    ) -> ChatResult {
        let escaped_message = format!("{}", escape(message));
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

        self.0.insert(
            now,
            ChatMessage {
                message: message.to_string(),
                timestamp: now.timestamp(),
                player_name,
                discord_id,
                google_id,
                ip,
                real_ip,
                is_escaped: if is_escaped { Some(is_escaped) } else { None },
                is_censored: if is_censored { Some(is_censored) } else { None },
                is_spam: if is_spam { Some(is_spam) } else { None },
            },
        );

        ChatResult::Ok(censored_message)
    }

    pub fn get_messages(&self, query: GetChat, with_ip: bool) -> Vec<ChatMessage> {
        let max_messages = 100;
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
                if &msg.player_name != player_name {
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
}

#[skip_serializing_none]
#[derive(Apiv2Schema, Clone, Debug, Deserialize, Serialize)]
pub struct ChatMessage {
    message: String,
    timestamp: i64,
    player_name: String,
    discord_id: Option<String>,
    google_id: Option<String>,
    ip: Option<String>,
    real_ip: Option<String>,
    is_escaped: Option<bool>,
    is_censored: Option<bool>,
    is_spam: Option<bool>,
}

pub enum ChatResult {
    Ok(String),
    Err(ChatError),
}

pub enum ChatError {
    Spam,
}
