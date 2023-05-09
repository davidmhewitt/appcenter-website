use actix_session::Session;
use actix_web::{web::Json, HttpResponse};
use serde::{Deserialize, Serialize};
use url::Url;

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

    let url = match Url::parse(&info.git_repo_url) {
        Ok(u) => u,
        Err(e) => {
            return HttpResponse::BadRequest().json(ErrorResponse {
                error: "Invalid URL passed in `git_repo_url`".into(),
                translation_key: ErrorTranslationKey::AppRegisterInvalidRepositoryUrl,
            });
        }
    };

    let github_id: Option<i64> = if info.app_id.starts_with("com.github.") || info.app_id.starts_with("io.github.") {
        if url.host_str() != Some("github.com") {
            return HttpResponse::BadRequest().json(ErrorResponse {
                error: "GitHub RDNN repositories must be served from GitHub".into(),
                translation_key: ErrorTranslationKey::AppRegisterNonMatchingGithubRDNN,
            });
        }

        let path_segments = match url.path_segments() {
            Some(s) => s,
            None => {
                return HttpResponse::BadRequest().json(ErrorResponse {
                    error: "Invalid GitHub repository URL passed in `git_repo_url`".into(),
                    translation_key: ErrorTranslationKey::AppRegisterInvalidRepositoryUrl,
                });
            }
        }.collect::<Vec<&str>>();

        if path_segments.len() != 2 {
            return HttpResponse::BadRequest().json(ErrorResponse {
                error: "Invalid GitHub repository URL passed in `git_repo_url`".into(),
                translation_key: ErrorTranslationKey::AppRegisterInvalidRepositoryUrl,
            });
        }

        let rdnn_parts = info.app_id.split('.').collect::<Vec<&str>>();
        if rdnn_parts.len() != 4 {
            return HttpResponse::BadRequest().json(ErrorResponse {
                error: "GitHub RDNNs must have exactly 4 sections/components".into(),
                translation_key: ErrorTranslationKey::AppRegisterNonMatchingGithubRDNN,
            });
        }

        if rdnn_parts.get(2).unwrap() != path_segments.get(0).unwrap() {
            return HttpResponse::BadRequest().json(ErrorResponse {
                error: "RDNN owner doesn't match GitHub URL owner".into(),
                translation_key: ErrorTranslationKey::AppRegisterNonMatchingGithubRDNN,
            });
        }

        None
    } else {
        None
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
