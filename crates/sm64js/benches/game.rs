use actix::prelude::*;
use actix_rt::System;
use criterion::{criterion_group, criterion_main, Criterion};
use dashmap::DashMap;
use parking_lot::RwLock;
use sm64js_ws::{Client, Game, Player, Room};
use std::sync::Arc;

struct ServerStub;

impl Actor for ServerStub {
    type Context = Context<Self>;

    fn started(&mut self, _: &mut Self::Context) {}
}

impl Handler<sm64js_ws::Message> for ServerStub {
    type Result = ();

    fn handle(&mut self, _msg: sm64js_ws::Message, _ctx: &mut Self::Context) -> Self::Result {}
}

fn broadcast_data(c: &mut Criterion) {
    let _ = System::new();

    let rooms = Room::init_rooms();

    {
        let mut room = rooms.get_mut(&1000).unwrap();

        let clients = Arc::new(DashMap::new());
        let server_stub = ServerStub {};
        let session_addr = server_stub.start().recipient();

        for i in 0..100u32 {
            let auth_info = sm64js_auth::AuthInfo(sm64js_db::AuthInfo {
                account: sm64js_db::models::Account {
                    id: i as i32,
                    ..Default::default()
                },
                ..Default::default()
            });
            clients.insert(
                i,
                Client::new(session_addr.clone(), auth_info, "".to_string(), i),
            );
            let player = Arc::new(RwLock::new(Player::new(
                clients.clone(),
                i,
                1000,
                "".to_string(),
            )));
            room.add_player(0, Arc::downgrade(&player));
        }
    }

    c.bench_function("Game::broadcast_data", |b| {
        b.iter(|| Game::broadcast_data(rooms.clone()))
    });
}

criterion_group!(benches, broadcast_data);
criterion_main!(benches);
