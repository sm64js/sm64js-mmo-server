use actix::Addr;
use actix_web::{dev::Body, http::StatusCode, HttpRequest, HttpResponse, ResponseError};
use actix_web_actors::ws;
use paperclip::actix::{api_v2_errors, api_v2_operation, web};
use sm64js_auth::Identity;
use sm64js_common::get_ip_from_req;
use sm64js_db::{models::Ban, DbPool};
use sm64js_ws::{Sm64JsServer, Sm64JsWsSession};
use thiserror::Error;

#[api_v2_operation(tags(Hidden))]
pub async fn index(
    req: HttpRequest,
    stream: web::Payload,
    srv: web::Data<Addr<Sm64JsServer>>,
    pool: web::Data<DbPool>,
    identity: Identity,
) -> Result<HttpResponse, WsError> {
    let auth_info = identity.get_auth_info();
    let conn = pool.get().unwrap();

    if let Some(ban) = sm64js_db::is_account_banned(&conn, auth_info.get_account_id())? {
        return Err(WsError::Banned(ban));
    }

    let ip = get_ip_from_req(&req).ok_or(WsError::IpRequired)?;
    Ok(ws::start(
        Sm64JsWsSession::new(srv.get_ref().clone(), auth_info, ip),
        &req,
        stream,
    )?)
}

#[api_v2_errors(code = 400)]
#[derive(Debug, Error)]
pub enum WsError {
    #[error("IP address could not be read")]
    IpRequired,
    #[error("[Actix]: {0}")]
    Actix(#[from] actix_web::Error),
    #[error("[Banned]: {0:?}")]
    Banned(Ban),
    #[error("[DbError]: {0}")]
    DbError(#[from] sm64js_db::DbError),
}

impl ResponseError for WsError {
    fn error_response(&self) -> HttpResponse {
        let res = match self {
            Self::IpRequired => HttpResponse::new(StatusCode::BAD_REQUEST),
            Self::Actix(_) => HttpResponse::new(StatusCode::INTERNAL_SERVER_ERROR),
            Self::Banned(_) => HttpResponse::new(StatusCode::FORBIDDEN),
            Self::DbError(err) => return err.error_response(),
        };
        res.set_body(Body::from(format!("{}", self)))
    }
}
