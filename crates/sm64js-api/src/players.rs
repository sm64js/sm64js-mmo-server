use actix_http::ResponseError;
use actix_web::{
    dev::{Body, HttpServiceFactory},
    http::StatusCode,
    HttpResponse,
};
use paperclip::actix::{api_v2_errors, api_v2_operation, web, Mountable};
use sm64js_auth::{Identity, Permission};
use sm64js_common::PlayerInfo;
use sm64js_ws::Sm64JsServer;
use thiserror::Error;

pub fn service() -> impl HttpServiceFactory + Mountable {
    web::scope("/players").service(web::resource("").route(web::get().to(get_players)))
}

/// GET Player list
#[api_v2_operation(tags(PlayerList))]
async fn get_players(
    identity: Identity,
    server: web::Data<Sm64JsServer>,
) -> Result<web::Json<Vec<PlayerInfo>>, GetPlayerError> {
    let auth_info = identity.get_auth_info();
    if auth_info.has_permission(&Permission::GetPlayerList) {
        Ok(web::Json(server.get_players()))
    } else {
        Err(GetPlayerError::Unauthorized)
    }
}

#[api_v2_errors(code = 401)]
#[derive(Debug, Error)]
enum GetPlayerError {
    #[error("[Unauthorized]")]
    Unauthorized,
}

impl ResponseError for GetPlayerError {
    fn error_response(&self) -> HttpResponse {
        let res = match *self {
            Self::Unauthorized => HttpResponse::new(StatusCode::UNAUTHORIZED),
        };
        res.set_body(Body::from(format!("{}", self)))
    }
}
