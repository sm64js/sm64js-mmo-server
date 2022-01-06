#[macro_use]
extern crate diesel;

pub mod models;
pub mod schema;

pub use models::{Account, AuthInfo, DiscordAuthInfo, GoogleAuthInfo};

pub type DbPool = r2d2::Pool<ConnectionManager<PgConnection>>;

use actix_session::Session;
use actix_web::{dev::Body, http::StatusCode, HttpResponse, ResponseError};
use chrono::{prelude::*, Duration};
use diesel::{
    pg::{upsert::on_constraint, PgConnection},
    prelude::*,
    r2d2::ConnectionManager,
};
use paperclip::actix::api_v2_errors;
use sm64js_common::{AccountInfo, DiscordAccount, DiscordGuildMember, DiscordUser};
#[cfg(debug_assertions)]
use sm64js_env::{
    DEV_ACCOUNT_ID, DEV_GOOGLE_ACCOUNT_ID, DEV_GOOGLE_SESSION_TOKEN, DEV_GOOGLE_TEST_USER,
};
use thiserror::Error;

type Result<T> = std::result::Result<T, DbError>;

pub fn insert_discord_session(
    conn: &PgConnection,
    access_token: String,
    token_type: String,
    expires_in: i64,
    discord_user: DiscordUser,
    guild_member: Option<DiscordGuildMember>,
    ip: String,
) -> Result<models::DiscordSession> {
    use schema::discord_sessions;

    let mut account_id = None;
    if let Some(account) = get_discord_account_if_exists(conn, &discord_user.id)? {
        if let Some(ban) = is_account_banned(conn, account.account_id)? {
            return Err(DbError::Banned(ban));
        }

        update_account(
            conn,
            account.account_id,
            &models::UpdateAccount {
                username: None,
                last_ip: Some(ip.clone()),
            },
        )?;
        account_id = Some(account.account_id);
        if let Ok(session) =
            models::DiscordSession::belonging_to(&account).first::<models::DiscordSession>(conn)
        {
            delete_discord_session(conn, session.id)?;
        }
    }

    let new_account = if let Some(guild_member) = guild_member {
        models::NewDiscordAccount {
            id: discord_user.id,
            username: discord_user.username,
            discriminator: discord_user.discriminator,
            avatar: discord_user.avatar,
            mfa_enabled: discord_user.mfa_enabled,
            locale: discord_user.locale,
            flags: discord_user.flags,
            premium_type: discord_user.premium_type,
            public_flags: discord_user.public_flags,
            nick: guild_member.nick,
            roles: guild_member.roles,
            joined_at: guild_member.joined_at,
            premium_since: guild_member.premium_since,
            deaf: guild_member.deaf,
            mute: guild_member.mute,
        }
    } else {
        models::NewDiscordAccount {
            id: discord_user.id,
            username: discord_user.username,
            discriminator: discord_user.discriminator,
            avatar: discord_user.avatar,
            mfa_enabled: discord_user.mfa_enabled,
            locale: discord_user.locale,
            flags: discord_user.flags,
            premium_type: discord_user.premium_type,
            public_flags: discord_user.public_flags,
            nick: None,
            roles: vec![],
            joined_at: "".to_string(),
            premium_since: None,
            deaf: false,
            mute: false,
        }
    };
    let discord_account_id = upsert_discord_account(conn, new_account, ip, account_id)?;

    let expires_at = Utc::now().naive_utc() + Duration::seconds(expires_in);
    let new_session = models::NewDiscordSession {
        access_token,
        token_type,
        expires_at,
        discord_account_id,
    };
    let session: models::DiscordSession = diesel::insert_into(discord_sessions::table)
        .values(&new_session)
        .get_result(conn)?;
    Ok(session)
}

pub fn insert_google_session(
    conn: &PgConnection,
    id_token: String,
    expires_at: i64,
    sub: String,
    ip: String,
) -> Result<models::GoogleSession> {
    use schema::google_sessions;

    let mut account_id = None;
    if let Some(account) = get_google_account_if_exists(conn, &sub)? {
        if let Some(ban) = is_account_banned(conn, account.account_id)? {
            return Err(DbError::Banned(ban));
        }

        update_account(
            conn,
            account.account_id,
            &models::UpdateAccount {
                username: None,
                last_ip: Some(ip.clone()),
            },
        )?;
        account_id = Some(account.account_id);
        if let Ok(session) =
            models::GoogleSession::belonging_to(&account).first::<models::GoogleSession>(conn)
        {
            delete_google_session(conn, session.id)?;
        }
    }
    let google_account_id = upsert_google_account(conn, sub, ip, account_id)?;

    let expires_at = Utc.timestamp(expires_at, 0).naive_utc();
    let new_session = models::NewGoogleSession {
        id_token,
        expires_at,
        google_account_id,
    };
    let session: models::GoogleSession = diesel::insert_into(google_sessions::table)
        .values(&new_session)
        .get_result(conn)?;
    Ok(session)
}

