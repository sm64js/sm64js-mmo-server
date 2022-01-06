use crate::{Message, Rooms};
use actix::Recipient;
use actix_web::web;
use anyhow::Result;
use dashmap::DashMap;
use parking_lot::RwLock;
use sm64js_auth::AuthInfo;
use sm64js_common::{ChatHistoryData, ChatResult};
use sm64js_db::DbPool;
use sm64js_proto::{MarioMsg, SkinData};
use std::{
    collections::HashMap,
    sync::{Arc, Weak},
};

pub type Clients = DashMap<u32, Client>;
pub type Players = HashMap<u32, Arc<RwLock<Player>>>;
pub type WeakPlayers = HashMap<u32, Weak<RwLock<Player>>>;

#[derive(Debug)]
pub struct Client {
    addr: Recipient<Message>,
    auth_info: AuthInfo,
    ip: String,
    data: Option<MarioMsg>,
    socket_id: u32,
    level: Option<u32>,
}

impl Client {
    pub fn new(addr: Recipient<Message>, auth_info: AuthInfo, ip: String, socket_id: u32) -> Self {
        Client {
            addr,
            auth_info,
            ip,
            data: None,
            socket_id,
            level: None,
        }
    }

    pub fn set_data(&mut self, mut data: MarioMsg) {
        data.socket_id = self.socket_id;
        self.data = Some(data);
    }

    pub fn get_pos(&self) -> Option<&Vec<f32>> {
        self.data.as_ref().map(|data| &data.pos)
    }

    pub fn get_account_id(&self) -> i32 {
        self.auth_info.get_account_id()
    }

    pub fn get_discord_id(&self) -> Option<String> {
        self.auth_info.get_discord_id()
    }

    pub fn get_google_id(&self) -> Option<String> {
        self.auth_info.get_google_id()
    }

    pub fn get_ip(&self) -> &String {
        &self.ip
    }

    pub fn get_socket_id(&self) -> u32 {
        self.socket_id
    }

    pub fn set_level(&mut self, level: u32) {
        self.level = Some(level);
    }

    pub fn get_level(&self) -> Option<u32> {
        self.level
    }

    pub fn send(&self, msg: Message) -> Result<()> {
        self.addr.do_send(msg)?;
        Ok(())
    }
}

#[derive(Debug)]
pub struct Player {
    clients: Arc<Clients>,
    socket_id: u32,
    level: u32,
    name: String,
    skin_data: Option<SkinData>,
    skin_data_updated: bool,
}

impl Player {
    pub fn new(clients: Arc<Clients>, socket_id: u32, level: u32, name: String) -> Self {
        Self {
            clients,
            socket_id,
            level,
            name,
            skin_data: None,
            skin_data_updated: false,
        }
    }

    pub fn get_avatar_url(&self) -> Option<String> {
        self.clients.get(&self.socket_id).map(|client| {
            if let Some(discord) = &client.auth_info.0.discord {
                if let Some(avatar) = &discord.account.avatar {
                    format!(
                        "https://cdn.discordapp.com/avatars/{}/{}.png?size=64",
                        discord.account.id, avatar
                    )
                } else {
                    "https://discord.com/assets/2c21aeda16de354ba5334551a883b481.png".to_string()
                }
            } else {
                "https://developers.google.com/identity/images/g-logo.png".to_string()
            }
        })
    }

    pub fn get_socket_id(&self) -> u32 {
        self.socket_id
    }

    pub fn get_account_id(&self) -> Option<i32> {
        self.clients
            .get(&self.socket_id)
            .map(|client| client.get_account_id())
    }

    pub fn get_level(&self) -> u32 {
        self.level
    }

    pub fn get_name(&self) -> &String {
        &self.name
    }

    pub fn get_data(&self) -> Option<MarioMsg> {
        self.clients
            .get(&self.socket_id)
            .and_then(|d| d.data.clone())
    }

    pub fn set_skin_data(&mut self, skin_data: Option<SkinData>) {
        self.skin_data = skin_data;
        self.skin_data_updated = true;
    }

    pub fn is_in_game_admin(&self) -> bool {
        self.clients
            .get(&self.socket_id)
            .map(|client| client.auth_info.is_in_game_admin())
            .unwrap_or_default()
    }

    pub fn send_message(&self, msg: Vec<u8>) -> Result<()> {
        if let Some(client) = self.clients.get(&self.socket_id) {
            client.send(Message::SendData(msg))?;
        }
        Ok(())
    }

    pub fn add_chat_message(
        &mut self,
        pool: web::Data<DbPool>,
        chat_history: ChatHistoryData,
        message: &str,
        rooms: Rooms,
    ) -> ChatResult {
        if let Some(client) = self.clients.get(&self.socket_id) {
            let auth_info = &self.clients.get(&self.socket_id).unwrap().auth_info;

            let conn = pool.get().unwrap();
            let account_info =
                sm64js_db::get_account_info(&conn, auth_info.get_account_id(), true).unwrap();

            chat_history.write().add_message(
                message,
                account_info,
                self.name.clone(),
                rooms
                    .get(&self.level)
                    .map(|room| room.name.clone())
                    .unwrap_or_else(|| "Lobby".to_string()),
                client.ip.to_string(),
            )
        } else {
            ChatResult::NotFound
        }
    }

    pub fn get_skin_data(&self) -> Option<&SkinData> {
        self.skin_data.as_ref()
    }

    pub fn get_updated_skin_data(&mut self) -> Option<SkinData> {
        if self.skin_data_updated {
            self.skin_data_updated = false;
            self.skin_data.clone()
        } else {
            None
        }
    }
}
