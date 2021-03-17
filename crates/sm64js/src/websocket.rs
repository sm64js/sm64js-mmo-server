use actix::Addr;
use actix_web::{dev::Body, http::StatusCode, HttpRequest, HttpResponse, ResponseError};
use actix_web_actors::ws;
use paperclip::actix::{api_v2_errors, api_v2_operation, web};
use sm64js_auth::Identity;
use sm64js_ws::{Sm64JsServer, Sm64JsWsSession};
use thiserror::Error;

#[api_v2_operation(tags(Hidden))]
pub async fn index(
    req: HttpRequest,
    stream: web::Payload,
    srv: web::Data<Addr<Sm64JsServer>>,
    identity: Identity,
) -> Result<HttpResponse, WsError> {
    let auth_info = identity.get_auth_info();
    let ip = req.peer_addr().ok_or(WsError::IpRequired)?;
    let real_ip = req
        .connection_info()
        .realip_remote_addr()
        .map(|ip| ip.to_string());
    Ok(ws::start(
        Sm64JsWsSession::new(srv.get_ref().clone(), auth_info, ip, real_ip),
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
}

impl ResponseError for WsError {
    fn error_response(&self) -> HttpResponse {
        let res = match *self {
            Self::IpRequired => HttpResponse::new(StatusCode::BAD_REQUEST),
            Self::Actix(_) => HttpResponse::new(StatusCode::INTERNAL_SERVER_ERROR),
        };
        res.set_body(Body::from(format!("{}", self)))
    }
}