pub fn get_auth_info(conn: &PgConnection, req_session: &Session) -> Result<Option<AuthInfo>> {
    if let (Ok(Some(account_id)), Ok(Some(session_id)), Ok(Some(token)), Ok(Some(account_type))) = (
        req_session.get::<String>("account_id"),
        req_session.get::<i32>("session_id"),
        req_session.get::<String>("token"),
        req_session.get::<String>("account_type"),
    ) {
        match account_type.as_ref() {
            "discord" => {
                use schema::discord_sessions::dsl::*;

                let session = discord_sessions.find(session_id).first(conn);

                let session: models::DiscordSession = match session {
                    Ok(session) => session,
                    Err(diesel::result::Error::NotFound) => return Ok(None),
                    Err(err) => return Err(err.into()),
                };

                let is_expired = Utc::now().naive_utc() >= session.expires_at;
                if is_expired {
                    diesel::delete(discord_sessions.find(session_id)).execute(conn)?;
                    return Err(DbError::SessionExpired);
                }

                if session.access_token != token {
                    return Err(DbError::AccessTokenInvalid);
                }

                if session.discord_account_id != account_id {
                    return Err(DbError::AccountIdInvalid);
                }

                let discord_account = get_discord_account(conn, &account_id)?;
                let account = get_account(conn, discord_account.account_id)?;
                return Ok(Some(AuthInfo {
                    account,
                    discord: Some(models::DiscordAuthInfo {
                        account: discord_account,
                        session,
                    }),
                    google: None,
                }));
            }
            "google" => {
                use schema::google_sessions::dsl::*;

                let session = {
                    #[cfg(debug_assertions)]
                    {
                        if token == DEV_GOOGLE_SESSION_TOKEN {
                            Ok(models::GoogleSession {
                                id: DEV_ACCOUNT_ID,
                                id_token: DEV_GOOGLE_SESSION_TOKEN.to_string(),
                                expires_at: Utc::now().naive_utc() + Duration::weeks(1000),
                                google_account_id: DEV_GOOGLE_ACCOUNT_ID.to_string(),
                            })
                        } else {
                            google_sessions.find(session_id).first(conn)
                        }
                    }
                    #[cfg(not(debug_assertions))]
                    google_sessions.find(session_id).first(conn)
                };

                let session: models::GoogleSession = match session {
                    Ok(session) => session,
                    Err(diesel::result::Error::NotFound) => return Ok(None),
                    Err(err) => return Err(err.into()),
                };

                let is_expired = Utc::now().naive_utc() >= session.expires_at;
                if is_expired {
                    diesel::delete(google_sessions.find(session_id)).execute(conn)?;
                    return Err(DbError::SessionExpired);
                }

                if session.id_token != token {
                    return Err(DbError::AccessTokenInvalid);
                }

                if session.google_account_id != account_id {
                    return Err(DbError::AccountIdInvalid);
                }

                let google_account = get_google_account(conn, &account_id)?;
                let account = get_account(conn, google_account.account_id)?;
                return Ok(Some(AuthInfo {
                    account,
                    discord: None,
                    google: Some(models::GoogleAuthInfo {
                        account: google_account,
                        session,
                    }),
                }));
            }
            _ => {}
        }
    }

    Ok(None)
}

