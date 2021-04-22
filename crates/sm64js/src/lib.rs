#[macro_use]
extern crate diesel_migrations;

mod websocket;

use actix::prelude::*;
use actix_cors::Cors;
use actix_http::cookie::SameSite;
use actix_web::{dev::Server, middleware, App, HttpServer};
use diesel::{
    r2d2::{self, ConnectionManager},
    PgConnection,
};
use paperclip::{
    actix::{web, OpenApiExt},
    v2::models::{DefaultApiRaw, Info, Tag},
};
use sm64js_common::{ChatHistory, ChatHistoryData};
use sm64js_env::{COOKIE_SAME_SITE_NONE, DATABASE_URL};
use sm64js_ws::{Game, Room, Sm64JsServer};

embed_migrations!("../sm64js-db/migrations");

#[cfg(feature = "docker")]
const DIST_FOLDER: &str = "./dist";
#[cfg(not(feature = "docker"))]
const DIST_FOLDER: &str = "./client/dist";

#[cfg(debug_assertions)]
const LOG_LEVEL: &str = "actix_server=debug,actix_web=debug";
#[cfg(not(debug_assertions))]
const LOG_LEVEL: &str = "actix_server=info,actix_web=warn";

pub fn main() -> std::io::Result<Server> {
    use actix_session::CookieSession;
    use parking_lot::RwLock;
    use std::env;

    sm64js_env::load();

    env::set_var("RUST_BACKTRACE", "1");
    env::set_var("RUST_LOG", LOG_LEVEL);
    env_logger::init();

    let manager = ConnectionManager::<PgConnection>::new(DATABASE_URL.get().unwrap());
    let pool = web::Data::new(
        r2d2::Pool::builder()
            .build(manager)
            .expect("Failed to create pool."),
    );
    let conn = pool.get().unwrap();
    embedded_migrations::run(&conn).unwrap();
    let chat_history: ChatHistoryData = web::Data::new(RwLock::new(ChatHistory::default()));
    let rooms = Room::init_rooms();
    let server = Sm64JsServer::new(pool.clone(), chat_history.clone(), rooms.clone()).start();
    Game::run(server.clone(), rooms);

    // TODO fetch Google Discovery document and cache it
    // let request = awc::Client::default()
    //     .get("https://accounts.google.com/.well-known/openid-configuration")
    //     .send();
    // let response = request.await.unwrap();
    // if !response.status().is_success() {
    //     panic!("Could not fetch Google Discovery document");
    // }

    Ok(HttpServer::new(move || {
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
                    name: "Auth".to_string(),
                    description: Some(
                        "\
Authentification related API endpoints.\n\n
Initial authentification must be done on the site itself via Discord OAuth2.\
A session cookie will then be stored in the user's browser that can be used to fetch all APIs that require authentification.
"
                        .to_string(),
                    ),
                    external_docs: None,
                },
                Tag {
                    name: "PlayerInfo".to_string(),
                    description: None,
                    external_docs: None,
                },
                Tag {
                    name: "Chat".to_string(),
                    description: Some("Chat related API endpoints".to_string()),
                    external_docs: None,
                },
                Tag {
                    name: "Moderation".to_string(),
                    description: None,
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
            .app_data(pool.clone())
            .app_data(chat_history.clone())
            .data(server.clone())
            .wrap(middleware::Logger::default())
            .with_json_spec_at("/apispec")
            .service(web::resource("/ws/").to(websocket::index))
            .service(sm64js_api::service())
            .wrap(sm64js_auth::Auth)
            .wrap(
                CookieSession::signed(&[0; 32])
                    .name("sm64js")
                    .path("/")
                    .max_age(3600 * 24 * 7)
                    .http_only(true)
                    .same_site(if *COOKIE_SAME_SITE_NONE.get().unwrap() {
                        SameSite::None
                    } else {
                        SameSite::Lax
                    })
                    .secure(true),
            )
            .wrap(
                Cors::default()
                    .allow_any_header()
                    .allow_any_method()
                    .allow_any_origin()
                    .supports_credentials()
                    .max_age(3600),
            )
            .build()
            .service(actix_files::Files::new("/apidoc", "./openapi").index_file("index.html"))
            .service(actix_files::Files::new("/", DIST_FOLDER).index_file("index.html"))
    })
    .bind("0.0.0.0:3060")?
    .run())
}
