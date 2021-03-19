use crate::Message;
use actix::Recipient;
use anyhow::Result;
use dashmap::DashMap;
use parking_lot::RwLock;
use sm64js_auth::AuthInfo;
use sm64js_common::{ChatHistoryData, ChatResult};
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
    real_ip: Option<String>,
    data: Option<MarioMsg>,
    socket_id: u32,
    level: Option<u32>,
}

impl Client {
    pub fn new(
        addr: Recipient<Message>,
        auth_info: AuthInfo,
        ip: String,
        real_ip: Option<String>,
        socket_id: u32,
    ) -> Self {
        let add_real_ip = real_ip
            .clone()
            .map(|real_ip| real_ip != ip)
            .unwrap_or_default();
        Client {
            addr,
            auth_info,
            ip,
            real_ip: if add_real_ip { real_ip } else { None },
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

    pub fn get_ip(&self) -> &String {
        &self.ip
    }

    pub fn get_real_ip(&self) -> Option<&String> {
        self.real_ip.as_ref()
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

    pub fn get_socket_id(&self) -> u32 {
        self.socket_id
    }

    pub fn get_level(&self) -> u32 {
        self.level
    }

    pub fn get_name(&self) -> &String {
        &self.name
    }

    pub fn get_data(&self) -> Option<MarioMsg> {
        self.clients.get(&self.socket_id).unwrap().data.clone()
    }

    pub fn set_skin_data(&mut self, skin_data: Option<SkinData>) {
        self.skin_data = skin_data;
        self.skin_data_updated = true;
    }

    pub fn send_message(&self, msg: Vec<u8>) -> Result<()> {
        self.clients
            .get(&self.socket_id)
            .unwrap()
            .send(Message::SendData(msg))
    }

    pub fn add_chat_message(&mut self, chat_history: ChatHistoryData, message: &str) -> ChatResult {
        if let Some(client) = self.clients.get(&self.socket_id) {
            let auth_info = &self.clients.get(&self.socket_id).unwrap().auth_info;
            chat_history.write().add_message(
                message,
                self.name.clone(),
                auth_info.get_discord_id(),
                auth_info.get_google_id(),
                client.ip.to_string(),
                client.real_ip.clone(),
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
