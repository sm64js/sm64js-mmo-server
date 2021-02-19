use crate::schema::*;

#[derive(Queryable)]
pub struct Account {
    pub id: i32,
    pub username: Option<String>,
    pub discord_id: Option<i32>,
    pub google_id: Option<i32>,
}

#[derive(Insertable)]
#[table_name = "accounts"]
pub struct NewAccount {
    pub username: Option<String>,
    pub discord_username: Option<String>,
    pub discord_discriminator: Option<String>,
    pub google_sub: Option<String>,
}

#[derive(Queryable)]
pub struct DiscordAccount {
    pub username: String,
    pub discriminator: String,
    pub avatar: Option<String>,
    pub mfa_enabled: Option<bool>,
    pub locale: Option<String>,
    pub flags: Option<i32>,
    pub premium_type: Option<i16>,
    pub public_flags: Option<i32>,
    pub session: Option<i32>,
}

#[derive(Insertable)]
#[table_name = "discord_accounts"]
pub struct NewDiscordAccount {
    pub username: String,
    pub discriminator: String,
    pub avatar: Option<String>,
    pub mfa_enabled: Option<bool>,
    pub locale: Option<String>,
    pub flags: Option<i32>,
    pub premium_type: Option<i16>,
    pub public_flags: Option<i32>,
    pub session: Option<i32>,
}

#[derive(Queryable)]
pub struct GoogleAccount {
    pub id: i32,
    pub session: Option<i32>,
}

#[derive(Insertable)]
#[table_name = "google_accounts"]
pub struct NewGoogleAccount {
    pub session: Option<i32>,
}

#[derive(Queryable)]
pub struct DiscordSession {
    pub id: i32,
    pub access_token: String,
    pub token_type: String,
    pub expires_in: i64,
}

#[derive(Insertable)]
#[table_name = "discord_sessions"]
pub struct NewDiscordSession {
    pub access_token: String,
    pub token_type: String,
    pub expires_in: i64,
}

#[derive(Queryable)]
pub struct GoogleSession {
    pub id: i32,
    pub id_token: String,
    pub expires_in: i64,
}

#[derive(Insertable)]
#[table_name = "google_sessions"]
pub struct NewGoogleSession {
    pub id_token: String,
    pub expires_in: i64,
}
