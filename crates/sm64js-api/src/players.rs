use actix::Addr;
use actix_http::ResponseError;
use actix_web::{
    dev::{Body, HttpServiceFactory},
    http::StatusCode,
    HttpResponse,
};
use paperclip::actix::{api_v2_errors, api_v2_operation, web, Apiv2Schema, Mountable};
use serde::Deserialize;
use sm64js_auth::{Identity, Permission};
use sm64js_common::PlayerInfo;
use sm64js_ws::Sm64JsServer;
use thiserror::Error;

pub fn service() -> impl HttpServiceFactory + Mountable {
    web::scope("/players").service(web::resource("").route(web::get().to(get_players)))
}

/// GET Player list
///
/// Returns the list of currently online players.
#[api_v2_operation(tags(PlayerInfo))]
async fn get_players(
    query: web::Query<GetPlayers>,
    identity: Identity,
    srv: web::Data<Addr<Sm64JsServer>>,
) -> Result<web::Json<Vec<PlayerInfo>>, GetPlayerError> {
    let auth_info = identity.get_auth_info();
    if auth_info.has_permission(&Permission::GetPlayerList) {
        Ok(web::Json(
            srv.send(sm64js_ws::GetPlayers {
                include_chat: query.include_chat,
            })
            .await
            .map_err(|e| anyhow!(e))?,
        ))
    } else {
        Err(GetPlayerError::Unauthorized)
    }
}

#[derive(Apiv2Schema, Debug, Deserialize)]
pub struct GetPlayers {
    /// Append last x chat messages
    include_chat: Option<u32>,
}

#[api_v2_errors(code = 401)]
#[derive(Debug, Error)]
enum GetPlayerError {
    #[error("[Unauthorized]")]
    Unauthorized,
    #[error("[Anyhow]: {0}")]
    Anyhow(#[from] anyhow::Error),
}

impl ResponseError for GetPlayerError {
    fn error_response(&self) -> HttpResponse {
        let res = match *self {
            Self::Unauthorized => HttpResponse::new(StatusCode::UNAUTHORIZED),
            Self::Anyhow(_) => HttpResponse::new(StatusCode::INTERNAL_SERVER_ERROR),
        };
        res.set_body(Body::from(format!("{}", self)))
    }
}
