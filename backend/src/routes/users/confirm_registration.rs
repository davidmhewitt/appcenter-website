use actix_web::get;
use anyhow::Result;
use diesel::ExpressionMethods;
use diesel_async::{pooled_connection::bb8::Pool, AsyncPgConnection, RunQueryDsl};

use crate::types::ErrorTranslationKey;

#[cfg(feature = "openapi")]
use utoipa::IntoParams;

#[derive(serde::Deserialize)]
#[cfg_attr(feature = "openapi", derive(IntoParams))]

pub struct Parameters {
    // The PASETO confirmation token emailed to the user
    token: String,
}

#[cfg_attr(feature = "openapi", utoipa::path(
    path = "/users/register/confirm",
    params(Parameters),
    responses(
        (status = 303),
    )
))]
#[cfg_attr(
    not(coverage),
    tracing::instrument(name = "Activating a new user", skip(pool, parameters, redis_pool))
)]
#[get("/register/confirm")]
pub async fn confirm(
    parameters: actix_web::web::Query<Parameters>,
    pool: actix_web::web::Data<Pool<AsyncPgConnection>>,
    redis_pool: actix_web::web::Data<deadpool_redis::Pool>,
) -> actix_web::HttpResponse {
    let settings = common::settings::get_settings().expect("Failed to read settings.");

    let mut redis_con = redis_pool
        .get()
        .await
        .map_err(|e| {
            tracing::event!(target: "backend", tracing::Level::ERROR, "{}", e);

            actix_web::HttpResponse::SeeOther()
                .insert_header((
                    actix_web::http::header::LOCATION,
                    format!("{}/auth/error", settings.frontend_url),
                ))
                .json(crate::types::ErrorResponse {
                    error: "We cannot activate your account at the moment".to_string(),
                    translation_key: ErrorTranslationKey::GenericConfirmationProblem,
                })
        })
        .expect("Redis connection cannot be gotten.");

    let confirmation_token = match crate::utils::auth::tokens::verify_confirmation_token_pasetor(
        parameters.token.clone(),
        &mut redis_con,
        false,
    )
    .await
    {
        Ok(token) => token,
        Err(e) => {
            tracing::event!(target: "backend",tracing::Level::ERROR, "{:#?}", e);

            return actix_web::HttpResponse::SeeOther().insert_header((
                    actix_web::http::header::LOCATION,
                    format!("{}/auth/regenerate-token", settings.frontend_url),
                )).json(crate::types::ErrorResponse {
                    error: "It appears that your confirmation token has expired or previously used. Kindly generate a new token".to_string(),
                    translation_key: ErrorTranslationKey::ConfirmationTokenUsed
                });
        }
    };
    match activate_new_user(&pool, confirmation_token.user_id).await {
        Ok(_) => {
            tracing::event!(target: "backend", tracing::Level::INFO, "New user was activated successfully.");

            actix_web::HttpResponse::SeeOther()
                .insert_header((
                    actix_web::http::header::LOCATION,
                    format!("{}/auth/confirmed", settings.frontend_url),
                ))
                .json(crate::types::SuccessResponse {
                    message: "Your account has been activated successfully!!! You can now log in"
                        .to_string(),
                })
        }
        Err(e) => {
            tracing::event!(target: "backend", tracing::Level::ERROR, "Cannot activate account : {}", e);

            actix_web::HttpResponse::SeeOther()
                .insert_header((
                    actix_web::http::header::LOCATION,
                    format!("{}/auth/error?reason={e}", settings.frontend_url),
                ))
                .json(crate::types::ErrorResponse {
                    error: "We cannot activate your account at the moment".to_string(),
                    translation_key: ErrorTranslationKey::GenericConfirmationProblem,
                })
        }
    }
}

#[cfg_attr(not(coverage), tracing::instrument(name = "Mark a user active", skip(pool), fields(
    new_user_user_id = %user_id
)))]
pub async fn activate_new_user(pool: &Pool<AsyncPgConnection>, user_id: uuid::Uuid) -> Result<()> {
    use common::schema::users;
    use common::schema::users::dsl::*;

    let mut con = pool.get().await?;

    diesel::update(users::table)
        .filter(id.eq(user_id))
        .set(is_active.eq(true))
        .execute(&mut con)
        .await?;

    Ok(())
}
