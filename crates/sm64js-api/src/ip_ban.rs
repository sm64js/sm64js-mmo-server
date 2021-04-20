use std::{net::IpAddr, time::Duration};

use actix::prelude::*;
use actix_http::{body::Body, client::SendRequestError, http::StatusCode, ResponseError};
use actix_web::HttpResponse;
use chrono::Utc;
use paperclip::actix::{api_v2_errors, api_v2_operation, web, Apiv2Schema, NoContent};
use serde::Deserialize;
use serde_with::skip_serializing_none;
use sm64js_auth::{Identity, Permission};
use sm64js_db::DbPool;
use sm64js_ws::{KickClientByIpAddr, Sm64JsServer};
use thiserror::Error;

/// POST Ban IP address
#[api_v2_operation(tags(Moderation))]
pub async fn post_ban(
    query: web::Query<PostIpBan>,
    pool: web::Data<DbPool>,
    identity: Identity,
    srv: web::Data<Addr<Sm64JsServer>>,
) -> Result<NoContent, BanError> {
    let auth_info = identity.get_auth_info();

    let perm = if let Some(expires_in) = query.expires_in {
        let expires_in = chrono::Duration::from_std(expires_in)
            .unwrap_or_else(|_| chrono::Duration::milliseconds(0));
        Permission::TempBanAccount(expires_in)
    } else {
        Permission::PermBanAccount
    };
    if !auth_info.has_permission(&perm) {
        return Err(BanError::Unauthorized);
    }

    match query.ip.parse::<IpAddr>() {
        Ok(ip) => ip,
        Err(_) => return Err(BanError::IpAddrParse),
    };

    match srv
        .send(KickClientByIpAddr {
            ip: query.ip.clone(),
        })
        .await?
    {
        Ok(_) => {}
        Err(err) => {
            eprintln!("{:?}", err);
        }
    }

    let conn = pool.get().unwrap();

    let expires_at = query.expires_in.map(|exp| {
        Utc::now().naive_utc()
            + chrono::Duration::from_std(exp).unwrap_or_else(|_| chrono::Duration::milliseconds(0))
    });
    sm64js_db::ban_ip(&conn, query.ip.clone(), query.reason.clone(), expires_at)?;

    actix::spawn(async move {
        let message = format!(
            r"reason: {}
    expires_at: {}
            ",
            query.reason.clone().unwrap_or_default(),
            expires_at.map(|exp| exp.to_string()).unwrap_or_default()
        );
        let author = sm64js_common::DiscordRichEmbedAuthor {
            name: format!(
                "POST Ban IP address by {}",
                auth_info.get_discord_username().unwrap_or_default()
            ),
            url: None,
            icon_url: None,
        };
        let footer = Some(sm64js_common::DiscordRichEmbedFooter {
            text: query.ip.clone(),
        });
        sm64js_common::send_discord_message(
            "829813249520042066",
            None,
            message,
            None,
            author,
            footer,
        )
        .await;
    });

    Ok(NoContent)
}

#[skip_serializing_none]
#[derive(Apiv2Schema, Debug, Deserialize)]
pub struct PostIpBan {
    ip: String,
    reason: Option<String>,
    /// Parses duration for temp bans, e.g. "15days". See https://docs.rs/humantime/2.1.0/humantime/index.html
    ///
    /// Keep this empty for a permanent ban.
    /// Banning will overwrite an already existing ban, so if you want to unban someone, just set this to "0s"
    #[serde(default)]
    #[serde(with = "humantime_serde")]
    expires_in: Option<Duration>,
}

#[api_v2_errors(code = 400, code = 500)]
#[derive(Debug, Error)]
pub enum BanError {
    #[error("[Unauthorized]")]
    Unauthorized,
    #[error("[IpAddrParse]")]
    IpAddrParse,
    #[error("[SendRequest]: {0}")]
    SendRequest(#[from] SendRequestError),
    #[error("[MailboxError]: {0}")]
    Mailbox(#[from] MailboxError),
    #[error("[DbError]: {0}")]
    DbError(#[from] sm64js_db::DbError),
}

impl ResponseError for BanError {
    fn error_response(&self) -> HttpResponse {
        let res = match self {
            Self::Unauthorized => HttpResponse::new(StatusCode::UNAUTHORIZED),
            Self::IpAddrParse => HttpResponse::new(StatusCode::BAD_REQUEST),
            Self::SendRequest(_) => HttpResponse::new(StatusCode::INTERNAL_SERVER_ERROR),
            Self::Mailbox(_) => HttpResponse::new(StatusCode::INTERNAL_SERVER_ERROR),
            Self::DbError(err) => return err.error_response(),
        };
        res.set_body(Body::from(format!("{}", self)))
    }
}
