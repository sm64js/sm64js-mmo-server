use crate::server;

use actix::prelude::*;
use actix_web_actors::ws;
use prost::Message as ProstMessage;
use server::Sm64JsServer;
use sm64js_auth::AuthInfo;
use sm64js_proto::{
    initialization_msg, root_msg, sm64_js_msg, InitGameDataMsg, InitializationMsg, RootMsg,
    Sm64JsMsg,
};
use std::time::{Duration, Instant};

/// How often heartbeat pings are sent
const HEARTBEAT_INTERVAL: Duration = Duration::from_secs(5);

/// How long before lack of client response causes a timeout
const CLIENT_TIMEOUT: Duration = Duration::from_secs(120);

/// How long before client gets kicked on not updating its location
const CLIENT_AFK_TIMEOUT: Duration = Duration::from_secs(300);

pub struct Sm64JsWsSession {
    id: u32,
    hb: Instant,
    hb_data: Instant,
    hb_afk: Instant,
    data: Vec<f32>,
    data_afk_check: Vec<f32>,
    data_loop_index: u8,
    addr: Addr<server::Sm64JsServer>,
    auth_info: AuthInfo,
    ip: String,
}

impl Actor for Sm64JsWsSession {
    type Context = ws::WebsocketContext<Self>;

    fn started(&mut self, ctx: &mut Self::Context) {
        self.hb(ctx);

        let addr = ctx.address();
        self.addr
            .send(server::Connect {
                addr: addr.recipient(),
                auth_info: self.auth_info.clone(),
                ip: self.ip.clone(),
            })
            .into_actor(self)
            .then(|res, act, ctx| {
                match res {
                    Ok(res) => act.id = res,
                    _ => ctx.stop(),
                }
                fut::ready(())
            })
            .wait(ctx);
    }

    fn stopping(&mut self, _: &mut Self::Context) -> Running {
        self.addr.do_send(server::Disconnect { socket_id: self.id });
        Running::Stop
    }
}

