use std::time::Duration;

use actix::prelude::*;
use actix_http::{body::Body, client::SendRequestError, http::StatusCode, ResponseError};
use actix_web::{client::JsonPayloadError, HttpResponse};
use awc::SendClientRequest;
use chrono::Utc;
use paperclip::actix::{api_v2_errors, api_v2_operation, web, Apiv2Schema, NoContent};
use serde::Deserialize;
use serde_with::skip_serializing_none;
use sm64js_auth::{Identity, Permission};
use sm64js_db::{models::Geolocation, DbPool};
use sm64js_ws::{KickClientByAccountId, Sm64JsServer};
use thiserror::Error;

/// POST Ban player
#[api_v2_operation(tags(Moderation))]
pub async fn post_ban(
    query: web::Query<PostBan>,
    pool: web::Data<DbPool>,
    identity: Identity,
    srv: web::Data<Addr<Sm64JsServer>>,
) -> Result<NoContent, BanError> {
    let auth_info = identity.get_auth_info();

    let perm = if query.expires_in.is_some() {
        Permission::TempBanAccount
    } else {
        Permission::PermBanAccount
    };
    if !auth_info.has_permission(&perm) {
        return Err(BanError::Unauthorized);
    }

    match srv
        .send(KickClientByAccountId {
            account_id: query.account_id,
        })
        .await?
    {
        Ok(_) => {}
        Err(err) => {
            eprintln!("{:?}", err);
        }
    }

    let conn = pool.get().unwrap();
    let account = sm64js_db::get_account(&conn, query.account_id)?;

    let geolocation: Option<Geolocation> = {
        let mut ip = account.last_ip.clone();
        if ip == "127.0.0.1" {
            ip = "".to_string();
        }
        let request: SendClientRequest = awc::Client::default()
            .get(format!("http://ip-api.com/json/{}?fields=205814", ip))
            .send();
        let mut response = request.await?;
        if !response.status().is_success() {
            None
        } else {
            Some(response.json().await?)
        }
    };

    let expires_at = query.expires_in.map(|exp| {
        Utc::now().naive_utc()
            + chrono::Duration::from_std(exp).unwrap_or_else(|_| chrono::Duration::milliseconds(0))
    });
    sm64js_db::ban_account(
        &conn,
        geolocation,
        account.last_ip,
        query.reason.clone(),
        expires_at,
        Some(account.id),
    )?;

    Ok(NoContent)
}

#[skip_serializing_none]
#[derive(Apiv2Schema, Debug, Deserialize)]
pub struct PostBan {
    account_id: i32,
    reason: Option<String>,
    // #[serde(with = "humantime_serde")]
    expires_in: Option<Duration>,
}

#[api_v2_errors(code = 400, code = 500)]
#[derive(Debug, Error)]
pub enum BanError {
    #[error("[Unauthorized]")]
    Unauthorized,
    #[error("[SendRequest]: {0}")]
    SendRequest(#[from] SendRequestError),
    #[error("[JsonPayload]: {0}")]
    JsonPayload(#[from] JsonPayloadError),
    #[error("[MailboxError]: {0}")]
    Mailbox(#[from] MailboxError),
    #[error("[DbError]: {0}")]
    DbError(#[from] sm64js_db::DbError),
}

impl ResponseError for BanError {
    fn error_response(&self) -> HttpResponse {
        let res = match self {
            Self::Unauthorized => HttpResponse::new(StatusCode::UNAUTHORIZED),
            Self::SendRequest(_) => HttpResponse::new(StatusCode::INTERNAL_SERVER_ERROR),
            Self::JsonPayload(_) => HttpResponse::new(StatusCode::BAD_REQUEST),
            Self::Mailbox(_) => HttpResponse::new(StatusCode::INTERNAL_SERVER_ERROR),
            Self::DbError(err) => return err.error_response(),
        };
        res.set_body(Body::from(format!("{}", self)))
    }
}
