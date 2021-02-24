#![feature(try_blocks)]

#[macro_use]
extern crate lazy_static;

#[macro_use]
extern crate maplit;

mod auth;
mod chat;
mod client;
mod date_format;
mod game;
mod identity;
mod login;
// mod permission;
mod room;
mod server;
mod session;

pub use chat::{ChatError, ChatHistory, ChatHistoryData, ChatResult};
pub use client::{Client, Clients, Player, Players, WeakPlayers};
pub use identity::Identity;
// pub use permission::{Permission, Token, Tokens};
pub use room::{Flag, Room, Rooms};
pub use server::Message;

pub type DbPool = r2d2::Pool<ConnectionManager<PgConnection>>;

pub mod proto {
    include!(concat!(env!("OUT_DIR"), "/sm64js.rs"));
}

use actix::prelude::*;
use actix_web::{middleware, App, Error, HttpRequest, HttpResponse, HttpServer};
use actix_web_actors::ws;
use diesel::{
    r2d2::{self, ConnectionManager},
    PgConnection,
};
use paperclip::{
    actix::{api_v2_operation, web, OpenApiExt},
    v2::models::{DefaultApiRaw, Info, Tag},
};
use session::Sm64JsWsSession;

#[api_v2_operation(tags(Hidden))]
async fn ws_index(
    req: HttpRequest,
    stream: web::Payload,
    srv: web::Data<Addr<server::Sm64JsServer>>,
) -> Result<HttpResponse, Error> {
    let ip = req.peer_addr();
    let real_ip = req
        .connection_info()
        .realip_remote_addr()
        .map(|ip| ip.to_string());
    ws::start(
        Sm64JsWsSession::new(srv.get_ref().clone(), ip, real_ip),
        &req,
        stream,
    )
}

#[cfg(feature = "docker")]
const DIST_FOLDER: &str = "./dist";
#[cfg(not(feature = "docker"))]
const DIST_FOLDER: &str = "../client/dist";

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    use actix_session::CookieSession;
    use parking_lot::RwLock;
    use std::env;

    dotenv::dotenv().ok();

    env::set_var("RUST_BACKTRACE", "1");
    env::set_var("RUST_LOG", "actix_server=info,actix_web=info");
    env_logger::init();

    let connspec = env::var("DATABASE_URL").expect("DATABASE_URL");
    let manager = ConnectionManager::<PgConnection>::new(connspec);
    let pool = r2d2::Pool::builder()
        .build(manager)
        .expect("Failed to create pool.");
    let chat_history = web::Data::new(RwLock::new(ChatHistory::default()));
    let rooms = Room::init_rooms();
    let server = server::Sm64JsServer::new(chat_history.clone(), rooms.clone()).start();
    game::Game::run(rooms.clone());

    // let tokens = Token::try_load().unwrap();

    // TODO fetch Google Discovery document and cache it
    // let request = awc::Client::default()
    //     .get("https://accounts.google.com/.well-known/openid-configuration")
    //     .send();
    // let response = request.await.unwrap();
    // if !response.status().is_success() {
    //     panic!("Could not fetch Google Discovery document");
    // }

    HttpServer::new(move || {
        let spec = DefaultApiRaw {
            tags: vec![
                Tag {
                    name: "Hidden".to_string(),
                    description: None,
                    external_docs: None,
                },
                Tag {
                    name: "Permission".to_string(),
                    description: Some(
                        "API for generating new tokens and assigning permissions.".to_string(),
                    ),
                    external_docs: None,
                },
                Tag {
                    name: "Chat".to_string(),
                    description: Some("Chat related API endpoints".to_string()),
                    external_docs: None,
                },
            ],
            info: Info {
                title: "SM64JS API".into(),
                ..Default::default()
            },
            ..Default::default()
        };

        App::new()
            .wrap_api_with_spec(spec)
            .data(pool.clone())
            .app_data(chat_history.clone())
            .data(server.clone())
            // .app_data(tokens.clone())
            .wrap(middleware::Logger::default())
            .with_json_spec_at("/api/spec")
            .service(web::resource("/ws/").to(ws_index))
            .service(web::resource("/chat").route(web::get().to(chat::get_chat)))
            // .service(permission::service())
            .service(login::service())
            .wrap(auth::Auth)
            .wrap(
                CookieSession::signed(&[0; 32])
                    .name("sm64js")
                    .path("/")
                    .max_age(3600 * 24 * 7)
                    .secure(false),
            )
            .build()
            .service(
                actix_files::Files::new("/api", "./sm64js/src/openapi").index_file("index.html"),
            )
            .service(actix_files::Files::new("/", DIST_FOLDER).index_file("index.html"))
    })
    .bind("0.0.0.0:3060")?
    .run()
    .await
}
