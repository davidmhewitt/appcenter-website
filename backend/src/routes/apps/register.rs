use actix_session::Session;
use actix_web::{web::Json, HttpResponse};
use serde::{Deserialize, Serialize};

use crate::types::ErrorTranslationKey;

#[derive(Debug, Deserialize)]
pub struct RegisterInfo {
    app_id: String,
    git_repo_url: String,
}

#[derive(Serialize)]
struct ErrorResponse {
    error: String,
    translation_key: ErrorTranslationKey,
}

#[tracing::instrument(name = "Registering app", skip(session, pool))]
#[actix_web::post("/register")]
pub async fn register(
    session: Session,
    pool: actix_web::web::Data<sqlx::postgres::PgPool>,
    info: Json<RegisterInfo>,
) -> actix_web::HttpResponse {
    let user_uuid = match crate::utils::auth::check_auth(session, &pool).await {
        Some(u) => u,
        None => return HttpResponse::Unauthorized().finish(),
    };

    let mut transaction = match pool.begin().await {
        Ok(transaction) => transaction,
        Err(e) => {
            tracing::error!("Couldn't start database transaction: {}", e);
            return HttpResponse::InternalServerError().json(ErrorResponse {
                error: "Couldn't start database transaction".into(),
                translation_key: ErrorTranslationKey::GenericAppRegisterProblem,
            });
        }
    };

    match sqlx::query("INSERT INTO apps (id, user_id, repository) VALUES ($1, $2, $3)")
        .bind(&info.app_id)
        .bind(user_uuid)
        .bind(&info.git_repo_url)
        .execute(&mut transaction)
        .await
    {
        Ok(_) => {}
        Err(e) => {
            tracing::error!("Couldn't insert app into database: {}", e);
        }
    }

    if let Err(e) = transaction.commit().await {
        tracing::error!("Couldn't commit to database: {}", e);
        return HttpResponse::InternalServerError().json(ErrorResponse {
            error: "Couldn't commit database transaction".into(),
            translation_key: ErrorTranslationKey::GenericAppRegisterProblem,
        });
    }

    HttpResponse::Ok().finish()
}
