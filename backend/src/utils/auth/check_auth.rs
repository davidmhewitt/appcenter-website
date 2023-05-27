use actix_session::Session;
use anyhow::Result;
use diesel::{ExpressionMethods, QueryDsl};
use diesel_async::{pooled_connection::bb8::Pool, AsyncPgConnection, RunQueryDsl};
use uuid::Uuid;

use common::models::*;

#[cfg_attr(not(coverage), tracing::instrument(name = "Check Auth", skip(session, pool)))]
pub async fn check_auth(
    session: Session,
    pool: &actix_web::web::Data<Pool<AsyncPgConnection>>,
) -> Option<Uuid> {
    let id = match session.get::<Uuid>(crate::types::USER_ID_KEY) {
        Ok(u) => match u {
            Some(u) => u,
            None => return None,
        },
        Err(_) => return None,
    };
    let email = match session.get::<String>(crate::types::USER_EMAIL_KEY) {
        Ok(u) => match u {
            Some(u) => u,
            None => return None,
        },
        Err(_) => return None,
    };

    if get_active_user_by_email_and_id(pool, &id, &email)
        .await
        .is_ok()
    {
        return Some(id);
    }

    None
}

#[cfg_attr(not(coverage), tracing::instrument(name = "Getting a user from DB.", skip(pool, user_email)))]
async fn get_active_user_by_email_and_id(
    pool: &Pool<AsyncPgConnection>,
    user_id: &Uuid,
    user_email: &String,
) -> Result<User> {
    use common::schema::users::dsl::*;

    let mut con = pool.get().await?;
    let result = users
        .filter(is_active.eq(true))
        .filter(id.eq(user_id))
        .filter(email.eq(user_email))
        .get_result::<User>(&mut con)
        .await?;

    Ok(result)
}
