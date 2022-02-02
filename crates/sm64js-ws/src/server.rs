use crate::{Client, Clients, Player, Players, Rooms};
use actix::{prelude::*, Recipient};
use actix_web::web;
use anyhow::Result;
use chrono::{Duration, Utc};
use dashmap::{mapref::one::Ref, DashMap};
use humantime::format_duration;
use once_cell::sync::Lazy;
use parking_lot::{Mutex, RwLock};
use prost::Message as ProstMessage;
use rand::{self, Rng};
use rustrict::CensorStr;
use sm64js_auth::{AuthInfo, Permission};
use sm64js_common::{
    sanitize_chat, send_discord_message, ChatError, ChatHistoryData, ChatResult, GetChat,
    PlayerInfo,
};
use sm64js_db::DbPool;
use sm64js_proto::{
    root_msg, sm64_js_msg, AnnouncementMsg, AttackMsg, ChatMsg, GrabFlagMsg, JoinGameMsg, MarioMsg,
    RootMsg, SkinMsg, Sm64JsMsg,
};
use std::{collections::HashMap, sync::Arc, time};

pub static PRIVILEGED_COMMANDS: Lazy<Mutex<HashMap<&str, Permission>>> = Lazy::new(|| {
    let mut m = HashMap::new();
    m.insert("ANNOUNCEMENT", Permission::SendAnnouncement);
    Mutex::new(m)
});

#[derive(Message)]
#[rtype(result = "()")]
pub enum Message {
    SendData(Vec<u8>),
    Kick,
}

pub struct Sm64JsServer {
    pool: web::Data<DbPool>,
    clients: Arc<Clients>,
    players: Players,
    rooms: Rooms,
    chat_history: ChatHistoryData,
}

impl Actor for Sm64JsServer {
    type Context = Context<Self>;

    fn started(&mut self, _: &mut Self::Context) {}
}

#[derive(Message)]
#[rtype(u32)]
pub struct Connect {
    pub addr: Recipient<Message>,
    pub auth_info: AuthInfo,
    pub ip: String,
}

impl Handler<Connect> for Sm64JsServer {
    type Result = u32;

    fn handle(&mut self, msg: Connect, _: &mut Context<Self>) -> Self::Result {
        if let Some(client) = self
            .clients
            .iter()
            .find(|client| client.get_account_id() == msg.auth_info.get_account_id())
        {
            match client.send(Message::Kick) {
                Ok(_) => {}
                Err(err) => {
                    eprintln!("{:?}", err)
                }
            };
        }

        let socket_id = rand::thread_rng().gen::<u32>();
        let client = Client::new(msg.addr, msg.auth_info, msg.ip, socket_id);

        self.clients.insert(socket_id, client);
        socket_id
    }
}

#[derive(Message)]
#[rtype(result = "()")]
pub struct Disconnect {
    pub socket_id: u32,
}

impl Handler<Disconnect> for Sm64JsServer {
    type Result = ();

    fn handle(&mut self, msg: Disconnect, _: &mut Context<Self>) {
        self.clients.remove(&msg.socket_id);
        if let Some(player) = self.players.remove(&msg.socket_id) {
            let level_id = player.read().get_level();
            if let Some(mut room) = self.rooms.get_mut(&level_id) {
                room.drop_flag_if_holding(msg.socket_id);
            }
        }
    }
}

#[derive(Message)]
#[rtype(result = "()")]
pub struct SetData {
    pub socket_id: u32,
    pub data: MarioMsg,
}

impl Handler<SetData> for Sm64JsServer {
    type Result = ();

    fn handle(&mut self, msg: SetData, _: &mut Context<Self>) {
        if let Some(mut client) = self.clients.get_mut(&msg.socket_id) {
            client.set_data(msg.data);
        }
    }
}

#[derive(Message)]
#[rtype(result = "()")]
pub struct SendAttack {
    pub socket_id: u32,
    pub attack_msg: AttackMsg,
}

impl Handler<SendAttack> for Sm64JsServer {
    type Result = ();

