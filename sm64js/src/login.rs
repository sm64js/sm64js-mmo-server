use actix_http::{body::Body, client::SendRequestError};
use actix_session::Session;
use actix_web::{dev, error::ResponseError, http::StatusCode, HttpResponse};
use awc::{error::JsonPayloadError, SendClientRequest};
use paperclip::actix::{api_v2_errors, api_v2_operation, web, Apiv2Schema, Mountable};
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Deserialize)]
struct Login {
    code: String,
}

#[derive(Debug, Serialize)]
struct GoogleOAuthRequest {
    client_id: String,
    client_secret: String,
    code: String,
    grant_type: String,
    redirect_uri: String,
}

#[derive(Debug, Deserialize)]
struct GoogleOAuthResponse {
    id_token: String,
    expires_in: i64,
}

pub fn service() -> impl dev::HttpServiceFactory + Mountable {
    web::scope("/login")
        .service(web::resource("").route(web::post().to(login)))
        .service(web::resource("/google").route(web::post().to(login_with_google)))
        .service(web::resource("/discord").route(web::post().to(login_with_discord)))
}

#[api_v2_operation(tags(Auth))]
async fn login() -> String {
    // TODO persist session
    todo!()
}

pub static GOOGLE_CLIENT_ID: &str =
    "1000892686951-dkp1vpqohmbq64h7jiiop9v6ic4t1mul.apps.googleusercontent.com";

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

#[api_v2_operation(tags(Auth))]
async fn login_with_google(
    json: web::Json<Login>,
    _session: Session,
) -> Result<web::Json<AuthorizedUserMessage>, LoginError> {
    let req = GoogleOAuthRequest {
        client_id: GOOGLE_CLIENT_ID.to_string(),
        client_secret: std::env::var("GOOGLE_CLIENT_SECRET").unwrap(),
        code: json.code.clone(),
        grant_type: "authorization_code".to_string(),
        redirect_uri: "http://localhost:3060".to_string(),
    };
    let request: SendClientRequest = awc::Client::default()
        .post("https://oauth2.googleapis.com/token")
        .send_form(&req);
    let mut response = request.await?;
    if !response.status().is_success() {
        return Err(LoginError::TokenExpired);
    };
    let response: GoogleOAuthResponse = response.json().await?;

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
    let _response: IdToken = response.json().await?;

    // TODO store session
    Ok(web::Json(AuthorizedUserMessage {
        username: None,
        code: 1,
        message: None,
    }))
}

#[derive(Apiv2Schema, Debug, Serialize)]
struct AuthorizedUserMessage {
    username: Option<String>,
    code: u8,
    message: Option<String>,
}

#[api_v2_operation(tags(Auth))]
async fn login_with_discord() -> String {
    // TODO
    todo!()
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
}

impl ResponseError for LoginError {
    fn error_response(&self) -> HttpResponse {
        let res = match *self {
            Self::SendRequest(_) => HttpResponse::new(StatusCode::INTERNAL_SERVER_ERROR),
            Self::TokenExpired => HttpResponse::new(StatusCode::BAD_REQUEST),
            Self::SerdeJson(_) => HttpResponse::new(StatusCode::BAD_REQUEST),
            Self::JsonPayload(_) => HttpResponse::new(StatusCode::BAD_REQUEST),
        };
        res.set_body(Body::from(format!("{}", self)))
    }
}
