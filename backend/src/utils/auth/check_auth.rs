use actix_session::Session;
use secrecy::SecretString;
use sqlx::Row;
use uuid::Uuid;

#[tracing::instrument(name = "Check Auth", skip(session, pool))]
pub async fn check_auth(
    session: Session,
    pool: &actix_web::web::Data<sqlx::postgres::PgPool>,
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

#[tracing::instrument(name = "Getting a user from DB.", skip(pool, email),fields(user_email = %email))]
async fn get_active_user_by_email_and_id(
    pool: &sqlx::postgres::PgPool,
    id: &Uuid,
    email: &String,
) -> Result<crate::types::User, sqlx::Error> {
    match sqlx::query("SELECT id, email, password, is_admin, date_joined FROM users WHERE id = $1 AND email = $2 AND is_active = TRUE")
        .bind(id)
        .bind(email)
        .map(|row: sqlx::postgres::PgRow| crate::types::User {
            id: row.get("id"),
            email: row.get("email"),
            password_hash: row.get::<Option<String>, &str>("password").map(SecretString::from),
            is_active: true,
            is_admin: row.get("is_admin"),
            date_joined: row.get("date_joined"),
        })
        .fetch_one(pool)
        .await
    {
        Ok(user) => Ok(user),
        Err(e) => {
            tracing::event!(target: "sqlx",tracing::Level::ERROR, "User not found in DB: {:#?}", e);
            Err(e)
        }
    }
}
