use actix_session::Session;
use actix_web::{web::Json, HttpResponse};
use git_worker::GitWorker;
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize)]
pub struct SubmitAppInfo {
    #[serde(skip_serializing)]
    app_id: String,
    source: String,
    commit: String,
    version: String,
}

#[tracing::instrument(name = "Submitting app", skip(session, pool, git_worker))]
#[actix_web::post("/submit_app")]
pub async fn submit_app(
    session: Session,
    pool: actix_web::web::Data<sqlx::postgres::PgPool>,
    info: Json<SubmitAppInfo>,
    git_worker: actix_web::web::Data<GitWorker>,
) -> actix_web::HttpResponse {
    if !crate::utils::auth::check_auth(session, pool).await {
        return HttpResponse::Unauthorized().finish();
    }

    match tokio::task::spawn_blocking(move || {
        if let Err(e) = git_worker.checkout_branch("main") {
            tracing::error!("Error checking out main branch: {}", e);
            return false;
        }

        if let Err(e) = git_worker.update_repo() {
            tracing::error!("Error updating git repo: {}", e);
            return false;
        }

        let branch_name = format!("appcenter-website/{}-{}", info.app_id, info.version);

        if let Err(e) = git_worker.create_branch(&branch_name) {
            tracing::error!("Error creating branch: {}", e);
            return false;
        }

        if let Err(e) = std::fs::write(
            git_worker
                .repo_path
                .join("applications")
                .join(format!("{}.json", info.app_id)),
            serde_json::ser::to_string_pretty(&info.0).unwrap(),
        ) {
            tracing::error!("Error writing app info to repo: {}", e);
            return false;
        }

        if let Err(e) = git_worker.add_and_commit(
            &["applications"],
            &format!("{} version {}", info.app_id, info.version),
        ) {
            tracing::error!("Error committing app: {}", e);
            return false;
        }

        if let Err(e) = git_worker.push(&branch_name) {
            tracing::error!("Error pushing app: {}", e);
            return false;
        }

        true
    })
    .await
    {
        Ok(success) => {
            if !success {
                return HttpResponse::InternalServerError().finish();
            }
        }
        Err(_) => {
            return HttpResponse::InternalServerError().finish();
        }
    }

    HttpResponse::Ok().finish()
}