    fn handle(&mut self, send_attack: SendAttack, _: &mut Context<Self>) {
        let socket_id = send_attack.socket_id;
        let attack_msg = send_attack.attack_msg;
        if let Some((level_id, attacker_pos)) = self
            .clients
            .get(&socket_id)
            .and_then(|client| try { (client.get_level()?, client.get_pos()?.clone()) })
        {
            if let Some(room) = self.rooms.get(&level_id) {
                let flag_id = attack_msg.flag_id as usize;
                let target_id = attack_msg.target_socket_id;
                room.process_attack(flag_id, attacker_pos, target_id);
            }
        }
    }
}

#[derive(Message)]
#[rtype(result = "()")]
pub struct SendGrabFlag {
    pub socket_id: u32,
    pub grab_flag_msg: GrabFlagMsg,
}

impl Handler<SendGrabFlag> for Sm64JsServer {
    type Result = ();

    fn handle(&mut self, send_grab: SendGrabFlag, _: &mut Context<Self>) {
        let socket_id = send_grab.socket_id;
        let grab_flag_msg = send_grab.grab_flag_msg;
        if let Some(level_id) = self
            .clients
            .get(&socket_id)
            .and_then(|client| client.get_level())
        {
            if let Some(room) = self.rooms.get(&level_id) {
                let flag_id = grab_flag_msg.flag_id as usize;
                let pos = grab_flag_msg.pos;
                room.process_grab_flag(flag_id, pos, socket_id);
            }
        }
    }
}

#[derive(Message)]
#[rtype(result = "Option<Vec<u8>>")]
pub struct SendChat {
    pub socket_id: u32,
    pub chat_msg: ChatMsg,
    pub auth_info: AuthInfo,
}

impl Handler<SendChat> for Sm64JsServer {
    type Result = Option<Vec<u8>>;

    fn handle(&mut self, send_chat: SendChat, _: &mut Context<Self>) -> Self::Result {
        let socket_id = send_chat.socket_id;
        let chat_msg = send_chat.chat_msg;
        let auth_info = send_chat.auth_info;

        let msg = if chat_msg.message.starts_with('/') {
            Ok(Self::handle_command(chat_msg, auth_info))
        } else if let Some(player) = self.players.get(&socket_id) {
            self.handle_chat(player, socket_id, chat_msg, auth_info)
        } else {
            Ok(None)
        };

        match msg {
            Ok(Some(msg)) => {
                let level = self.clients.get(&socket_id)?.get_level()?;
                let room = self.rooms.get(&level)?;
                room.broadcast_message(&msg);
                None
            }
            Ok(None) => None,
            Err(msg) => Some(msg),
        }
    }
}

#[derive(Message)]
#[rtype(result = "()")]
pub struct SendSkin {
    pub socket_id: u32,
    pub skin_msg: SkinMsg,
}

impl Handler<SendSkin> for Sm64JsServer {
    type Result = ();

    fn handle(&mut self, send_skin: SendSkin, _: &mut Context<Self>) {
        let socket_id = send_skin.socket_id;
        let skin_msg = send_skin.skin_msg;
        if let Some(player) = self.players.get_mut(&socket_id) {
            player.write().set_skin_data(skin_msg.skin_data);
        }
    }
}

#[derive(Message)]
#[rtype(result = "Option<JoinGameAccepted>")]
pub struct SendJoinGame {
    pub socket_id: u32,
    pub join_game_msg: JoinGameMsg,
    pub auth_info: AuthInfo,
}

impl Handler<SendJoinGame> for Sm64JsServer {
    type Result = Option<JoinGameAccepted>;

    fn handle(&mut self, send_join_game: SendJoinGame, _: &mut Context<Self>) -> Self::Result {
        let join_game_msg = send_join_game.join_game_msg;
        let socket_id = send_join_game.socket_id;
        let auth_info = send_join_game.auth_info;
        if let Some(mut room) = self.rooms.get_mut(&join_game_msg.level) {
            if room.has_player(socket_id) {
                None
            } else {
                let name = if join_game_msg.use_discord_name {
                    auth_info.get_discord_username()?
                } else {
                    if !Self::is_name_valid(&join_game_msg.name) {
                        return None;
                    }
                    join_game_msg.name
                };
                let level = join_game_msg.level;
                if level == 0 {
                    // TODO is custom game
                    None
                } else {
                    let player = Arc::new(RwLock::new(Player::new(
                        self.clients.clone(),
                        socket_id,
                        level,
                        name.clone(),
                    )));
                    // TODO check duplicate custom name
                    room.add_player(socket_id, Arc::downgrade(&player));
                    if let Some(mut client) = self.clients.get_mut(&socket_id) {
                        client.set_level(level);
                    }
                    self.players.insert(socket_id, player);
                    Some(JoinGameAccepted { level, name })
                }
            }
        } else {
            None
        }
    }
}

