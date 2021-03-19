mod ban;
mod chat;
mod login;
mod logout;
mod players;

use actix_web::dev;
use paperclip::actix::{web, Mountable};

pub fn service() -> impl dev::HttpServiceFactory + Mountable {
    web::scope("/api")
        .service(web::resource("/chat").route(web::get().to(chat::get_chat)))
        .service(players::service())
        .service(login::service())
        .service(web::resource("/logout").route(web::post().to(logout::post_logout)))
        .service(web::resource("/ban").route(web::post().to(ban::post_ban)))
}
