use secrecy::SecretString;
use sqlx::Row;

use crate::types::ErrorTranslationKey;

#[derive(serde::Deserialize, Debug)]
pub struct LoginUser {
    email: String,
    password: SecretString,
}

#[tracing::instrument(name = "Logging a user in", skip( pool, user, session), fields(user_email = %user.email))]
#[actix_web::post("/login/")]
async fn login_user(
    pool: actix_web::web::Data<sqlx::postgres::PgPool>,
    user: actix_web::web::Json<LoginUser>,
    session: actix_session::Session,
) -> actix_web::HttpResponse {
    match get_user_who_is_active(&pool, &user.email).await {
        Ok(loggedin_user) => match tokio::task::spawn_blocking(move || {
            crate::utils::auth::password::verify_password(
                &loggedin_user.password_hash.unwrap(),
                &user.password,
            )
        })
        .await
        .expect("Unable to unwrap JoinError.")
        {
            Ok(_) => {
                tracing::event!(target: "backend", tracing::Level::INFO, "User logged in successfully.");
                session.renew();
                session
                    .insert(crate::types::USER_ID_KEY, loggedin_user.id)
                    .expect("`user_id` cannot be inserted into session");
                session
                    .insert(crate::types::USER_EMAIL_KEY, &loggedin_user.email)
                    .expect("`user_email` cannot be inserted into session");

                actix_web::HttpResponse::Ok().json(crate::types::UserVisible {
                    id: loggedin_user.id,
                    email: loggedin_user.email,
                    is_active: loggedin_user.is_active,
                    is_admin: loggedin_user.is_admin,
                })
            }
            Err(e) => {
                tracing::event!(target: "argon2",tracing::Level::ERROR, "Failed to authenticate user: {:#?}", e);
                actix_web::HttpResponse::BadRequest().json(crate::types::ErrorResponse {
                    error: "Email and password do not match".to_string(),
                    translation_key: ErrorTranslationKey::UsernamePasswordMismatch,
                })
            }
        },
        Err(e) => {
            tracing::event!(target: "sqlx",tracing::Level::ERROR, "User not found:{:#?}", e);
            actix_web::HttpResponse::NotFound().json(crate::types::ErrorResponse {
                error: "A user with these details does not exist. If you registered with these details, ensure you activate your account by clicking on the link sent to your e-mail address".to_string(),
                translation_key: ErrorTranslationKey::UserDoesntExist
            })
        }
    }
}

#[tracing::instrument(name = "Getting a user from DB.", skip(pool, email),fields(user_email = %email))]
async fn get_user_who_is_active(
    pool: &sqlx::postgres::PgPool,
    email: &String,
) -> Result<crate::types::User, sqlx::Error> {
    match sqlx::query("SELECT id, email, password, is_admin FROM users WHERE email = $1 AND is_active = TRUE AND password IS NOT NULL")
        .bind(email)
        .map(|row: sqlx::postgres::PgRow| crate::types::User {
            id: row.get("id"),
            email: row.get("email"),
            password_hash: row.get::<Option<String>, &str>("password").map(SecretString::from),
            is_active: true,
            is_admin: row.get("is_admin"),
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