#[derive(Message)]
#[rtype(result = "()")]
pub struct BroadcastLobbyData {
    pub data: Vec<u8>,
}

impl Handler<BroadcastLobbyData> for Sm64JsServer {
    type Result = ();

    fn handle(&mut self, msg: BroadcastLobbyData, _: &mut Context<Self>) {
        self.clients
            .iter()
            .filter(|client| client.get_level().is_none())
            .for_each(|client| {
                if let Err(err) = client.send(Message::SendData(msg.data.clone())) {
                    eprintln!("{:?}", err);
                }
            });
    }
}

#[derive(Message)]
#[rtype(result = "Result<()>")]
pub struct KickClientByAccountId {
    pub account_id: i32,
}

impl Handler<KickClientByAccountId> for Sm64JsServer {
    type Result = Result<()>;

    fn handle(&mut self, msg: KickClientByAccountId, _: &mut Context<Self>) -> Self::Result {
        let account_id = msg.account_id;
        let socket_id = {
            if let Some(client) = self.get_client_by_account_id(account_id) {
                client.send(Message::Kick)?;
                Some(client.get_socket_id())
            } else {
                None
            }
        };
        if let Some(socket_id) = socket_id {
            self.clients.remove(&socket_id);
            self.players.remove(&socket_id);
        }
        Ok(())
    }
}

#[derive(Message)]
#[rtype(result = "Result<()>")]
pub struct KickClientByIpAddr {
    pub ip: String,
}

impl Handler<KickClientByIpAddr> for Sm64JsServer {
    type Result = Result<()>;

    fn handle(&mut self, msg: KickClientByIpAddr, _: &mut Context<Self>) -> Self::Result {
        let ip = msg.ip;
        let socket_id = {
            if let Some(client) = self.get_client_by_ip_addr(ip) {
                client.send(Message::Kick)?;
                Some(client.get_socket_id())
            } else {
                None
            }
        };
        if let Some(socket_id) = socket_id {
            self.clients.remove(&socket_id);
            self.players.remove(&socket_id);
        }
        Ok(())
    }
}

#[derive(Message)]
#[rtype(result = "Vec<PlayerInfo>")]
pub struct GetPlayers {
    pub include_chat: Option<u32>,
}

impl Handler<GetPlayers> for Sm64JsServer {
    type Result = MessageResult<GetPlayers>;

    fn handle(&mut self, msg: GetPlayers, _: &mut Context<Self>) -> Self::Result {
        let include_chat = msg.include_chat;
        MessageResult(
            self.players
                .iter()
                .filter_map(|player| {
                    let player = player.1.read();
                    let client = self.clients.get(&player.get_socket_id())?;
                    let discord_id = client.get_discord_id();
                    let google_id = client.get_google_id();
                    let chat = if let Some(include_chat) = include_chat {
                        Some(self.chat_history.read().get_messages(
                            GetChat {
                                limit: Some(include_chat),
                                discord_id: discord_id.clone(),
                                google_id: google_id.clone(),
                                ..Default::default()
                            },
                            false,
                            false,
                        ))
                    } else {
                        None
                    };
                    Some(PlayerInfo {
                        account_id: client.get_account_id(),
                        discord_id,
                        google_id,
                        ip: client.get_ip().to_string(),
                        level: player.get_level(),
                        name: player.get_name().clone(),
                        chat,
                    })
                })
                .collect(),
        )
    }
}

#[derive(Debug)]
pub struct JoinGameAccepted {
    pub level: u32,
    pub name: String,
}

#[derive(Message)]
#[rtype(result = "Option<RequestCosmeticsAccepted>")]
pub struct SendRequestCosmetics {
    pub socket_id: u32,
}

impl Handler<SendRequestCosmetics> for Sm64JsServer {
    type Result = Option<RequestCosmeticsAccepted>;

