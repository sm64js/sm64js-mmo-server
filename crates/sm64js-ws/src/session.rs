use crate::{
    proto::{
        initialization_msg, root_msg, sm64_js_msg, InitGameDataMsg, InitializationMsg, RootMsg,
        Sm64JsMsg,
    },
    server,
};

use actix::prelude::*;
use actix_web_actors::ws;
use prost::Message as ProstMessage;
use server::Sm64JsServer;
use sm64js_auth::AuthInfo;
use std::{
    net::SocketAddr,
    time::{Duration, Instant},
};

/// How often heartbeat pings are sent
const HEARTBEAT_INTERVAL: Duration = Duration::from_secs(5);

/// How long before lack of client response causes a timeout
const CLIENT_TIMEOUT: Duration = Duration::from_secs(10);

pub struct Sm64JsWsSession {
    id: u32,
    hb: Instant,
    hb_data: Option<Instant>,
    addr: Addr<server::Sm64JsServer>,
    auth_info: AuthInfo,
    ip: Option<SocketAddr>,
    real_ip: Option<String>,
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
                ip: self.ip,
                real_ip: self.real_ip.clone(),
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
                        self.hb_data = Some(Instant::now());
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
    pub fn new(
        addr: Addr<server::Sm64JsServer>,
        auth_info: AuthInfo,
        ip: Option<SocketAddr>,
        real_ip: Option<String>,
    ) -> Self {
        Self {
            id: 0,
            hb: Instant::now(),
            hb_data: None,
            addr,
            auth_info,
            ip,
            real_ip,
        }
    }

    /// helper method that sends ping to client every second.
    ///
    /// also this method checks heartbeats from client
    fn hb(&self, ctx: &mut <Self as Actor>::Context) {
        ctx.run_interval(HEARTBEAT_INTERVAL, |act, ctx| {
            if Instant::now().duration_since(act.hb) > CLIENT_TIMEOUT {
                println!("Websocket Client heartbeat failed, disconnecting!");

                ctx.stop();

                return;
            }

            if let Some(hb_data) = act.hb_data {
                if Instant::now().duration_since(hb_data) > CLIENT_TIMEOUT {
                    println!("Websocket Client timed out due to not sending data, disconnecting!");

                    ctx.stop();

                    return;
                }
            }

            ctx.ping(b"");
        });
    }
}

impl Handler<server::Message> for Sm64JsWsSession {
    type Result = ();

    fn handle(&mut self, msg: server::Message, ctx: &mut Self::Context) {
        ctx.binary(msg.0);
    }
}
