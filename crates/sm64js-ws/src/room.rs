use crate::{
    proto::{root_msg, sm64_js_msg, FlagMsg, MarioListMsg, RootMsg, SkinMsg, Sm64JsMsg},
    Player, WeakPlayers,
};

use anyhow::Result;
use dashmap::DashMap;
use flate2::{write::ZlibEncoder, Compression};
use prost::Message as ProstMessage;
use rand::{self, Rng};
use rayon::prelude::*;
use std::{
    collections::HashMap,
    io::prelude::*,
    sync::{Arc, RwLock, Weak},
};

pub type Rooms = Arc<DashMap<u32, Room>>;

#[derive(Debug)]
pub struct Room {
    id: String,
    flags: Vec<RwLock<Flag>>,
    players: WeakPlayers,
}

impl Room {
    pub fn init_rooms() -> Rooms {
        let rooms = DashMap::new();
        rooms.insert(
            5,
            Room {
                id: "Cool, Cool Mountain".to_string(),
                flags: vec![RwLock::new(Flag::new([0., 7657., 0.]))],
                players: HashMap::new(),
            },
        );
        rooms.insert(
            9,
            Room {
                id: "Bob-omb Battlefield".to_string(),
                flags: vec![RwLock::new(Flag::new([-2384., 260., 6203.]))],
                players: HashMap::new(),
            },
        );
        rooms.insert(
            16,
            Room {
                id: "Castle Grounds".to_string(),
                flags: vec![RwLock::new(Flag::new([0., 3657., 0.]))],
                players: HashMap::new(),
            },
        );
        rooms.insert(
            24,
            Room {
                id: "Whomps Fortress".to_string(),
                flags: vec![RwLock::new(Flag::new([0., 7657., 0.]))],
                players: HashMap::new(),
            },
        );
        rooms.insert(
            27,
            Room {
                id: "Princess's Secret Slide".to_string(),
                flags: vec![RwLock::new(Flag::new([0., 7657., 0.]))],
                players: HashMap::new(),
            },
        );
        rooms.insert(
            36,
            Room {
                id: "Tall, Tall Mountain".to_string(),
                flags: vec![RwLock::new(Flag::new([0., 7657., 0.]))],
                players: HashMap::new(),
            },
        );
        rooms.insert(
            1000,
            Room {
                id: "Mushroom Battlefield".to_string(),
                flags: vec![
                    RwLock::new(Flag::new([9380., 7657., -8980.])),
                    RwLock::new(Flag::new([-5126., 3678., 10106.])),
                    RwLock::new(Flag::new([-14920., 3800., -8675.])),
                    RwLock::new(Flag::new([12043., 3000., 10086.])),
                ],
                players: HashMap::new(),
            },
        );

        Arc::new(rooms)
    }

    pub fn process_flags(&self) {
        self.flags.par_iter().for_each(|flag| {
            let mut flag = flag.write().unwrap();
            flag.process_falling();
            flag.process_idle();
        });
    }

    pub fn broadcast_data(&self) -> Result<()> {
        let mario_list: Vec<_> = self
            .players
            .values()
            .par_bridge()
            .filter_map(|player| {
                if let Some(player) = player.upgrade() {
                    player.read().get_data()
                } else {
                    None
                }
            })
            .collect();
        let flag_list: Vec<_> = self
            .flags
            .iter()
            .par_bridge()
            .map(|flag| flag.read().unwrap().get_msg())
            .collect();
        let sm64js_msg = Sm64JsMsg {
            message: Some(sm64_js_msg::Message::ListMsg(MarioListMsg {
                flag: flag_list,
                mario: mario_list,
            })),
        };
        let mut msg = vec![];
        sm64js_msg.encode(&mut msg)?;

        let mut encoder = ZlibEncoder::new(Vec::new(), Compression::fast());
        encoder.write_all(&msg)?;
        let msg = encoder.finish()?;

        let root_msg = RootMsg {
            message: Some(root_msg::Message::CompressedSm64jsMsg(msg)),
        };
        let mut msg = vec![];
        root_msg.encode(&mut msg)?;

        self.broadcast_message(&msg);
        Ok(())
    }

