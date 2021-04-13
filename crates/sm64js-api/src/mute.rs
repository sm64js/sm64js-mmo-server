use std::time::Duration;

use actix::prelude::*;
use actix_http::{body::Body, client::SendRequestError, http::StatusCode, ResponseError};
use actix_web::HttpResponse;
use chrono::Utc;
use paperclip::actix::{api_v2_errors, api_v2_operation, web, Apiv2Schema, NoContent};
use serde::Deserialize;
use serde_with::skip_serializing_none;
use sm64js_auth::{Identity, Permission};
use sm64js_db::DbPool;
use sm64js_env::REDIRECT_URI;
use thiserror::Error;

/// POST Mute player
#[api_v2_operation(tags(Moderation))]
pub async fn post_mute(
    query: web::Query<PostBan>,
    pool: web::Data<DbPool>,
    identity: Identity,
) -> Result<NoContent, MuteError> {
    let auth_info = identity.get_auth_info();

    let perm = if let Some(expires_in) = query.expires_in {
        let expires_in = chrono::Duration::from_std(expires_in)
            .unwrap_or_else(|_| chrono::Duration::milliseconds(0));
        Permission::TempMuteAccount(expires_in)
    } else {
        Permission::PermMuteAccount
    };
    if !auth_info.has_permission(&perm) {
        return Err(MuteError::Unauthorized);
    }

    let conn = pool.get().unwrap();
    let account = sm64js_db::get_account(&conn, query.account_id)?;
    let account_info = sm64js_db::get_account_info(&conn, account.id, true).unwrap();

    let expires_at = query.expires_in.map(|exp| {
        Utc::now().naive_utc()
            + chrono::Duration::from_std(exp).unwrap_or_else(|_| chrono::Duration::milliseconds(0))
    });
    sm64js_db::mute_account(&conn, query.reason.clone(), expires_at, account.id)?;

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
                "POST Mute player by {}",
                auth_info.get_discord_username().unwrap_or_default()
            ),
            url: Some(format!(
                "{}/api/account?account_id={}",
                REDIRECT_URI.get().unwrap(),
                account_info.account.id
            )),
            icon_url: Some(if let Some(discord) = &account_info.discord {
                if let Some(avatar) = &discord.avatar {
                    format!(
                        "https://cdn.discordapp.com/avatars/{}/{}.png?size=64",
                        discord.id, avatar
                    )
                } else {
                    "https://discord.com/assets/2c21aeda16de354ba5334551a883b481.png".to_string()
                }
            } else {
                "https://developers.google.com/identity/images/g-logo.png".to_string()
            }),
        };
        let footer = Some(sm64js_common::DiscordRichEmbedFooter {
            text: format!("#{}", account_info.account.id),
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
pub struct PostBan {
    /// You can either get the `account_id` from Discord's #in-game-chat
    /// or from the <a href="#get-/api/players">player list</a>
    account_id: i32,
    reason: Option<String>,
    /// Parses duration for temp mutes, e.g. "15days". See https://docs.rs/humantime/2.1.0/humantime/index.html
    ///
    /// Keep this empty for a permanent mute.
    /// Muting will overwrite an already existing mute, so if you want to unmute someone, just set this to "0s"
    #[serde(default)]
    #[serde(with = "humantime_serde")]
    expires_in: Option<Duration>,
}

#[api_v2_errors(code = 400, code = 500)]
#[derive(Debug, Error)]
pub enum MuteError {
    #[error("[Unauthorized]")]
    Unauthorized,
    #[error("[SendRequest]: {0}")]
    SendRequest(#[from] SendRequestError),
    #[error("[MailboxError]: {0}")]
    Mailbox(#[from] MailboxError),
    #[error("[DbError]: {0}")]
    DbError(#[from] sm64js_db::DbError),
}

impl ResponseError for MuteError {
    fn error_response(&self) -> HttpResponse {
        let res = match self {
            Self::Unauthorized => HttpResponse::new(StatusCode::UNAUTHORIZED),
            Self::SendRequest(_) => HttpResponse::new(StatusCode::INTERNAL_SERVER_ERROR),
            Self::Mailbox(_) => HttpResponse::new(StatusCode::INTERNAL_SERVER_ERROR),
            Self::DbError(err) => return err.error_response(),
        };
        res.set_body(Body::from(format!("{}", self)))
    }
}
