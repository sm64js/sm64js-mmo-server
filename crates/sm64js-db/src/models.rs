use crate::schema::*;

use chrono::prelude::*;
use serde::Deserialize;

#[derive(Clone, Debug)]
pub struct AuthInfo {
    pub account: Account,
    pub discord: Option<DiscordAuthInfo>,
    pub google: Option<GoogleAuthInfo>,
}

#[derive(Clone, Debug)]
pub struct DiscordAuthInfo {
    pub account: DiscordAccount,
    pub session: DiscordSession,
}

#[derive(Clone, Debug)]
pub struct GoogleAuthInfo {
    pub account: GoogleAccount,
    pub session: GoogleSession,
}

#[derive(Clone, Debug, Identifiable, Queryable)]
pub struct Account {
    pub id: i32,
    pub username: Option<String>,
}

#[derive(Insertable)]
#[table_name = "accounts"]
pub struct NewAccount {
    pub username: Option<String>,
}

#[derive(AsChangeset, Associations, Clone, Debug, Identifiable, Insertable, Queryable)]
#[belongs_to(Account)]
pub struct DiscordAccount {
    pub id: String,
    pub username: String,
    pub discriminator: String,
    pub avatar: Option<String>,
    pub mfa_enabled: Option<bool>,
    pub locale: Option<String>,
    pub flags: Option<i32>,
    pub premium_type: Option<i16>,
    pub public_flags: Option<i32>,
    pub account_id: i32,
}

#[derive(Clone, Deserialize)]
pub struct NewDiscordAccount {
    pub id: String,
    pub username: String,
    pub discriminator: String,
    pub avatar: Option<String>,
    pub mfa_enabled: Option<bool>,
    pub locale: Option<String>,
    pub flags: Option<i32>,
    pub premium_type: Option<i16>,
    pub public_flags: Option<i32>,
}

#[derive(AsChangeset, Associations, Clone, Debug, Identifiable, Insertable, Queryable)]
#[primary_key(sub)]
#[belongs_to(Account)]
pub struct GoogleAccount {
    pub sub: String,
    pub account_id: i32,
}

#[derive(Clone, Deserialize)]
pub struct NewGoogleAccount {
    pub sub: String,
}

#[derive(Associations, Clone, Debug, Identifiable, Queryable)]
#[belongs_to(DiscordAccount)]
pub struct DiscordSession {
    pub id: i32,
    pub access_token: String,
    pub token_type: String,
    pub expires_at: NaiveDateTime,
    pub discord_account_id: String,
}

#[derive(Insertable)]
#[table_name = "discord_sessions"]
pub struct NewDiscordSession {
    pub access_token: String,
    pub token_type: String,
    pub expires_at: NaiveDateTime,
    pub discord_account_id: String,
}

#[derive(Associations, Clone, Debug, Identifiable, Queryable)]
#[belongs_to(GoogleAccount)]
pub struct GoogleSession {
    pub id: i32,
    pub id_token: String,
    pub expires_at: NaiveDateTime,
    pub google_account_id: String,
}

#[derive(Insertable)]
#[table_name = "google_sessions"]
pub struct NewGoogleSession {
    pub id_token: String,
    pub expires_at: NaiveDateTime,
    pub google_account_id: String,
}
