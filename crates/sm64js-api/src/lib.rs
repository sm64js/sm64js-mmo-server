mod chat;
mod date_format;
mod login;

pub use chat::{ChatError, ChatHistory, ChatHistoryData, ChatResult};

use actix_web::dev;
use paperclip::actix::{Mountable, web};

pub fn service() -> impl dev::HttpServiceFactory + Mountable {
    web::scope("")
        .service(web::resource("/chat").route(web::get().to(chat::get_chat)))
        .service(login::service())
}