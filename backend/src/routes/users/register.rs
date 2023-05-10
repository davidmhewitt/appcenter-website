use secrecy::{ExposeSecret, SecretString};
use sqlstate::{
    postgres::{
        class::IntegrityConstraintViolation::UniqueViolation,
        SqlState::IntegrityConstraintViolation,
    },
    PostgresSqlState,
};
use sqlx::Row;

use crate::types::{CreateNewUser, ErrorTranslationKey};

#[derive(serde::Deserialize, Debug)]
pub struct NewUser {
    email: String,
    password: SecretString,
}

#[tracing::instrument(name = "Adding a new user",
skip( pool, new_user, redis_pool),
fields(
    new_user_email = %new_user.email,
))]
#[actix_web::post("/register")]
pub async fn register_user(
    pool: actix_web::web::Data<sqlx::postgres::PgPool>,
    new_user: actix_web::web::Json<NewUser>,
    redis_pool: actix_web::web::Data<deadpool_redis::Pool>,
) -> actix_web::HttpResponse {
    let mut transaction = match pool.begin().await {
        Ok(transaction) => transaction,
        Err(e) => {
            tracing::event!(target: "backend", tracing::Level::ERROR, "Unable to begin DB transaction: {:#?}", e);
            return actix_web::HttpResponse::InternalServerError().json(
                crate::types::ErrorResponse {
                    error: "Something unexpected happened. Kindly try again.".to_string(),
                    translation_key: ErrorTranslationKey::GenericRegistrationProblem,
                },
            );
        }
    };
    let hashed_password =
        crate::utils::auth::password::hash(new_user.0.password.expose_secret().as_bytes()).await;

    let create_new_user = CreateNewUser {
        password: Some(hashed_password),
        email: new_user.0.email,
        is_active: false,
        github_id: None,
        github_access_token: None,
        github_refresh_token: None,
    };

    let user_id = match insert_created_user_into_db(&mut transaction, &create_new_user).await {
        Ok(id) => id,
        Err(e) => {
            tracing::event!(target: "sqlx",tracing::Level::ERROR, "Failed to insert user into DB: {:#?}", e);
            let error_message = if e
                .as_database_error()
                .unwrap()
                .code()
                .unwrap()
                .parse::<PostgresSqlState>()
                .unwrap()
                == PostgresSqlState::Custom(IntegrityConstraintViolation(Some(UniqueViolation)))
            {
                crate::types::ErrorResponse {
                    error: "A user with that email address already exists".to_string(),
                    translation_key: ErrorTranslationKey::UserAlreadyExists,
                }
            } else {
                crate::types::ErrorResponse {
                    error: "Error inserting user into the database".to_string(),
                    translation_key: ErrorTranslationKey::GenericRegistrationProblem,
                }
            };
            return actix_web::HttpResponse::InternalServerError().json(error_message);
        }
    };

    // send confirmation email to the new user.
    let mut redis_con = redis_pool
        .get()
        .await
        .map_err(|e| {
            tracing::event!(target: "backend", tracing::Level::ERROR, "{}", e);
            actix_web::HttpResponse::InternalServerError().json(crate::types::ErrorResponse {
                error: "We cannot activate your account at the moment".to_string(),
                translation_key: ErrorTranslationKey::GenericRegistrationProblem,
            })
        })
        .expect("Redis connection cannot be gotten.");

    crate::utils::emails::send_multipart_email(
        "RustAuth - Let's get you verified".to_string(),
        user_id,
        Some(String::from("accounts@elementary.io")),
        create_new_user.email,
        "verification_email.html",
        &mut redis_con,
    )
    .await
    .unwrap();

    if transaction.commit().await.is_err() {
        return actix_web::HttpResponse::InternalServerError().finish();
    }

    tracing::event!(target: "backend", tracing::Level::INFO, "User created successfully.");
    actix_web::HttpResponse::Ok().json(crate::types::SuccessResponse {
        message: "Your account was created successfully. Check your email address to activate your account as we just sent you an activation link. Ensure you activate your account before the link expires".to_string(),
    })
}

#[tracing::instrument(name = "Inserting new user into DB.", skip(transaction, new_user),fields(
    new_user_email = %new_user.email,
))]
pub async fn insert_created_user_into_db(
    transaction: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    new_user: &CreateNewUser,
) -> Result<uuid::Uuid, sqlx::Error> {
    let user_id = match sqlx::query(
        "INSERT INTO users (email, password, is_active, github_id, github_access_token, github_refresh_token) VALUES ($1, $2, $3, $4, $5, $6) RETURNING id",
    )
    .bind(&new_user.email)
    .bind(
        new_user
            .password
            .as_ref()
            .map(|p| p.expose_secret()),
    )
    .bind(new_user.is_active)
    .bind(new_user.github_id)
    .bind(new_user.github_access_token.as_ref().map(|t| t.expose_secret()))
    .bind(new_user.github_refresh_token.as_ref().map(|t| t.expose_secret()))
    .map(|row: sqlx::postgres::PgRow| -> uuid::Uuid { row.get("id") })
    .fetch_one(&mut *transaction)
    .await
    {
        Ok(id) => id,
        Err(e) => {
            tracing::event!(target: "sqlx",tracing::Level::ERROR, "Failed to insert user into DB: {:#?}", e);
            return Err(e);
        }
    };

    match sqlx::query(
        "INSERT INTO user_profile (user_id)
                VALUES ($1)
            ON CONFLICT (user_id)
            DO NOTHING
            RETURNING user_id",
    )
    .bind(user_id)
    .map(|row: sqlx::postgres::PgRow| -> uuid::Uuid { row.get("user_id") })
    .fetch_one(&mut *transaction)
    .await
    {
        Ok(id) => {
            tracing::event!(target: "sqlx",tracing::Level::INFO, "User profile created successfully {}.", id);
            Ok(id)
        }
        Err(e) => {
            tracing::event!(target: "sqlx",tracing::Level::ERROR, "Failed to insert user's profile into DB: {:#?}", e);
            Err(e)
        }
    }
}