pub fn get_account_info(
    conn: &PgConnection,
    account_id: i32,
    extended_info: bool,
) -> Option<AccountInfo> {
    #[cfg(debug_assertions)]
    if account_id == DEV_ACCOUNT_ID {
        return Some(AccountInfo {
            account: sm64js_common::Account {
                id: account_id,
                last_ip: None,
                username: Some(DEV_GOOGLE_TEST_USER.to_string()),
                ..Default::default()
            },
            ..Default::default()
        });
    }

    use schema::accounts::dsl::*;

    let account = accounts.find(account_id).first(conn);

    let account: Account = match account {
        Ok(account) => account,
        Err(_) => return None,
    };

    let ban = is_account_banned(conn, account_id).ok().flatten();
    let mute = is_account_muted(conn, account_id).ok().flatten();

    let mut account_info = {
        let account = account.clone();
        AccountInfo {
            account: sm64js_common::Account {
                id: account.id,
                last_ip: Some(account.last_ip),
                username: account.username,
                is_banned: if ban.is_some() { Some(true) } else { None },
                banned_until: ban.clone().and_then(|b| b.expires_at),
                ban_reason: ban.and_then(|b| b.reason),
                is_muted: if mute.is_some() { Some(true) } else { None },
                muted_until: mute.clone().and_then(|m| m.expires_at),
                mute_reason: mute.and_then(|m| m.reason),
            },
            discord: None,
            google: None,
        }
    };

    if let Ok(discord) =
        models::DiscordAccount::belonging_to(&account).first::<models::DiscordAccount>(conn)
    {
        let mut discord: DiscordAccount = discord.into();
        if !extended_info {
            discord.mfa_enabled = None;
            discord.flags = None;
            discord.public_flags = None;
            discord.premium_since = None;
        }
        account_info.discord = Some(discord);
    }

    if let Ok(google) =
        models::GoogleAccount::belonging_to(&account).first::<models::GoogleAccount>(conn)
    {
        account_info.google = Some(google.into());
    }

    Some(account_info)
}

pub fn delete_session(conn: &PgConnection, auth_info: AuthInfo) -> Result<()> {
    match auth_info {
        AuthInfo {
            discord:
                Some(models::DiscordAuthInfo {
                    account: _,
                    session,
                }),
            ..
        } => delete_discord_session(conn, session.id)?,
        AuthInfo {
            google:
                Some(models::GoogleAuthInfo {
                    account: _,
                    session,
                }),
            ..
        } => delete_google_session(conn, session.id)?,
        _ => {}
    }
    Ok(())
}

pub fn get_account(conn: &PgConnection, account_id: i32) -> Result<models::Account> {
    #[cfg(debug_assertions)]
    if account_id == DEV_ACCOUNT_ID {
        return Ok(models::Account {
            id: account_id,
            username: None,
            last_ip: "0.0.0.0".to_string(),
        });
    }

    use schema::accounts::dsl::*;

    Ok(accounts.find(account_id).first(conn)?)
}

pub fn update_account(
    conn: &PgConnection,
    account_id: i32,
    update_account: &models::UpdateAccount,
) -> Result<()> {
    use schema::accounts::dsl::*;

    let account = accounts.find(account_id);
    diesel::update(account).set(update_account).execute(conn)?;

    Ok(())
}

pub fn ban_account(
    conn: &PgConnection,
    geolocation: Option<models::NewGeolocation>,
    ip: String,
    reason: Option<String>,
    expires_at: Option<NaiveDateTime>,
    account_id: Option<i32>,
) -> Result<models::Ban> {
    use schema::bans;

    if let Some(account_id) = account_id {
        if let Some(ban) = is_account_banned(conn, account_id)? {
            use schema::bans::dsl::*;
            diesel::delete(bans).filter(id.eq(ban.id)).execute(conn)?;
        }
    }

    let new_ban = models::NewBan {
        ip,
        reason,
        expires_at,
        account_id,
    };
    let ban: models::Ban = diesel::insert_into(bans::table)
        .values(&new_ban)
        .get_result(conn)?;

    if let Some(geolocation) = geolocation {
        add_geolocation(conn, geolocation)?;
    }

    Ok(ban)
}

pub fn ban_ip(
    conn: &PgConnection,
    ip: String,
    reason: Option<String>,
    expires_at: Option<NaiveDateTime>,
) -> Result<models::IpBan> {
    use schema::ip_bans;

    let ip_ban = models::IpBan {
        ip,
        reason,
        expires_at,
    };
    let ip_ban: models::IpBan = diesel::insert_into(ip_bans::table)
        .values(&ip_ban)
        .on_conflict(on_constraint("ip_bans_pkey"))
        .do_update()
        .set(&ip_ban)
        .get_result(conn)?;

    Ok(ip_ban)
}

pub fn mute_account(
    conn: &PgConnection,
    reason: Option<String>,
    expires_at: Option<NaiveDateTime>,
    account_id: i32,
) -> Result<models::Mute> {
    use schema::mutes;

    if let Some(ban) = is_account_muted(conn, account_id)? {
        use schema::mutes::dsl::*;
        diesel::delete(mutes).filter(id.eq(ban.id)).execute(conn)?;
    }

    let new_mute = models::NewMute {
        reason,
        expires_at,
        account_id,
    };
    let mute: models::Mute = diesel::insert_into(mutes::table)
        .values(&new_mute)
        .get_result(conn)?;

    Ok(mute)
}

