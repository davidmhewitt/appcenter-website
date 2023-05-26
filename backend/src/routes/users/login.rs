use actix_web::HttpResponse;
use anyhow::Result;
use diesel::{ExpressionMethods, QueryDsl};
use diesel_async::{pooled_connection::bb8::Pool, AsyncPgConnection, RunQueryDsl};
use secrecy::SecretString;

use common::models::User;

use crate::types::ErrorTranslationKey;

#[derive(serde::Deserialize, Debug)]
pub struct LoginUser {
    email: String,
    password: SecretString,
}

#[tracing::instrument(name = "Logging a user in", skip( pool, user, session), fields(user_email = %user.email))]
#[actix_web::post("/login")]
async fn login_user(
    pool: actix_web::web::Data<Pool<AsyncPgConnection>>,
    user: actix_web::web::Form<LoginUser>,
    session: actix_session::Session,
) -> actix_web::HttpResponse {
    let settings = common::settings::get_settings().expect("Failed to read settings.");

    let mut con = match pool.get().await {
        Ok(c) => c,
        Err(_) => return HttpResponse::InternalServerError().finish(),
    };

    match get_user_who_is_active_with_password(&mut con, &user.email).await {
        Ok(loggedin_user) => match tokio::task::spawn_blocking(move || {
            crate::utils::auth::password::verify_password(
                &loggedin_user.password.unwrap(),
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

                actix_web::HttpResponse::SeeOther()
                    .insert_header((actix_web::http::header::LOCATION, settings.frontend_url))
                    .finish()
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

#[tracing::instrument(name = "Getting a user from DB.", skip(con))]
pub(crate) async fn get_user_who_is_active_with_password(
    con: &mut AsyncPgConnection,
    user_email: &str,
) -> Result<common::models::User> {
    use common::schema::users::dsl::*;

    Ok(users
        .filter(email.eq(user_email))
        .filter(is_active.eq(true))
        .filter(password.is_not_null())
        .get_result::<User>(con)
        .await?)
}

#[cfg(test)]
mod tests {
    use diesel::{PgConnection, Connection};
    use diesel_async::{pooled_connection::AsyncDieselConnectionManager, AsyncConnection};
    use diesel_migrations::MigrationHarness;

    use crate::utils::auth::password::hash;

    use super::*;

    #[tokio::test]
    async fn two_users_same_password() {
        use common::schema::users::dsl::*;

        let settings = common::settings::get_settings().expect("Failed to read settings.");

        let mut connection = PgConnection::establish(&settings.database.url)
            .expect("Unable to connect to database to run migrations");
        connection
            .run_pending_migrations(crate::startup::MIGRATIONS)
            .expect("Unable to run database migrations");

        let manager =
            AsyncDieselConnectionManager::<AsyncPgConnection>::new(&settings.database.url);

        let pool = Pool::builder()
            .build(manager)
            .await
            .expect("Unable to build database pool");

        let mut con = pool
            .get()
            .await
            .expect("Unable to get connection from pool");

        con.begin_test_transaction()
            .await
            .expect("Couldn't start test transaction");

        diesel::insert_into(users)
            .values((
                email.eq("test100@example.com"),
                password.eq(hash("Password123!")),
                is_active.eq(true),
            ))
            .execute(&mut con)
            .await
            .expect("Unable to insert user");

        diesel::insert_into(users)
            .values((
                email.eq("test101@example.com"),
                password.eq(hash("Password123!")),
                is_active.eq(true),
            ))
            .execute(&mut con)
            .await
            .expect("Unable to insert user");

        diesel::insert_into(users)
            .values((email.eq("test102@example.com"), is_active.eq(true)))
            .execute(&mut con)
            .await
            .expect("Unable to insert user");

        let user1 = get_user_who_is_active_with_password(&mut con, "test100@example.com")
            .await
            .expect("Unable to get user 1");
        let user2 = get_user_who_is_active_with_password(&mut con, "test101@example.com")
            .await
            .expect("Unable to get user 2");
        let _user3 = get_user_who_is_active_with_password(&mut con, "test102@example.com")
            .await
            .expect_err("Shouldn't have returned a passwordless user");

        assert_eq!(user1.email, "test100@example.com");
        assert_eq!(user2.email, "test101@example.com");
    }
}
