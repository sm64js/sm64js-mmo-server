use actix_http::{body::Body, client::SendRequestError};
use actix_session::Session;
use actix_web::{client::Client, dev, error::ResponseError, http::StatusCode, HttpResponse};
use awc::{error::JsonPayloadError, SendClientRequest};
use paperclip::actix::{api_v2_errors, api_v2_operation, web, Mountable};
use serde::Deserialize;
use thiserror::Error;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Login {
    token_obj: TokenObj,
}

#[derive(Debug, Deserialize)]
struct TokenObj {
    id_token: String,
    expires_at: i64,
}

pub fn service() -> impl dev::HttpServiceFactory + Mountable {
    web::scope("/login")
        .service(web::resource("").route(web::post().to(login)))
        .service(web::resource("/google").route(web::post().to(login_with_google)))
        .service(web::resource("/discord").route(web::post().to(login_with_discord)))
}

#[api_v2_operation(tags(Auth))]
fn login() -> HttpResponse {
    // TODO persist session
    todo!()
}

pub static GOOGLE_CLIENT_ID: &str =
    "1000892686951-dkp1vpqohmbq64h7jiiop9v6ic4t1mul.apps.googleusercontent.com";

#[derive(Debug, Deserialize)]
pub struct IdInfo {
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
    client: web::Data<Client>,
    _session: Session,
) -> Result<web::Json<()>, LoginError> {
    let id_token = json.token_obj.id_token.clone();
    let request: SendClientRequest = client
        .get(&format!(
            "https://oauth2.googleapis.com/tokeninfo?id_token={}",
            id_token
        ))
        .send();
    let mut response = request.await?;

    // TODO handle bad status codes
    match response.status() {
        x if x.is_success() => {}
        x if x.is_client_error() => {}
        x if x.is_server_error() => {}
        _ => {}
    };
    let id_info: IdInfo = response.json().await?;
    if GOOGLE_CLIENT_ID != id_info.aud {
        Err(LoginError::ClientIdInvalid(id_info.aud).into())
    } else {
        // TODO store session and send AuthorizedUserMsg
        todo!()
    }
}

#[api_v2_operation(tags(Auth))]
fn login_with_discord() -> HttpResponse {
    // TODO
    todo!()
}

#[api_v2_errors(code = 400, code = 500)]
#[derive(Debug, Error)]
enum LoginError {
    #[error("[SendRequest]: {0}")]
    SendRequest(#[from] SendRequestError),
    #[error("[ClientIdInvalid]: {0}")]
    ClientIdInvalid(String),
    #[error("[SerdeJson]: {0}")]
    SerdeJson(#[from] serde_json::Error),
    #[error("[JsonPayload]: {0}")]
    JsonPayload(#[from] JsonPayloadError),
}

impl ResponseError for LoginError {
    fn error_response(&self) -> HttpResponse {
        let res = match *self {
            Self::SendRequest(_) => HttpResponse::new(StatusCode::INTERNAL_SERVER_ERROR),
            Self::ClientIdInvalid(_) => HttpResponse::new(StatusCode::BAD_REQUEST),
            Self::SerdeJson(_) => HttpResponse::new(StatusCode::BAD_REQUEST),
            Self::JsonPayload(_) => HttpResponse::new(StatusCode::BAD_REQUEST),
        };
        res.set_body(Body::from(format!("{}", self)))
    }
}
