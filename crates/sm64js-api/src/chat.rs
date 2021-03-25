use actix_web::{dev::Body, http::StatusCode, HttpResponse, ResponseError};
use paperclip::actix::{api_v2_errors, api_v2_operation, web};
use sm64js_auth::{Identity, Permission};
use sm64js_common::{ChatHistoryData, ChatMessage, GetChat};
use thiserror::Error;

/// GET Chat history data
#[api_v2_operation(tags(Chat))]
pub async fn get_chat(
    query: web::Query<GetChat>,
    identity: Identity,
    chat_history: ChatHistoryData,
) -> Result<web::Json<Vec<ChatMessage>>, GetChatError> {
    let auth_info = identity.get_auth_info();
    if auth_info.has_permission(&Permission::ReadChatLog) {
        let with_ip = auth_info.has_permission(&Permission::SeeIp);
        Ok(web::Json(chat_history.read().get_messages(
            query.into_inner(),
            true,
            with_ip,
        )))
    } else {
        Err(GetChatError::Unauthorized)
    }
}

#[api_v2_errors(code = 401)]
#[derive(Debug, Error)]
pub enum GetChatError {
    #[error("[Unauthorized]")]
    Unauthorized,
}

impl ResponseError for GetChatError {
    fn error_response(&self) -> HttpResponse {
        let res = match *self {
            Self::Unauthorized => HttpResponse::new(StatusCode::UNAUTHORIZED),
        };
        res.set_body(Body::from(format!("{}", self)))
    }
}
