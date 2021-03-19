#![feature(try_blocks)]
#![feature(try_trait)]

#[macro_use]
extern crate lazy_static;

#[macro_use]
extern crate maplit;

mod client;
mod game;
mod room;
mod server;
mod session;

pub use client::{Client, Clients, Player, Players, WeakPlayers};
pub use game::Game;
pub use room::{Flag, Room, Rooms};
pub use server::{KickClientByAccountId, Message, Sm64JsServer};
pub use session::Sm64JsWsSession;
