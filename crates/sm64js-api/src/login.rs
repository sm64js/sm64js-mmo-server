use actix_http::{body::Body, client::SendRequestError};
use actix_session::Session;
use actix_web::{dev, error::ResponseError, http::StatusCode, HttpResponse};
use awc::{error::JsonPayloadError, SendClientRequest};
use paperclip::actix::{api_v2_errors, api_v2_operation, web, Apiv2Schema, Mountable};
use serde::{Deserialize, Serialize};
use sm64js_auth::Identity;
use sm64js_common::{DiscordGuildMember, DiscordUser};
use sm64js_db::DbPool;
use std::env;
use thiserror::Error;

pub static GOOGLE_CLIENT_ID: &str =
    "1000892686951-dkp1vpqohmbq64h7jiiop9v6ic4t1mul.apps.googleusercontent.com";
pub static DISCORD_CLIENT_ID: &str = "807123464414429184";

#[derive(Debug, Deserialize)]
struct Login {
    code: String,
}

#[derive(Debug, Serialize)]
struct OAuth2Request {
    client_id: String,
    client_secret: String,
    code: String,
    grant_type: String,
    redirect_uri: String,
    scopes: Option<String>,
}

#[derive(Debug, Deserialize)]
struct GoogleOAuth2Response {
    id_token: String,
    expires_in: i64,
}

#[derive(Debug, Deserialize)]
struct DiscordOAuth2Response {
    access_token: String,
    token_type: String,
    expires_in: i64,
}

