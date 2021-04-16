use crate::schema::*;

use chrono::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Default)]
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

#[derive(Clone, Debug, Default, Identifiable, Queryable)]
pub struct Account {
    pub id: i32,
    pub username: Option<String>,
    pub last_ip: String,
}

#[derive(Insertable)]
#[table_name = "accounts"]
pub struct NewAccount {
    pub username: Option<String>,
    pub last_ip: String,
}

#[derive(AsChangeset)]
#[table_name = "accounts"]
pub struct UpdateAccount {
    pub username: Option<String>,
    pub last_ip: Option<String>,
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
    pub nick: Option<String>,
    pub roles: Vec<String>,
    pub joined_at: String,
    pub premium_since: Option<String>,
    pub deaf: bool,
    pub mute: bool,
    pub account_id: i32,
}

#[allow(clippy::from_over_into)]
impl Into<sm64js_common::DiscordAccount> for DiscordAccount {
    fn into(self) -> sm64js_common::DiscordAccount {
        sm64js_common::DiscordAccount {
            id: self.id,
            username: self.username,
            discriminator: self.discriminator,
            avatar: self.avatar,
            mfa_enabled: self.mfa_enabled,
            locale: self.locale,
            flags: self.flags,
            premium_type: self.premium_type,
            public_flags: self.public_flags,
            nick: self.nick,
            roles: self.roles,
            joined_at: self.joined_at,
            premium_since: self.premium_since,
            deaf: self.deaf,
            mute: self.mute,
        }
    }
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
    pub nick: Option<String>,
    pub roles: Vec<String>,
    pub joined_at: String,
    pub premium_since: Option<String>,
    pub deaf: bool,
    pub mute: bool,
}

#[derive(AsChangeset, Associations, Clone, Debug, Identifiable, Insertable, Queryable)]
#[primary_key(sub)]
#[belongs_to(Account)]
pub struct GoogleAccount {
    pub sub: String,
    pub account_id: i32,
}

#[allow(clippy::from_over_into)]
impl Into<sm64js_common::GoogleAccount> for GoogleAccount {
    fn into(self) -> sm64js_common::GoogleAccount {
        sm64js_common::GoogleAccount { sub: self.sub }
    }
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

// TODO implement Display trait for better human readable error message on ban
#[derive(Associations, Clone, Debug, Identifiable, Queryable, Serialize)]
#[belongs_to(Account)]
#[serde(rename_all = "camelCase")]
pub struct Ban {
    #[serde(skip_serializing)]
    pub id: i32,
    pub ip: String,
    pub reason: Option<String>,
    pub expires_at: Option<NaiveDateTime>,
    pub account_id: Option<i32>,
}

// TODO implement Display trait for better human readable error message on ban
#[derive(Clone, Debug, Insertable)]
#[table_name = "bans"]
pub struct NewBan {
    pub ip: String,
    pub reason: Option<String>,
    pub expires_at: Option<NaiveDateTime>,
    pub account_id: Option<i32>,
}

#[derive(
    AsChangeset, Associations, Clone, Debug, Identifiable, Insertable, Queryable, Serialize,
)]
#[serde(rename_all = "camelCase")]
#[primary_key(ip)]
pub struct IpBan {
    pub ip: String,
    pub reason: Option<String>,
    pub expires_at: Option<NaiveDateTime>,
}

#[derive(Associations, Clone, Debug, Identifiable, Insertable, Queryable, Serialize)]
#[belongs_to(Account)]
#[serde(rename_all = "camelCase")]
pub struct Mute {
    #[serde(skip_serializing)]
    pub id: i32,
    pub reason: Option<String>,
    pub expires_at: Option<NaiveDateTime>,
    pub account_id: i32,
}

// TODO implement Display trait for better human readable error message on ban
#[derive(Clone, Debug, Insertable)]
#[table_name = "mutes"]
pub struct NewMute {
    pub reason: Option<String>,
    pub expires_at: Option<NaiveDateTime>,
    pub account_id: i32,
}

#[derive(Associations, Clone, Debug, Identifiable, Insertable, Queryable)]
#[belongs_to(DiscordSession, GoogleSession)]
pub struct Geolocation {
    pub id: i32,
    pub query: String,
    pub country_code: String,
    pub region: String,
    pub city: String,
    pub zip: String,
    pub lat: f64,
    pub lon: f64,
    pub timezone: String,
    pub isp: String,
    pub mobile: bool,
    pub proxy: bool,
    pub discord_session_id: Option<i32>,
    pub google_session_id: Option<i32>,
    pub ban_id: Option<i32>,
}

#[derive(Clone, Debug, Deserialize, Insertable)]
#[serde(rename_all = "camelCase")]
#[table_name = "geolocations"]
pub struct NewGeolocation {
    pub query: String,
    pub country_code: String,
    pub region: String,
    pub city: String,
    pub zip: String,
    pub lat: f64,
    pub lon: f64,
    pub timezone: String,
    pub isp: String,
    pub mobile: bool,
    pub proxy: bool,
    pub discord_session_id: Option<i32>,
    pub google_session_id: Option<i32>,
    pub ban_id: Option<i32>,
}
