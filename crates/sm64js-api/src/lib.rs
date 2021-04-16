#[macro_use]
extern crate anyhow;

mod account;
mod ban;
mod chat;
mod ip_ban;
mod login;
mod logout;
mod mute;
mod players;

use actix_web::dev;
use paperclip::actix::{web, Mountable};

pub fn service() -> impl dev::HttpServiceFactory + Mountable {
    web::scope("/api")
        .service(web::resource("/chat").route(web::get().to(chat::get_chat)))
        .service(players::service())
        .service(account::service())
        .service(login::service())
        .service(web::resource("/logout").route(web::post().to(logout::post_logout)))
        .service(web::resource("/ban").route(web::post().to(ban::post_ban)))
        .service(web::resource("/ipban").route(web::post().to(ip_ban::post_ban)))
        .service(web::resource("/mute").route(web::post().to(mute::post_mute)))
}
