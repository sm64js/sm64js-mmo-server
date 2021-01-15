use std::sync::Arc;

use censor::Censor;
use chrono::{prelude::*, Duration};
use indexmap::IndexMap;
use paperclip::actix::{api_v2_operation, web, Apiv2Schema};
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use v_htmlescape::escape;

#[api_v2_operation(tags(Chat))]
pub async fn get_chat(
    query: web::Query<GetChat>,
    chat_history: web::Data<ChatHistoryData>,
) -> web::Json<Vec<ChatMessage>> {
    web::Json(chat_history.read().get_messages(query.into_inner()))
}

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
}

pub type ChatHistoryData = Arc<RwLock<ChatHistory>>;

#[derive(Debug)]
pub struct ChatHistory(IndexMap<DateTime<Utc>, ChatMessage>);

impl ChatHistory {
    pub fn new() -> Self {
        Self(IndexMap::new())
    }

    pub fn add_message(
        &mut self,
        message: &String,
        player_name: String,
        ip: Option<String>,
        real_ip: Option<String>,
    ) -> ChatResult {
        let escaped_message = format!("{}", escape(message));
        let is_escaped = &escaped_message != message;

        let censor = Censor::Standard;
        let censored_message = censor.censor(&escaped_message);
        let is_censored = censored_message != escaped_message;

        let date = Utc::now() - Duration::seconds(15);
        let is_spam = self
            .0
            .keys()
            .skip_while(|k| *k < &date)
            .filter(|k| !self.0.get(*k).unwrap().is_spam)
            .count()
            > 5;

        self.0.insert(
            Utc::now(),
            ChatMessage {
                message: message.clone(),
                player_name,
                ip,
                real_ip,
                is_escaped,
                is_censored,
                is_spam,
            },
        );

        ChatResult::Ok(censored_message)
    }

    fn get_messages(&self, query: GetChat) -> Vec<ChatMessage> {
        let max_messages = 100;
        let mut res: Vec<ChatMessage> = vec![];
        let mut reached = false;
        while let Some(key) = self.0.keys().next_back() {
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
            let msg = self.0.get(key).unwrap().clone();
            if let Some(player_name) = &query.player_name {
                if &msg.player_name != player_name {
                    continue;
                }
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

#[derive(Apiv2Schema, Clone, Debug, Deserialize, Serialize)]
pub struct ChatMessage {
    message: String,
    player_name: String,
    ip: Option<String>,
    real_ip: Option<String>,
    is_escaped: bool,
    is_censored: bool,
    is_spam: bool,
}

pub enum ChatResult {
    Ok(String),
    Err(ChatError),
}

pub enum ChatError {
    Spam,
}