    fn handle(
        &mut self,
        send_request_cosmetics: SendRequestCosmetics,
        _: &mut Context<Self>,
    ) -> Self::Result {
        let socket_id = send_request_cosmetics.socket_id;
        let level = self.clients.get(&socket_id)?.get_level()?;
        let room = self.rooms.get(&level)?;

        Some(RequestCosmeticsAccepted(room.get_all_skin_data().ok()?))
    }
}

#[derive(Debug)]
pub struct RequestCosmeticsAccepted(pub Vec<Vec<u8>>);

#[derive(Message)]
#[rtype(result = "()")]
pub struct SendPlayerList;

impl Handler<SendPlayerList> for Sm64JsServer {
    type Result = ();

    fn handle(&mut self, _: SendPlayerList, _: &mut Context<Self>) {
        let mut fields: Vec<_> = self
            .rooms
            .iter_mut()
            .filter_map(|mut room| room.get_player_list_field())
            .collect();
        fields.sort_unstable_by(|(num1, _), (num2, _)| num2.cmp(num1));

        let mut sum = 100u16;
        let mut field_sum = 0u8;
        fields.retain(|(_, field)| {
            sum += field.name.len() as u16 + field.value.len() as u16;
            field_sum += 1;
            sum <= 6000 && field_sum <= 25
        });
        let author = sm64js_common::DiscordRichEmbedAuthor {
            name: format!("Players online: {}", self.players.len()),
            url: None,
            icon_url: None,
        };

        actix::spawn(async move {
            #[cfg(debug_assertions)]
            let channel_id = "831511367763623966";
            #[cfg(not(debug_assertions))]
            let channel_id = "831428759655284797";
            #[cfg(debug_assertions)]
            let message_id = "831512522308845568";
            #[cfg(not(debug_assertions))]
            let message_id = "831438385624776714";
            send_discord_message(
                channel_id,
                Some(message_id),
                "".to_string(),
                Some(fields.into_iter().map(|(_, field)| field).collect()),
                author,
                None,
            )
            .await;
        });
    }
}

