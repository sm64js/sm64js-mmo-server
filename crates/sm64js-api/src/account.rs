use actix_http::ResponseError;
use actix_web::{
    dev::{Body, HttpServiceFactory},
    http::StatusCode,
    HttpResponse,
};
use paperclip::actix::{api_v2_errors, api_v2_operation, web, Apiv2Schema, Mountable};
use serde::Deserialize;
use sm64js_auth::{Identity, Permission};
use sm64js_common::AccountInfo;
use sm64js_db::DbPool;
use thiserror::Error;

pub fn service() -> impl HttpServiceFactory + Mountable {
    web::scope("/account").service(web::resource("").route(web::get().to(get_account_info)))
}

/// GET Account info
#[api_v2_operation(tags(PlayerInfo))]
async fn get_account_info(
    query: web::Query<GetAccount>,
    identity: Identity,
    pool: web::Data<DbPool>,
) -> Result<web::Json<AccountInfo>, GetAccountError> {
    let auth_info = identity.get_auth_info();
    if auth_info.has_permission(&Permission::GetAccount) {
        let extended_info = auth_info.has_permission(&Permission::GetAccountExt);
        let conn = pool.get().unwrap();
        if let Some(mut account_info) =
            sm64js_db::get_account_info(&conn, query.account_id, extended_info)
        {
            if !auth_info.has_permission(&Permission::SeeIp) {
                account_info.account.last_ip = None;
            }
            Ok(web::Json(account_info))
        } else {
            Err(GetAccountError::NotFound)
        }
    } else {
        Err(GetAccountError::Unauthorized)
    }
}

#[derive(Apiv2Schema, Debug, Deserialize)]
pub struct GetAccount {
    /// You can either get the `account_id` from Discord's #in-game-chat
    /// or from the <a href="#get-/api/players">player list</a>
    account_id: i32,
}

#[api_v2_errors(code = 401)]
#[derive(Debug, Error)]
enum GetAccountError {
    #[error("[Unauthorized]")]
    Unauthorized,
    #[error("[NotFound]")]
    NotFound,
    #[error("[Anyhow]: {0}")]
    Anyhow(#[from] anyhow::Error),
}

impl ResponseError for GetAccountError {
    fn error_response(&self) -> HttpResponse {
        let res = match *self {
            Self::Unauthorized => HttpResponse::new(StatusCode::UNAUTHORIZED),
            Self::NotFound => HttpResponse::new(StatusCode::NOT_FOUND),
            Self::Anyhow(_) => HttpResponse::new(StatusCode::INTERNAL_SERVER_ERROR),
        };
        res.set_body(Body::from(format!("{}", self)))
    }
}
