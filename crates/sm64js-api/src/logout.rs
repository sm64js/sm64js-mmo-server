use actix_session::Session;
use paperclip::actix::{api_v2_operation, web, NoContent};
use sm64js_auth::Identity;
use sm64js_db::{DbError, DbPool};

/// POST Delete cookie session
#[api_v2_operation(tags(Auth))]
pub async fn post_logout(
    pool: web::Data<DbPool>,
    identity: Identity,
    session: Session,
) -> Result<NoContent, DbError> {
    let auth_info = identity.get_auth_info();

    let conn = pool.get().unwrap();
    sm64js_db::delete_session(&conn, auth_info.into_inner())?;

    session.clear();

    Ok(NoContent)
}
