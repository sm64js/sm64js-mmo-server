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
use sm64js_api::{ChatHistory, ChatHistoryData};
use sm64js_auth::Identity;
use sm64js_ws::{Game, Room, Sm64JsServer, Sm64JsWsSession};

#[api_v2_operation(tags(Hidden))]
async fn ws_index(
    req: HttpRequest,
    stream: web::Payload,
    srv: web::Data<Addr<Sm64JsServer>>,
    identity: Identity,
) -> Result<HttpResponse, Error> {
    let auth_info = identity.get_auth_info();
    let ip = req.peer_addr();
    let real_ip = req
        .connection_info()
        .realip_remote_addr()
        .map(|ip| ip.to_string());
    ws::start(
        Sm64JsWsSession::new(srv.get_ref().clone(), auth_info, ip, real_ip),
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
    let chat_history: ChatHistoryData = web::Data::new(RwLock::new(ChatHistory::default()));
    let rooms = Room::init_rooms();
    let server = Sm64JsServer::new(chat_history.clone(), rooms.clone()).start();
    Game::run(rooms.clone());

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
            .wrap(middleware::Logger::default())
            .with_json_spec_at("/apispec")
            .service(web::resource("/ws/").to(ws_index))
            .service(sm64js_api::service())
            .wrap(sm64js_auth::Auth)
            .wrap(
                CookieSession::signed(&[0; 32])
                    .name("sm64js")
                    .path("/")
                    .max_age(3600 * 24 * 7)
                    .secure(false),
            )
            .build()
            .service(actix_files::Files::new("/apidoc", "./openapi").index_file("index.html"))
            .service(actix_files::Files::new("/", DIST_FOLDER).index_file("index.html"))
    })
    .bind("0.0.0.0:3060")?
    .run()
    .await
}
