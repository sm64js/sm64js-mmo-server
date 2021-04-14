use crate::{
    server::{BroadcastLobbyData, SendPlayerList},
    Rooms, Sm64JsServer,
};

use actix::Addr;
use anyhow::Result;
use rayon::prelude::*;
use sm64js_common::create_uncompressed_msg;
use sm64js_env::ENABLE_PLAYER_LIST;
use sm64js_proto::{sm64_js_msg, PlayerListsMsg};
use std::{thread, time::Duration};

pub struct Game;

impl Game {
    pub fn run(server: Addr<Sm64JsServer>, rooms: Rooms) {
        thread::spawn(move || {
            let mut i = 0u16;
            let mut j = 0u16;
            loop {
                Self::process_flags(rooms.clone());
                Self::broadcast_data(rooms.clone());
                i += 1;
                if i == 30 {
                    Self::broadcast_skins(rooms.clone());
                    Self::broadcast_valid_update(server.clone(), rooms.clone());
                    i = 0;
                }
                if *ENABLE_PLAYER_LIST.get().unwrap() {
                    j += 1;
                    if j == 300 {
                        Self::send_player_list(server.clone());
                        j = 0;
                    }
                }
                thread::sleep(Duration::from_millis(33));
            }
        });
    }

    fn process_flags(rooms: Rooms) {
        rooms.par_iter().for_each(|room| room.process_flags());
    }

    pub fn broadcast_data(rooms: Rooms) {
        if let Err(err) = rooms
            .par_iter()
            .map(|room| room.broadcast_data())
            .collect::<Result<Vec<_>>>()
        {
            eprintln!("{:?}", err);
        }
    }

    fn broadcast_skins(rooms: Rooms) {
        if let Err(err) = rooms
            .par_iter()
            .map(|room| room.broadcast_skins())
            .collect::<Result<Vec<_>>>()
        {
            eprintln!("{:?}", err);
        }
    }

    fn broadcast_valid_update(server: Addr<Sm64JsServer>, rooms: Rooms) {
        let game = rooms
            .par_iter()
            .map(|room| room.get_and_send_valid_players())
            .collect::<Vec<_>>();
        let message = sm64_js_msg::Message::PlayerListsMsg(PlayerListsMsg { game });
        let root_msg = create_uncompressed_msg(message);

        server.do_send(BroadcastLobbyData { data: root_msg });
    }

    fn send_player_list(server: Addr<Sm64JsServer>) {
        server.do_send(SendPlayerList);
    }
}
