use actix_web::{
    post,
    web::{Data, Json},
    HttpResponse,
};
use anyhow::Result;
use diesel::{ExpressionMethods, QueryDsl};
use diesel_async::{pooled_connection::bb8::Pool, AsyncPgConnection, RunQueryDsl};
use git_worker::GitWorker;
use serde::Serialize;
use uuid::Uuid;

use crate::{
    extractors::AuthedUser,
    types::{dashboard::AppUpdateSubmission, ErrorResponse, ErrorTranslationKey},
};

#[derive(Serialize)]
struct RepoAppFile {
    source: String,
    commit: String,
    version: String,
}

#[utoipa::path(
    path = "/dashboard/submit_app_update",
    request_body = AppUpdateSubmission,
)]
#[post("/submit_app_update")]
#[tracing::instrument(name = "Submitting app update", skip(user, pool, git_worker))]
pub async fn submit(
    user: AuthedUser,
    pool: Data<Pool<AsyncPgConnection>>,
    git_worker: actix_web::web::Data<GitWorker>,
    submission: Json<AppUpdateSubmission>,
) -> HttpResponse {
    let repo_url = match get_repo_url_from_db(&pool, &submission.app_id, &user.uuid).await {
        Ok(r) => r,
        Err(_) => {
            return HttpResponse::InternalServerError().json(ErrorResponse {
                error: "Unable to get Repository URL for app".into(),
                translation_key: ErrorTranslationKey::SubmitAppUpdateCannotGetUrl,
            })
        }
    };

    let branch_name = format!(
        "appcenter-website/{}-{}",
        submission.app_id, submission.version_tag
    );
    let commit_message = format!("{} version {}", submission.app_id, submission.version_tag);

    let w = git_worker.clone();
    let branch = branch_name.to_owned();
    let message = commit_message.to_owned();

    match tokio::task::spawn_blocking(move || {
        let git_worker = w;
        let branch_name = branch;
        let commit_message = message;

        let commit_id =
            match git_worker::get_remote_commit_id_from_tag(&repo_url, &submission.version_tag) {
                Ok(id) => id,
                Err(_) => return false,
            };

        let info = RepoAppFile {
            source: repo_url,
            commit: commit_id,
            version: submission.version_tag.to_owned(),
        };

        if let Err(e) = git_worker.checkout_branch("main") {
            tracing::error!("Error checking out main branch: {}", e);
            return false;
        }

        if let Err(e) = git_worker.update_repo() {
            tracing::error!("Error updating git repo: {}", e);
            return false;
        }

        if let Err(e) = git_worker.create_branch(&branch_name) {
            tracing::error!("Error creating branch: {}", e);
            return false;
        }

        if let Err(e) = std::fs::write(
            git_worker
                .repo_path
                .join("applications")
                .join(format!("{}.json", submission.app_id)),
            serde_json::ser::to_string_pretty(&info).unwrap(),
        ) {
            tracing::error!("Error writing app info to repo: {}", e);
            if let Err(e) = git_worker.checkout_branch("main") {
                tracing::error!("Error changing local branch: {}", e);
            }

            if let Err(e) = git_worker.delete_local_branch(&branch_name) {
                tracing::error!("Error deleting local branch: {}", e);
            }

            return false;
        }

        if let Err(e) = git_worker.add_and_commit(&["applications"], &commit_message) {
            tracing::error!("Error committing app: {}", e);
            if let Err(e) = git_worker.checkout_branch("main") {
                tracing::error!("Error changing local branch: {}", e);
            }

            if let Err(e) = git_worker.delete_local_branch(&branch_name) {
                tracing::error!("Error deleting local branch: {}", e);
            }

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

    if let Err(e) = git_worker
        .create_pull_request(
            commit_message,
            branch_name,
            "main".into(),
            "This pull request was automatically generated by the AppCenter website.".into(),
        )
        .await
    {
        tracing::error!("Error opening pull request: {}", e);
        return HttpResponse::InternalServerError().finish();
    }

    HttpResponse::Ok().finish()
}

async fn get_repo_url_from_db(
    pool: &Pool<AsyncPgConnection>,
    app_id: &str,
    uuid: &Uuid,
) -> Result<String> {
    use crate::schema::app_owners;
    use crate::schema::apps::dsl::*;

    let mut con = pool.get().await?;

    Ok(apps
        .inner_join(app_owners::table)
        .select(repository)
        .filter(app_owners::user_id.eq(uuid))
        .filter(id.eq(app_id))
        .filter(app_owners::verified_owner.eq(true))
        .get_result::<String>(&mut con)
        .await?)
}

// #[cfg(test)]
// mod tests {
//     use super::*;

//     #[sqlx::test]
//     async fn get_repo_from_db(pool: PgPool) -> sqlx::Result<()> {
//         let mut transaction = pool.begin().await?;

//         let user1_id: Uuid = sqlx::query(
//             "INSERT INTO users (email, password, is_active) VALUES ($1, NULL, TRUE) RETURNING id",
//         )
//         .bind("test1@example.com")
//         .map(|row: sqlx::postgres::PgRow| -> uuid::Uuid { row.get("id") })
//         .fetch_one(&mut transaction)
//         .await?;

//         sqlx::query(
//             "INSERT INTO apps (id, repository)
//             VALUES ('com.github.davidmhewitt.torrential', 'https://github.com/davidmhewitt/torrential.git')"
//         ).execute(&mut transaction)
//         .await?;

//         sqlx::query(
//             "INSERT INTO app_owners (user_id, app_id, verified_owner)
//             VALUES ($1, 'com.github.davidmhewitt.torrential', FALSE)",
//         )
//         .bind(user1_id)
//         .execute(&mut transaction)
//         .await?;

//         transaction.commit().await?;

//         let repo_url =
//             get_repo_url_from_db(&pool, "com.github.davidmhewitt.torrential", &user1_id).await;
//         assert!(repo_url.is_err());

//         let mut transaction = pool.begin().await?;

//         sqlx::query(
//             "UPDATE app_owners SET verified_owner = TRUE
//             WHERE app_id = 'com.github.davidmhewitt.torrential'",
//         )
//         .execute(&mut transaction)
//         .await?;

//         transaction.commit().await?;

//         let repo_url =
//             get_repo_url_from_db(&pool, "com.github.davidmhewitt.torrential", &user1_id).await;
//         assert!(repo_url.is_ok());
//         assert_eq!(
//             repo_url.unwrap(),
//             "https://github.com/davidmhewitt/torrential.git"
//         );

//         Ok(())
//     }
// }