impl StreamHandler<Result<ws::Message, ws::ProtocolError>> for Sm64JsWsSession {
    fn handle(&mut self, msg: Result<ws::Message, ws::ProtocolError>, ctx: &mut Self::Context) {
        match msg {
            Ok(ws::Message::Ping(msg)) => {
                self.hb = Instant::now();
                ctx.pong(&msg);
            }
            Ok(ws::Message::Pong(_)) => {
                self.hb = Instant::now();
            }
            Ok(ws::Message::Binary(bin)) => {
                let data = RootMsg::decode(bin.clone()).unwrap();
                let sm64js_msg: Sm64JsMsg = match data.message {
                    Some(root_msg::Message::UncompressedSm64jsMsg(msg)) => msg,
                    Some(root_msg::Message::CompressedSm64jsMsg(msg)) => {
                        use flate2::write::ZlibDecoder;
                        use std::io::Write;

                        let mut decoder = ZlibDecoder::new(Vec::new());
                        decoder.write_all(&msg).unwrap();
                        let msg = decoder.finish().unwrap();
                        Sm64JsMsg::decode(&msg[..]).unwrap()
                    }
                    None => {
                        return;
                    }
                };
                match sm64js_msg.message {
                    Some(sm64_js_msg::Message::PingMsg(_)) => {
                        ctx.binary(bin);
                    }
                    Some(sm64_js_msg::Message::MarioMsg(mario_msg)) => {
                        self.data_loop_index += 1;
                        self.hb_data = Instant::now();
                        if self.data_loop_index >= 30 {
                            self.data = mario_msg.pos.clone();
                            self.data_loop_index = 0;
                        }
                        self.addr.do_send(server::SetData {
                            socket_id: self.id,
                            data: mario_msg,
                        });
                    }
                    Some(sm64_js_msg::Message::AttackMsg(attack_msg)) => {
                        self.addr.do_send(server::SendAttack {
                            socket_id: self.id,
                            attack_msg,
                        })
                    }
                    Some(sm64_js_msg::Message::GrabMsg(grab_flag_msg)) => {
                        self.addr.do_send(server::SendGrabFlag {
                            socket_id: self.id,
                            grab_flag_msg,
                        })
                    }
                    Some(sm64_js_msg::Message::ChatMsg(chat_msg)) => {
                        self.addr
                            .send(server::SendChat {
                                socket_id: self.id,
                                chat_msg,
                                auth_info: self.auth_info.clone(),
                            })
                            .into_actor(self)
                            .then(move |res, _act, ctx| {
                                if let Ok(Some(msg)) = res {
                                    ctx.binary(msg);
                                }

                                fut::ready(())
                            })
                            .wait(ctx);
                    }
                    Some(sm64_js_msg::Message::InitializationMsg(init_msg)) => {
                        match init_msg.message {
                            Some(initialization_msg::Message::InitGameDataMsg(_)) => {
                                // TODO clients don't send this
                            }
                            Some(initialization_msg::Message::JoinGameMsg(join_game_msg)) => {
                                let socket_id = self.id;
                                self.addr
                                .send(server::SendJoinGame {
                                    socket_id: self.id,
                                    join_game_msg,
                                    auth_info: self.auth_info.clone()
                                })
                                .into_actor(self)
                                .then(move |res, _act, ctx| {
                                    match res {
                                        Ok(res) => {
                                            let init_msg = if let Some(server::JoinGameAccepted { level, name }) = res {
                                                InitializationMsg {
                                                    message: Some(initialization_msg::Message::InitGameDataMsg(InitGameDataMsg {
                                                        accepted: true,
                                                        level,
                                                        name,
                                                        socket_id,
                                                }))}
                                            } else {
                                                InitializationMsg {
                                                    message: Some(initialization_msg::Message::InitGameDataMsg(InitGameDataMsg {
                                                        accepted: false,
                                                        ..Default::default()
                                                }))}
                                            };
                                            let msg = Sm64JsServer::create_uncompressed_msg(sm64_js_msg::Message::InitializationMsg(
                                                init_msg,
                                            ));
                                            ctx.binary(msg);
                                        }
                                        Err(err) => {
                                            eprintln!("{:?}", err);
                                        }
                                    }
                                    fut::ready(())
                                })
                                .wait(ctx);
                            }
                            Some(initialization_msg::Message::RequestCosmeticsMsg(_)) => {
                                self.addr
                                    .send(server::SendRequestCosmetics { socket_id: self.id })
                                    .into_actor(self)
                                    .then(move |res, _act, ctx| {
                                        match res {
                                            Ok(Some(messages)) => {
                                                messages.0.into_iter().for_each(|msg| {
                                                    ctx.binary(msg);
                                                });
                                            }
                                            Ok(None) => {
                                                // TODO ignore?
                                            }
                                            Err(err) => {
                                                eprintln!("{:?}", err);
                                            }
                                        }
                                        fut::ready(())
                                    })
                                    .wait(ctx);
                            }
                            None => {}
                        }
                    }
                    Some(sm64_js_msg::Message::SkinMsg(skin_msg)) => {
                        self.addr.do_send(server::SendSkin {
                            socket_id: self.id,
                            skin_msg,
                        });
                    }
                    Some(sm64_js_msg::Message::ListMsg(_)) => {
                        // TODO clients don't send this
                    }
                    Some(sm64_js_msg::Message::PlayerListsMsg(_player_lists_msg)) => {
                        // TODO clients don't send this
                    }
                    Some(sm64_js_msg::Message::AnnouncementMsg(_)) => {
                        // TODO clients don't send this
                    }
                    None => {}
                }
            }
            Ok(ws::Message::Close(reason)) => {
                ctx.close(reason);
                ctx.stop();
            }
            _ => ctx.stop(),
        }
    }
}

impl Sm64JsWsSession {
    pub fn new(addr: Addr<server::Sm64JsServer>, auth_info: AuthInfo, ip: String) -> Self {
        Self {
            id: 0,
            hb: Instant::now(),
            hb_data: Instant::now(),
            hb_afk: Instant::now(),
            data: Vec::new(),
            data_afk_check: Vec::new(),
            data_loop_index: 0,
            addr,
            auth_info,
            ip,
        }
    }

    /// helper method that sends ping to client every second.
    ///
    /// also this method checks heartbeats from client
    fn hb(&self, ctx: &mut <Self as Actor>::Context) {
        ctx.run_interval(HEARTBEAT_INTERVAL, |act, ctx| {
            if Instant::now().duration_since(act.hb) > CLIENT_TIMEOUT {
                ctx.stop();
                return;
            }

            if Instant::now().duration_since(act.hb_data) > CLIENT_TIMEOUT {
                ctx.stop();
                return;
            }

            if act.data_afk_check != act.data {
                act.data_afk_check = act.data.clone();
                act.hb_afk = Instant::now();
            } else if Instant::now().duration_since(act.hb_afk) > CLIENT_AFK_TIMEOUT {
                ctx.stop();
                return;
            }

            ctx.ping(b"");
        });
    }
}

impl Handler<server::Message> for Sm64JsWsSession {
    type Result = ();

    fn handle(&mut self, msg: server::Message, ctx: &mut Self::Context) {
        match msg {
            server::Message::SendData(data) => ctx.binary(data),
            server::Message::Kick => ctx.stop(),
        }
    }
}
