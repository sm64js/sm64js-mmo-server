mod chat;
mod date_format;
mod login;
mod logout;

pub use chat::{ChatError, ChatHistory, ChatHistoryData, ChatResult};

use actix_web::dev;
use paperclip::actix::{web, Mountable};

pub fn service() -> impl dev::HttpServiceFactory + Mountable {
    web::scope("/api")
        .service(web::resource("/chat").route(web::get().to(chat::get_chat)))
        .service(login::service())
        .service(web::resource("/logout").route(web::post().to(logout::post_logout)))
}