pub fn is_account_banned(conn: &PgConnection, account_id: i32) -> Result<Option<models::Ban>> {
    let account = get_account(conn, account_id)?;

    if let Ok(bans) = models::Ban::belonging_to(&account).load::<models::Ban>(conn) {
        if bans.is_empty() {
            Ok(None)
        } else {
            let max_ban = if let Some(max_ban) = bans.iter().find(|ban| ban.expires_at.is_none()) {
                max_ban
            } else {
                bans.iter().max_by_key(|ban| ban.expires_at).unwrap()
            };
            for ban in bans.iter().filter(|filtered| filtered.id != max_ban.id) {
                use schema::bans::dsl::*;

                diesel::delete(bans).filter(id.eq(ban.id)).execute(conn)?;
            }
            if let Some(expires_at) = max_ban.expires_at {
                if Utc::now().naive_utc() > expires_at {
                    use schema::bans::dsl::*;

                    diesel::delete(bans)
                        .filter(id.eq(max_ban.id))
                        .execute(conn)?;
                    return Ok(None);
                }
            }
            Ok(Some(max_ban.clone()))
        }
    } else {
        Ok(None)
    }
}

pub fn is_ip_banned(conn: &PgConnection, ip: &str) -> Result<Option<models::IpBan>> {
    use schema::ip_bans;

    match ip_bans::table.find(ip).first(conn) {
        Ok(ban) => {
            let ban: models::IpBan = ban;
            if let Some(expires_at) = ban.expires_at {
                if Utc::now().naive_utc() > expires_at {
                    use schema::ip_bans::dsl::*;

                    diesel::delete(ip_bans)
                        .filter(ip.eq(ban.ip))
                        .execute(conn)?;
                    return Ok(None);
                }
            }

            Ok(Some(ban))
        }
        Err(diesel::result::Error::NotFound) => Ok(None),
        Err(err) => Err(err.into()),
    }
}

pub fn is_account_muted(conn: &PgConnection, account_id: i32) -> Result<Option<models::Mute>> {
    let account = get_account(conn, account_id)?;

    if let Ok(mutes) = models::Mute::belonging_to(&account).load::<models::Mute>(conn) {
        if mutes.is_empty() {
            Ok(None)
        } else {
            let max_mute =
                if let Some(max_mute) = mutes.iter().find(|mute| mute.expires_at.is_none()) {
                    max_mute
                } else {
                    mutes.iter().max_by_key(|mute| mute.expires_at).unwrap()
                };
            for mute in mutes.iter().filter(|filtered| filtered.id != max_mute.id) {
                use schema::mutes::dsl::*;

                diesel::delete(mutes).filter(id.eq(mute.id)).execute(conn)?;
            }
            if let Some(expires_at) = max_mute.expires_at {
                if Utc::now().naive_utc() > expires_at {
                    use schema::mutes::dsl::*;

                    diesel::delete(mutes)
                        .filter(id.eq(max_mute.id))
                        .execute(conn)?;
                    return Ok(None);
                }
            }
            Ok(Some(max_mute.clone()))
        }
    } else {
        Ok(None)
    }
}

fn add_geolocation(conn: &PgConnection, geolocation: models::NewGeolocation) -> Result<()> {
    use schema::geolocations;

    diesel::insert_into(geolocations::table)
        .values(&geolocation)
        .execute(conn)?;

    Ok(())
}

fn get_discord_account(conn: &PgConnection, id: &str) -> Result<models::DiscordAccount> {
    use schema::discord_accounts;

    Ok(discord_accounts::table.find(id).first(conn)?)
}

fn get_google_account(conn: &PgConnection, id: &str) -> Result<models::GoogleAccount> {
    #[cfg(debug_assertions)]
    if id == DEV_GOOGLE_ACCOUNT_ID {
        return Ok(models::GoogleAccount {
            account_id: DEV_ACCOUNT_ID,
            sub: DEV_GOOGLE_ACCOUNT_ID.to_string(),
        });
    }

    use schema::google_accounts;

    Ok(google_accounts::table.find(id).first(conn)?)
}

