#[derive(Queryable)]
pub struct Account {
    pub id: i32,
    pub username: Option<String>,
    pub discord_id: Option<i32>,
    pub google_id: Option<i32>,
}

#[derive(Queryable)]
pub struct DiscordAccount {
    pub id: i32,
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

#[derive(Queryable)]
pub struct DiscordSession {
    pub id: i32,
    pub access_token: String,
    pub token_type: String,
    pub expires_in: String,
}

#[derive(Queryable)]
pub struct GoogleSession {
    pub id: i32,
    pub id_token: String,
    pub expires_in: String,
}