impl Sm64JsServer {
    pub fn new(pool: web::Data<DbPool>, chat_history: ChatHistoryData, rooms: Rooms) -> Self {
        Sm64JsServer {
            pool,
            clients: Arc::new(DashMap::new()),
            players: HashMap::new(),
            rooms,
            chat_history,
        }
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

    fn get_client_by_account_id(&self, account_id: i32) -> Option<Ref<u32, Client>> {
        self.clients
            .iter()
            .find(|client| client.get_account_id() == account_id)
            .and_then(|client| {
                let socket_id = client.value().get_socket_id();
                self.clients.get(&socket_id)
            })
    }

    fn get_client_by_ip_addr(&self, ip: String) -> Option<Ref<u32, Client>> {
        self.clients
            .iter()
            .find(|client| client.get_ip() == &ip)
            .and_then(|client| {
                let socket_id = client.value().get_socket_id();
                self.clients.get(&socket_id)
            })
    }

    fn handle_command(chat_msg: ChatMsg, auth_info: AuthInfo) -> Option<Vec<u8>> {
        let message = chat_msg
            .message
            .char_indices()
            .nth(1)
            .and_then(|(i, _)| chat_msg.message.get(i..))
            .unwrap_or("");
        if let Some(index) = message.find(' ') {
            let (cmd, message) = message.split_at(index);
            let cmd = cmd.to_ascii_uppercase();
            if let Some(permission) = PRIVILEGED_COMMANDS.lock().get(cmd.as_str()) {
                if !auth_info.has_permission(permission) {
                    return None;
                }
            }
            match cmd.as_ref() {
                // TODO store in enum
                "ANNOUNCEMENT" => {
                    let root_msg = RootMsg {
                        message: Some(root_msg::Message::UncompressedSm64jsMsg(Sm64JsMsg {
                            message: Some(sm64_js_msg::Message::AnnouncementMsg(AnnouncementMsg {
                                message: message.to_string(),
                                timer: 300,
                            })),
                        })),
                    };

                    let mut msg = vec![];
                    root_msg.encode(&mut msg).unwrap();
                    Some(msg)
                }
                _ => None,
            }
        } else {
            None
        }
    }

    fn handle_chat(
        &self,
        player: &Arc<RwLock<Player>>,
        socket_id: u32,
        mut chat_msg: ChatMsg,
        auth_info: AuthInfo,
    ) -> Result<Option<Vec<u8>>, Vec<u8>> {
        let account_id = if let Some(client) = self.clients.get(&socket_id) {
            client.get_account_id()
        } else {
            return Ok(None);
        };
        let conn = self.pool.get().unwrap();
        if let Ok(Some(mute)) = sm64js_db::is_account_muted(&conn, account_id) {
            let mut message = "You are muted".to_string();
            if let Some(expires_at) = mute.expires_at {
                let expires_in = expires_at - Utc::now().naive_utc();
                let expires_in = Duration::seconds(expires_in.num_seconds());
                message += &format!(
                    " for {}",
                    format_duration(expires_in.to_std().unwrap_or_default())
                );
            }
            chat_msg.message = message;
            chat_msg.sender = "[Server]".to_string();
            chat_msg.is_server = true;
            let root_msg = RootMsg {
                message: Some(root_msg::Message::UncompressedSm64jsMsg(Sm64JsMsg {
                    message: Some(sm64_js_msg::Message::ChatMsg(chat_msg)),
                })),
            };
            let mut msg = vec![];
            root_msg.encode(&mut msg).unwrap();

            return Err(msg);
        }
        drop(conn);

        let username = player.read().get_name().clone();
        let root_msg = match player.write().add_chat_message(
            self.pool.clone(),
            self.chat_history.clone(),
            &chat_msg.message,
            self.rooms.clone(),
        ) {
            ChatResult::Ok((message, is_spam)) => {
                if is_spam || message.is_empty() {
                    None
                } else {
                    chat_msg.message = message;
                    chat_msg.is_admin = auth_info.is_in_game_admin();
                    chat_msg.socket_id = socket_id;
                    chat_msg.sender = username;
                    Some(RootMsg {
                        message: Some(root_msg::Message::UncompressedSm64jsMsg(Sm64JsMsg {
                            message: Some(sm64_js_msg::Message::ChatMsg(chat_msg)),
                        })),
                    })
                }
            }
            ChatResult::Err(err) => match err {
                ChatError::Spam => {
                    chat_msg.message =
                            "Chat message ignored: You have to wait longer between sending chat messages"
                                .to_string();
                    chat_msg.sender = "[Server]".to_string();
                    chat_msg.is_server = true;

                    let msg = Sm64JsServer::create_uncompressed_msg(sm64_js_msg::Message::ChatMsg(
                        chat_msg,
                    ));

                    return Err(msg);
                }
                ChatError::ExcessiveSpam => {
                    let conn = self.pool.get().unwrap();
                    let expires_at = Utc::now().naive_utc()
                        + Duration::from_std(time::Duration::from_secs(300)).unwrap();
                    if let Err(err) = sm64js_db::mute_account(
                        &conn,
                        Some("automatic mute due to sending too many messages".to_string()),
                        Some(expires_at),
                        account_id,
                    ) {
                        eprintln!("{:?}", err);
                    };

                    chat_msg.message =
                        "You have been muted for 5min due to sending way too many messages"
                            .to_string();
                    chat_msg.sender = "[Server]".to_string();
                    chat_msg.is_server = true;

                    let msg = Sm64JsServer::create_uncompressed_msg(sm64_js_msg::Message::ChatMsg(
                        chat_msg,
                    ));

                    return Err(msg);
                }
                ChatError::Screaming => {
                    chat_msg.message = "COULD YOU PLEASE STOP SCREAMING?".to_string();
                    chat_msg.sender = "[Server]".to_string();
                    chat_msg.is_server = true;

                    let msg = Sm64JsServer::create_uncompressed_msg(sm64_js_msg::Message::ChatMsg(
                        chat_msg,
                    ));

                    return Err(msg);
                }
            },
            ChatResult::NotFound => None,
        };

        Ok(if let Some(root_msg) = root_msg {
            let mut msg = vec![];
            root_msg.encode(&mut msg).unwrap();
            Some(msg)
        } else {
            None
        })
    }

    fn is_name_valid(name: &str) -> bool {
        if name.len() < 3 || name.len() > 14 || name.to_ascii_uppercase().contains("SERVER") {
            return false;
        }
        let mut sanitized_name = sanitize_chat(name);
        sanitized_name = sanitized_name.censor();
        sanitized_name == name
    }
}