#[derive(Apiv2Schema, Debug, Serialize)]
struct AuthorizedUserMessage {
    username: Option<String>,
    code: u8,
    message: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct IdToken {
    pub iss: String,
    pub sub: String,
    pub azp: String,
    pub aud: String,
    pub iat: String,
    pub exp: String,
    pub hd: Option<String>,
    pub email: Option<String>,
    pub email_verified: Option<String>,
    pub name: Option<String>,
    pub picture: Option<String>,
    pub given_name: Option<String>,
    pub family_name: Option<String>,
    pub locale: Option<String>,
}

pub fn service() -> impl dev::HttpServiceFactory + Mountable {
    web::scope("/login")
        .service(web::resource("").route(web::post().to(login)))
        .service(web::resource("/google").route(web::post().to(login_with_google)))
        .service(web::resource("/discord").route(web::post().to(login_with_discord)))
}

#[api_v2_operation(tags(Auth))]
async fn login(identity: Identity) -> Result<web::Json<AuthorizedUserMessage>, LoginError> {
    let account_info = identity.get_auth_info();
    let username = account_info.get_discord_username();

    Ok(web::Json(AuthorizedUserMessage {
        username,
        code: 1,
        message: None,
    }))
}

#[api_v2_operation(tags(Hidden))]
async fn login_with_discord(
    json: web::Json<Login>,
    pool: web::Data<DbPool>,
    session: Session,
) -> Result<web::Json<AuthorizedUserMessage>, LoginError> {
    let req = OAuth2Request {
        client_id: DISCORD_CLIENT_ID.to_string(),
        client_secret: env::var("DISCORD_CLIENT_SECRET").unwrap(),
        code: json.code.clone(),
        grant_type: "authorization_code".to_string(),
        redirect_uri: env::var("REDIRECT_URI").unwrap(),
        scopes: Some("identify".to_string()),
    };
    let request: SendClientRequest = awc::Client::default()
        .post("https://discord.com/api/oauth2/token")
        .send_form(&req);
    let mut response = request.await?;
    if !response.status().is_success() {
        return Err(LoginError::TokenExpired);
    };
    let response: DiscordOAuth2Response = response.json().await?;
    let access_token = response.access_token;
    let token_type = response.token_type;
    let expires_in = response.expires_in;

    let request: SendClientRequest = awc::Client::default()
        .get("https://discord.com/api/users/@me")
        .header(
            awc::http::header::AUTHORIZATION,
            format!("{} {}", token_type, access_token),
        )
        .send();
    let mut response = request.await?;
    if !response.status().is_success() {
        return Err(LoginError::TokenExpired);
    };
    let discord_user: DiscordUser = response.json().await?;
    let username = discord_user.username.clone();
    let discriminator = discord_user.discriminator.clone();

    let request: SendClientRequest = awc::Client::default()
        .get(format!(
            "https://discord.com/api/guilds/{}/members/{}",
            "755122837077098630", discord_user.id
        ))
        .header(
            awc::http::header::AUTHORIZATION,
            format!("{} {}", "Bot", env::var("DISCORD_BOT_TOKEN").unwrap(),),
        )
        .send();
    let mut response = request.await?;
    if !response.status().is_success() {
        return Err(LoginError::TokenExpired);
    };

    let guild_member: DiscordGuildMember = response.json().await?;

    let conn = pool.get().unwrap();
    let discord_session = sm64js_db::insert_discord_session(
        &conn,
        access_token,
        token_type,
        expires_in,
        discord_user,
        guild_member,
    )?;

    session.set("account_id", discord_session.discord_account_id)?;
    session.set("session_id", discord_session.id)?;
    session.set("token", discord_session.access_token)?;
    session.set("expires_at", discord_session.expires_at.timestamp())?;
    session.set("account_type", "discord")?;

    Ok(web::Json(AuthorizedUserMessage {
        username: Some(format!("{}#{}", username, discriminator)),
        code: 1,
        message: None,
    }))
}

#[api_v2_operation(tags(Hidden))]
async fn login_with_google(
    json: web::Json<Login>,
    pool: web::Data<DbPool>,
    session: Session,
) -> Result<web::Json<AuthorizedUserMessage>, LoginError> {
    let req = OAuth2Request {
        client_id: GOOGLE_CLIENT_ID.to_string(),
        client_secret: std::env::var("GOOGLE_CLIENT_SECRET").unwrap(),
        code: json.code.clone(),
        grant_type: "authorization_code".to_string(),
        redirect_uri: std::env::var("REDIRECT_URI").unwrap(),
        scopes: None,
    };
    let request: SendClientRequest = awc::Client::default()
        .post("https://oauth2.googleapis.com/token")
        .send_form(&req);
    let mut response = request.await?;
    if !response.status().is_success() {
        return Err(LoginError::TokenExpired);
    };
    let response: GoogleOAuth2Response = response.json().await?;
    let jwt_token = response.id_token.clone();

    let request: SendClientRequest = awc::Client::default()
        .get(&format!(
            "https://oauth2.googleapis.com/tokeninfo?id_token={}",
            response.id_token
        ))
        .send();
    let mut response = request.await?;
    if !response.status().is_success() {
        return Err(LoginError::TokenExpired);
    };
    let id_token: IdToken = response.json().await?;
    let expires_at = id_token.exp.parse::<i64>().unwrap();

    let conn = pool.get().unwrap();
    let google_session =
        sm64js_db::insert_google_session(&conn, jwt_token, expires_at, id_token.sub).unwrap();

    session.set("account_id", google_session.google_account_id)?;
    session.set("session_id", google_session.id)?;
    session.set("token", google_session.id_token)?;
    session.set("expires_at", google_session.expires_at.timestamp())?;
    session.set("account_type", "google")?;

    Ok(web::Json(AuthorizedUserMessage {
        username: None,
        code: 1,
        message: None,
    }))
}

#[api_v2_errors(code = 400, code = 500)]
#[derive(Debug, Error)]
enum LoginError {
    #[error("[SendRequest]: {0}")]
    SendRequest(#[from] SendRequestError),
    #[error("[TokenExpired]")]
    TokenExpired,
    #[error("[SerdeJson]: {0}")]
    SerdeJson(#[from] serde_json::Error),
    #[error("[JsonPayload]: {0}")]
    JsonPayload(#[from] JsonPayloadError),
    #[error("[HttpError]: {0}")]
    HttpError(#[from] actix_http::Error),
    #[error("[DbError]: {0}")]
    DbError(#[from] sm64js_db::DbError),
}

impl ResponseError for LoginError {
    fn error_response(&self) -> HttpResponse {
        let res = match self {
            Self::SendRequest(_) => HttpResponse::new(StatusCode::INTERNAL_SERVER_ERROR),
            Self::TokenExpired => HttpResponse::new(StatusCode::BAD_REQUEST),
            Self::SerdeJson(_) => HttpResponse::new(StatusCode::BAD_REQUEST),
            Self::JsonPayload(_) => HttpResponse::new(StatusCode::BAD_REQUEST),
            Self::HttpError(err) => return err.as_response_error().error_response(),
            Self::DbError(err) => return err.error_response(),
        };
        res.set_body(Body::from(format!("{}", self)))
    }
}
