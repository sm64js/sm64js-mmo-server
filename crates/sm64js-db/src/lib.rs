#[macro_use]
extern crate diesel;

pub mod models;
pub mod schema;

use anyhow::Result;
use diesel::{pg::PgConnection, prelude::*};
use dotenv::dotenv;
use std::env;

pub struct Database {
    conn: PgConnection,
}

impl Database {
    pub fn try_connect() -> Result<Self> {
        dotenv().ok();

        let database_url = env::var("DATABASE_URL").expect("DATABASE_URL must be set");
        Ok(Database {
            conn: PgConnection::establish(&database_url)?,
        })
    }

    pub fn insert_discord_session(
        &self,
        access_token: String,
        token_type: String,
        expires_in: i64,
        new_account: models::NewDiscordAccount,
    ) -> Result<()> {
        use schema::discord_sessions;

        let new_session = models::NewDiscordSession {
            access_token,
            token_type,
            expires_in,
        };
        let session: models::DiscordSession = diesel::insert_into(discord_sessions::table)
            .values(&new_session)
            .get_result(&self.conn)?;
        self.upsert_discord_account(new_account, session)?;
        Ok(())
    }

    fn upsert_discord_account(
        &self,
        mut new_account: models::NewDiscordAccount,
        session: models::DiscordSession,
    ) -> Result<()> {
        use schema::discord_accounts;

        new_account.session = Some(session.id);
        let account: models::DiscordAccount = diesel::insert_into(discord_accounts::table)
            .values(&new_account)
            .get_result(&self.conn)?;
        self.insert_account(Some(account))?;
        Ok(())
    }

    fn insert_account(&self, discord_account: Option<models::DiscordAccount>) -> Result<()> {
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
            .execute(&self.conn)?;
        Ok(())
    }
}
