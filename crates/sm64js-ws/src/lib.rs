#![feature(try_blocks)]

#[macro_use]
extern crate lazy_static;

#[macro_use]
extern crate maplit;

pub mod proto {
  include!(concat!(env!("OUT_DIR"), "/sm64js.rs"));
}

mod client;
mod game;
mod room;
mod server;
mod session;

pub use client::{Client, Clients, Player, Players, WeakPlayers};
pub use game::Game;
pub use room::{Flag, Room, Rooms};
pub use server::{Message, Sm64JsServer};
pub use session::Sm64JsWsSession;