    pub fn broadcast_skins(&self) -> Result<()> {
        let messages: Vec<_> = self
            .players
            .par_iter()
            .filter_map(|(_, player)| {
                if let Some(player) = player.upgrade() {
                    let player_name = player.read().get_name().clone();
                    if let Some(skin_data) = player.write().get_updated_skin_data() {
                        Some((skin_data, player_name))
                    } else {
                        None
                    }
                } else {
                    None
                }
            })
            .map(|(skin_data, player_name)| -> Result<_> {
                let root_msg = RootMsg {
                    message: Some(root_msg::Message::UncompressedSm64jsMsg(Sm64JsMsg {
                        message: Some(sm64_js_msg::Message::SkinMsg(SkinMsg {
                            socket_id: 0,
                            skin_data: Some(skin_data),
                            player_name,
                        })),
                    })),
                };
                let mut msg = vec![];
                root_msg.encode(&mut msg)?;
                Ok(msg)
            })
            .collect::<Result<Vec<_>>>()?;

        messages
            .par_iter()
            .for_each(|msg| self.broadcast_message(msg));

        Ok(())
    }

    pub fn broadcast_message(&self, msg: &[u8]) {
        self.players
            .values()
            .par_bridge()
            .map(|player| -> Result<()> {
                if let Some(player) = player.upgrade() {
                    player.read().send_message(msg.to_vec())?
                }
                Ok(())
            })
            .filter_map(Result::ok)
            .collect::<Vec<_>>();
    }

    pub fn has_player(&self, socket_id: u32) -> bool {
        if let Some(res) = self
            .players
            .get(&socket_id)
            .map(|player| player.strong_count() > 0)
        {
            res
        } else {
            false
        }
    }

    pub fn add_player(&mut self, socket_id: u32, player: Weak<parking_lot::RwLock<Player>>) {
        self.players.insert(socket_id, player);
    }

    pub fn process_attack(&self, flag_id: usize, attacker_pos: Vec<f32>, target_id: u32) {
        if let Some(flag) = self.flags.get(flag_id) {
            let mut flag = flag.write().unwrap();
            if let Some(link_id) = flag.linked_to_player {
                // TODO use target_id to determine valid attack
                if link_id != target_id {
                    return;
                }
                flag.linked_to_player = None;
                flag.fall_mode = true;
                flag.pos = Box::new([
                    attacker_pos[0] + rand::thread_rng().gen_range(0f32..=1000.) - 500.,
                    attacker_pos[1] + 600.,
                    attacker_pos[2] + rand::thread_rng().gen_range(0f32..=1000.) - 500.,
                ]);
                flag.height_before_fall = flag.pos[1];
            }
        }
    }

    pub fn process_grab_flag(&self, flag_id: usize, pos: Vec<f32>, socket_id: u32) {
        if let Some(flag) = self.flags.get(flag_id) {
            let mut flag = flag.write().unwrap();
            if flag.linked_to_player.is_some() {
                return;
            }
            let x_diff = pos[0] - flag.pos[0];
            let z_diff = pos[2] - flag.pos[2];
            let dist = (x_diff * x_diff + z_diff * z_diff).sqrt();
            if dist < 50. {
                flag.linked_to_player = Some(socket_id);
                flag.fall_mode = false;
                flag.at_start_position = false;
                flag.idle_timer = 0;
            }
        }
    }
}

#[derive(Debug)]
pub struct Flag {
    pos: Box<[f32; 3]>,
    start_pos: Box<[f32; 3]>,
    linked_to_player: Option<u32>,
    at_start_position: bool,
    idle_timer: u16,
    fall_mode: bool,
    height_before_fall: f32,
}

impl Flag {
    pub fn new(pos: [f32; 3]) -> Self {
        Self {
            pos: Box::new(pos),
            start_pos: Box::new(pos),
            linked_to_player: None,
            at_start_position: true,
            idle_timer: 0,
            fall_mode: false,
            height_before_fall: 20000.,
        }
    }

    pub fn process_falling(&mut self) {
        if self.fall_mode && self.pos[1] > -10000. {
            self.pos[1] -= 2.;
        }
    }

    pub fn process_idle(&mut self) {
        if self.linked_to_player.is_none() && !self.at_start_position {
            self.idle_timer += 1;
            if self.idle_timer > 3000 {
                self.pos = self.start_pos.clone();
                self.fall_mode = false;
                self.at_start_position = true;
                self.idle_timer = 0;
            }
        }
    }

    pub fn get_msg(&self) -> FlagMsg {
        FlagMsg {
            pos: self.pos.to_vec(),
            linked_to_player: self.linked_to_player.is_some(),
            socket_id: self.linked_to_player.unwrap_or_default(), // TODO remove from proto
            height_before_fall: self.height_before_fall,
        }
    }
}
