#![feature(iter_map_while)]
#![feature(try_blocks)]

mod client;
mod game;
mod room;
mod server;
mod session;

pub use client::{Client, Clients, Player, Players, WeakPlayers};
pub use game::Game;
pub use room::{Flag, Room, Rooms};
pub use server::{GetPlayers, KickClientByAccountId, Message, Sm64JsServer};
pub use session::Sm64JsWsSession;
