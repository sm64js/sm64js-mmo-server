use actix_session::Session;
use paperclip::actix::{api_v2_operation, web, NoContent};
use sm64js_auth::Identity;
use sm64js_db::DbPool;

#[api_v2_operation(tags(Chat))]
pub async fn post_logout(
    pool: web::Data<DbPool>,
    identity: Identity,
    session: Session,
) -> NoContent {
    let account_info = identity.get_account();

    let conn = pool.get().unwrap();
    sm64js_db::delete_session(&conn, account_info).unwrap();

    session.purge();

    NoContent
}
