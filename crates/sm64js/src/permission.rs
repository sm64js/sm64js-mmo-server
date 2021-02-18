use crate::Identity;

use actix_web::{
    dev::{self, Body},
    http::StatusCode,
    HttpResponse, ResponseError,
};
use paperclip::actix::{api_v2_errors, api_v2_operation, web, Apiv2Schema, Mountable};
use rand::{distributions::Alphanumeric, thread_rng, Rng};
use serde::{Deserialize, Serialize};
use std::{
    fs::{self, read_to_string},
    io,
    path::Path,
    sync::RwLock,
};
use strum::{EnumIter, IntoEnumIterator};
use thiserror::Error;

pub fn service() -> impl dev::HttpServiceFactory + Mountable {
    web::scope("/permission")
        .service(web::resource("").route(web::get().to(get_own_permissions)))
        .service(web::resource("/list").route(web::get().to(get_permission_list)))
        .service(
            web::resource("/token")
                .route(web::get().to(generate_token))
                .route(web::post().to(edit_token))
                .route(web::delete().to(delete_token)),
        )
}

#[api_v2_operation(tags(Permission))]
async fn get_own_permissions() -> Result<String, ()> {
    todo!()
}

#[api_v2_operation(tags(Permission))]
async fn get_permission_list() -> web::Json<Vec<PermissionValue>> {
    web::Json(PermissionValue::iter().collect())
}

/// Generate token
///
/// Generate a new token with given permissions.
/// Permissions can also be blank and added afterwards.
///
/// If no token exists yet, this function will generate a new admin token, which will have all permissions.
#[api_v2_operation(tags(Permission))]
async fn generate_token(data: Tokens, _: Identity) -> Result<web::Json<Token>, GenerateTokenError> {
    match Token::generate(data) {
        Ok(token) => Ok(web::Json(token)),
        Err(err) => Err(err),
    }
}

#[api_v2_operation(tags(Permission))]
async fn edit_token() -> Result<String, ()> {
    todo!()
}

#[api_v2_operation(tags(Permission))]
async fn delete_token() -> Result<String, ()> {
    todo!()
}

lazy_static! {
    pub static ref PERMISSIONS: Vec<Permission> = vec![
        Permission {
            name: PermissionValue::Token,
            level: PermissionLevel::Admin,
        },
        Permission {
            name: PermissionValue::GetChat,
            level: PermissionLevel::Mod,
        },
        Permission {
            name: PermissionValue::GetChatIp,
            level: PermissionLevel::Admin,
        },
        Permission {
            name: PermissionValue::BanPlayer,
            level: PermissionLevel::Mod,
        }
    ];
}

#[derive(Apiv2Schema, Clone, Debug, Deserialize, Serialize)]
pub struct Permission {
    name: PermissionValue,
    level: PermissionLevel,
}

#[derive(Apiv2Schema, Clone, Debug, Deserialize, EnumIter, Serialize)]
pub enum PermissionValue {
    Token,
    GetChat,
    GetChatIp,
    BanPlayer,
}

#[derive(Apiv2Schema, Clone, Debug, Deserialize, EnumIter, Serialize)]
pub enum PermissionLevel {
    Admin,
    Mod,
    User,
}

#[derive(Apiv2Schema, Clone, Debug, Deserialize, Serialize)]
pub struct Token {
    key: String,
    permissions: Vec<Permission>,
}

pub type Tokens = web::Data<RwLock<Vec<Token>>>;

impl Token {
    pub fn try_load() -> anyhow::Result<Tokens> {
        let token_path = Path::new("tokens.ron");
        if token_path.exists() {
            Ok(web::Data::new(RwLock::new(ron::from_str(
                &read_to_string(token_path)?,
            )?)))
        } else {
            Ok(web::Data::new(RwLock::new(vec![])))
        }
    }

    pub fn find(tokens: &Tokens, apikey: String) -> Option<Token> {
        tokens
            .read()
            .unwrap()
            .clone()
            .into_iter()
            .find(|token| token.key == apikey)
    }

    fn generate(tokens: Tokens) -> Result<Self, GenerateTokenError> {
        let token_path = Path::new("tokens.ron");
        let key: String = thread_rng()
            .sample_iter(&Alphanumeric)
            .take(30)
            .map(char::from)
            .collect();
        let token = Token {
            key,
            permissions: vec![],
        };
        tokens.write().unwrap().push(token.clone());
        fs::write(token_path, ron::to_string(&*tokens.read().unwrap())?)?;
        Ok(token)
    }
}

#[api_v2_errors(
    code = 401,
    description = "Unauthorized: \"TOKEN\" permission required",
    code = 500
)]
#[derive(Debug, Error)]
pub enum GenerateTokenError {
    #[error("[RonError]: {0}")]
    RonError(#[from] ron::Error),
    #[error("[IoError]: {0}")]
    IoError(#[from] io::Error),
}

impl ResponseError for GenerateTokenError {
    fn error_response(&self) -> HttpResponse {
        let res = match *self {
            GenerateTokenError::RonError(_) => HttpResponse::new(StatusCode::INTERNAL_SERVER_ERROR),
            GenerateTokenError::IoError(_) => HttpResponse::new(StatusCode::INTERNAL_SERVER_ERROR),
        };
        res.set_body(Body::from(format!("{}", self)))
    }
}
