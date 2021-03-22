use crate::{Player, WeakPlayers};

use anyhow::Result;
use dashmap::DashMap;
use flate2::{write::ZlibEncoder, Compression};
use prost::Message as ProstMessage;
use rand::{self, Rng};
use rayon::prelude::*;
use sm64js_proto::{root_msg, sm64_js_msg, FlagMsg, MarioListMsg, RootMsg, SkinMsg, Sm64JsMsg};
use std::{
    collections::HashMap,
    io::prelude::*,
    sync::{Arc, RwLock, Weak},
};

pub type Rooms = Arc<DashMap<u32, Room>>;

macro_rules! room {
    ( $rooms:expr, $id:expr, $name:expr, $($flag:expr),* ) => {
        $rooms.insert(
            $id,
            Room {
                id: $name.to_string(),
                flags: vec![$(RwLock::new(Flag::new($flag)),)*],
                players: HashMap::new(),
            },
        );
    };
}

#[derive(Debug)]
pub struct Room {
    id: String,
    flags: Vec<RwLock<Flag>>,
    players: WeakPlayers,
}

impl Room {
    pub fn init_rooms() -> Rooms {
        let rooms = DashMap::new();
        room!(rooms, 4, "Big Boo's Haunt", [0., 7657., 0.]);
        room!(rooms, 5, "Cool, Cool Mountain", [0., 7657., 0.]);
        room!(rooms, 6, "Castle Inside First LevelCave", [0., 7657., 0.]);
        room!(rooms, 7, "Hazy Maze Cave", [0., 7657., 0.]);
        room!(rooms, 8, "Shifting Sand Land", [0., 7657., 0.]);
        room!(rooms, 9, "Bob-omb Battlefield", [-2384., 260., 6203.]);
        room!(rooms, 10, "Snowman's Land", [0., 7657., 0.]);
        room!(
            rooms,
            16,
            "Castle Grounds",
            [6300., 910., -5900.],
            [-4200., -1300., -5300.]
        );
        room!(rooms, 24, "Whomps Fortress", [0., 7657., 0.]);
        room!(rooms, 26, "Castle Courtyard", [0., 7657., 0.]);
        room!(rooms, 27, "Princess's Secret Slide", [0., 7657., 0.]);
        room!(rooms, 36, "Tall, Tall Mountain", [0., 7657., 0.]);
        room!(rooms, 602, "Castle Inside Second Level", [0., 7657., 0.]);
        room!(rooms, 999, "Clouded Ruins", [0., 7657., 0.]);
        room!(
            rooms,
            1000,
            "Mushroom Battlefield",
            [9380., 7657., -8980.],
            [-5126., 3678., 10106.],
            [-14920., 3800., -8675.],
            [12043., 3000., 10086.]
        );
        room!(
            rooms,
            1001,
            "CTF/Race Map",
            [-76., 467., -7768.],
            [-76., 467., 7945.]
        );
        room!(rooms, 1002, "Starman Fortress", [0., 7657., 0.]);

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
                    let player_r = player.read();
                    let player_name = player_r.get_name().clone();
                    let socket_id = player_r.get_socket_id();
                    drop(player_r);
                    if let Some(skin_data) = player.write().get_updated_skin_data() {
                        Some((skin_data, player_name, socket_id))
                    } else {
                        None
                    }
                } else {
                    None
                }
            })
            .map(|(skin_data, player_name, socket_id)| -> Result<_> {
                let root_msg = RootMsg {
                    message: Some(root_msg::Message::UncompressedSm64jsMsg(Sm64JsMsg {
                        message: Some(sm64_js_msg::Message::SkinMsg(SkinMsg {
                            socket_id,
                            skin_data: Some(skin_data),
                            player_name,
                            num_coins: 0,
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

    pub fn get_all_skin_data(&self) -> Result<Vec<Vec<u8>>> {
        let messages: Vec<_> = self
            .players
            .par_iter()
            .filter_map(|(_, player)| {
                if let Some(player) = player.upgrade() {
                    let player_name = player.read().get_name().clone();
                    if let Some(skin_data) = player.write().get_skin_data() {
                        Some((skin_data.clone(), player_name))
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
                            num_coins: 0,
                        })),
                    })),
                };
                let mut msg = vec![];
                root_msg.encode(&mut msg)?;
                Ok(msg)
            })
            .collect::<Result<Vec<_>>>()?;

        Ok(messages)
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