fn get_discord_account_if_exists(
    conn: &PgConnection,
    id: &str,
) -> Result<Option<models::DiscordAccount>> {
    use schema::discord_accounts;

    match discord_accounts::table.find(id).first(conn) {
        Ok(account) => Ok(Some(account)),
        Err(diesel::result::Error::NotFound) => Ok(None),
        Err(err) => Err(err.into()),
    }
}

fn get_google_account_if_exists(
    conn: &PgConnection,
    sub: &str,
) -> Result<Option<models::GoogleAccount>> {
    use schema::google_accounts;

    match google_accounts::table.find(sub).first(conn) {
        Ok(account) => Ok(Some(account)),
        Err(diesel::result::Error::NotFound) => Ok(None),
        Err(err) => Err(err.into()),
    }
}

fn delete_discord_session(conn: &PgConnection, key: i32) -> Result<()> {
    use schema::discord_sessions::dsl::*;

    diesel::delete(discord_sessions)
        .filter(id.eq(key))
        .execute(conn)?;
    Ok(())
}

fn delete_google_session(conn: &PgConnection, key: i32) -> Result<()> {
    use schema::google_sessions::dsl::*;

    diesel::delete(google_sessions)
        .filter(id.eq(key))
        .execute(conn)?;
    Ok(())
}

fn upsert_discord_account(
    conn: &PgConnection,
    discord_account: models::NewDiscordAccount,
    ip: String,
    account_id: Option<i32>,
) -> Result<String> {
    use schema::discord_accounts;

    let account_id = if let Some(account_id) = account_id {
        account_id
    } else {
        insert_account(conn, ip)?
    };
    let discord_account = models::DiscordAccount {
        id: discord_account.id,
        username: discord_account.username,
        discriminator: discord_account.discriminator,
        avatar: discord_account.avatar,
        mfa_enabled: discord_account.mfa_enabled,
        locale: discord_account.locale,
        flags: discord_account.flags,
        premium_type: discord_account.premium_type,
        public_flags: discord_account.public_flags,
        nick: discord_account.nick,
        roles: discord_account.roles,
        joined_at: discord_account.joined_at,
        premium_since: discord_account.premium_since,
        deaf: discord_account.deaf,
        mute: discord_account.mute,
        account_id,
    };

    let account: models::DiscordAccount = diesel::insert_into(discord_accounts::table)
        .values(&discord_account)
        .on_conflict(on_constraint("discord_accounts_pkey"))
        .do_update()
        .set(&discord_account)
        .get_result(conn)?;
    Ok(account.id)
}

fn upsert_google_account(
    conn: &PgConnection,
    sub: String,
    ip: String,
    account_id: Option<i32>,
) -> Result<String> {
    use schema::google_accounts;

    let account_id = if let Some(account_id) = account_id {
        account_id
    } else {
        insert_account(conn, ip)?
    };
    let google_account = models::GoogleAccount { sub, account_id };

    let account: models::GoogleAccount = diesel::insert_into(google_accounts::table)
        .values(&google_account)
        .on_conflict(on_constraint("google_accounts_pkey"))
        .do_update()
        .set(&google_account)
        .get_result(conn)?;
    Ok(account.sub)
}

fn insert_account(conn: &PgConnection, ip: String) -> Result<i32> {
    use schema::accounts;

    let new_account = models::NewAccount {
        username: None,
        last_ip: ip,
    };
    let account: models::Account = diesel::insert_into(accounts::table)
        .values(&new_account)
        .get_result(conn)?;
    Ok(account.id)
}

#[api_v2_errors(code = 500)]
#[derive(Debug, Error)]
pub enum DbError {
    #[error("Session expired")]
    SessionExpired,
    #[error("access_token does not match db entry")]
    AccessTokenInvalid,
    #[error("account_id does not match db entry")]
    AccountIdInvalid,
    #[error("[Banned]: {0:?}")]
    Banned(models::Ban),
    #[error("[Diesel]: {0}")]
    Diesel(#[from] diesel::result::Error),
}

impl ResponseError for DbError {
    fn error_response(&self) -> HttpResponse {
        let res = match self {
            Self::SessionExpired | Self::AccessTokenInvalid | Self::AccountIdInvalid => {
                HttpResponse::new(StatusCode::BAD_REQUEST)
            }
            Self::Banned(_) => HttpResponse::new(StatusCode::FORBIDDEN),
            Self::Diesel(_) => HttpResponse::new(StatusCode::INTERNAL_SERVER_ERROR),
        };
        res.set_body(Body::from(format!("{}", self)))
    }
}
