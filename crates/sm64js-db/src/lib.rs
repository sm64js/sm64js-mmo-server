#[macro_use]
extern crate diesel;

pub mod models;
pub mod schema;

use anyhow::Result;
use chrono::{prelude::*, Duration};
use diesel::{
    pg::{upsert::on_constraint, PgConnection},
    prelude::*,
};

pub fn insert_discord_session(
    conn: &PgConnection,
    access_token: String,
    token_type: String,
    expires_in: i64,
    new_account: models::NewDiscordAccount,
) -> Result<()> {
    use schema::discord_sessions;

    let mut account_exists = false;
    if let Some(account) = get_discord_account(conn, &new_account)? {
        account_exists = true;
        if let Some(session) = account.session {
            delete_discord_session(conn, session)?;
        }
    }

    let expires_at = Utc::now().naive_utc() + Duration::seconds(expires_in);
    let new_session = models::NewDiscordSession {
        access_token,
        token_type,
        expires_at,
    };
    let session: models::DiscordSession = diesel::insert_into(discord_sessions::table)
        .values(&new_session)
        .get_result(conn)?;
    upsert_discord_account(conn, new_account, session, account_exists)?;
    Ok(())
}

fn get_discord_account(
    conn: &PgConnection,
    account: &models::NewDiscordAccount,
) -> Result<Option<models::DiscordAccount>> {
    use schema::discord_accounts::dsl::*;

    match discord_accounts
        .find((&account.username, &account.discriminator))
        .first(conn)
    {
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

fn upsert_discord_account(
    conn: &PgConnection,
    mut account: models::NewDiscordAccount,
    session: models::DiscordSession,
    account_exists: bool,
) -> Result<()> {
    use schema::discord_accounts;

    account.session = Some(session.id);
    let account: models::DiscordAccount = diesel::insert_into(discord_accounts::table)
        .values(&account)
        .on_conflict(on_constraint("discord_accounts_pkey"))
        .do_update()
        .set(&account)
        .get_result(conn)?;
    if !account_exists {
        insert_account(conn, Some(account))?;
    }
    Ok(())
}

fn insert_account(
    conn: &PgConnection,
    discord_account: Option<models::DiscordAccount>,
) -> Result<()> {
    use schema::accounts;

    let new_account = if let Some(models::DiscordAccount {
        username: discord_username,
        discriminator: discord_discriminator,
        ..
    }) = discord_account
    {
        models::NewAccount {
            username: None,
            discord_username: Some(discord_username),
            discord_discriminator: Some(discord_discriminator),
            google_sub: None,
        }
    } else {
        todo!();
    };
    diesel::insert_into(accounts::table)
        .values(&new_account)
        .execute(conn)?;
    Ok(())
}
