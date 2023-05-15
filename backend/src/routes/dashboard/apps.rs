use actix_web::web::{Data, Json};
use actix_web::{get, post, HttpResponse};
use git_worker::GitWorker;
use sqlx::postgres::PgRow;
use sqlx::{PgPool, Row};
use url::Url;
use uuid::Uuid;

use crate::extractors::AuthedUser;
use crate::types::dashboard::{App, CreateApp};
use crate::types::{ErrorResponse, ErrorTranslationKey};

#[utoipa::path(
    path = "/dashboard/apps",
    responses(
        (
            status = 200,
            description = "A list of apps owned by the current user",
            body = Vec<App>,
            examples(
                ("example" = (value = json!(vec!
                    [
                        App {
                            id: "com.github.davidmhewitt.torrential".into(),
                            repository: "https://github.com/davidmhewitt/torrential.git".into(),
                            is_verified: true,
                            version: Some("3.0.1".into()),
                        },
                        App {
                            id: "io.elementary.photos".into(),
                            repository: "https://github.com/elementary/photos.git".into(),
                            is_verified: false,
                            version: None,
                        }
                    ]
                )))
            )
        )
    )
)]
#[get("/apps")]
#[tracing::instrument(name = "Fetching apps for dashboard", skip(user, pool))]
pub async fn get_apps(user: AuthedUser, pool: Data<sqlx::postgres::PgPool>) -> HttpResponse {
    let apps = get_apps_from_db(&pool, &user.uuid).await;

    match apps {
        Ok(a) => HttpResponse::Ok().json(a),
        Err(_) => HttpResponse::InternalServerError().finish(),
    }
}

async fn get_apps_from_db(
    pool: &sqlx::postgres::PgPool,
    uuid: &Uuid,
) -> Result<Vec<App>, sqlx::Error> {
    let mut con = pool.acquire().await?;

    return sqlx::query(
        "SELECT id, repository, verified_owner, last_submitted_version
            FROM apps app
            INNER JOIN app_owners owner
            ON app.id = owner.app_id
            WHERE owner.user_id = $1",
    )
    .bind(uuid)
    .map(|r: PgRow| App {
        id: r.get("id"),
        repository: r.get("repository"),
        is_verified: r.get("verified_owner"),
        version: r.get("last_submitted_version"),
    })
    .fetch_all(&mut con)
    .await;
}

#[utoipa::path(
    path = "/dashboard/apps",
    request_body = CreateApp,
)]
#[post("/apps")]
#[tracing::instrument(name = "Adding dashboard app", skip(user, pool, git_worker))]
pub async fn add_app(
    user: AuthedUser,
    pool: Data<sqlx::postgres::PgPool>,
    git_worker: Data<GitWorker>,
    app: Json<CreateApp>,
) -> HttpResponse {
    let github_user_id = get_github_user_id(&pool, &user.uuid).await;
    println!("{:?}", github_user_id);

    let url = match Url::parse(&app.repository) {
        Ok(u) => u,
        Err(_) => {
            return HttpResponse::BadRequest().json(ErrorResponse {
                error: "Invalid URL passed in `git_repo_url`".into(),
                translation_key: ErrorTranslationKey::AppRegisterInvalidRepositoryUrl,
            });
        }
    };

    let mut verified = false;
    if app.app_id.starts_with("com.github.") || app.app_id.starts_with("io.github.") {
        let (owner, path_repo_name) = match validate_github_url_and_rdnn(&url, &app.app_id) {
            GithubRdnnValidationResult::Valid((owner, repo)) => (owner, repo),
            GithubRdnnValidationResult::Invalid((error, translation_key)) => {
                return HttpResponse::BadRequest().json(ErrorResponse {
                    error,
                    translation_key,
                })
            }
        };

        if let Some(github_user_id) = github_user_id {
            let owner_id = git_worker
                .get_github_repo_owner_id(&owner, &path_repo_name)
                .await
                .ok();

            if let Some(owner_id) = owner_id {
                if let git_worker::GithubOwner::User(repo_owner_id) = owner_id {
                    verified = repo_owner_id == github_user_id;
                } else if let git_worker::GithubOwner::Org(org_id) = owner_id {
                    verified = is_user_admin_member_of_github_org(&pool, &user.uuid, org_id).await;
                }
            }
        }
    }

    match add_app_to_db(&pool, &user.uuid, &app.app_id, &app.repository, verified).await {
        Ok(_) => {}
        Err(e) => match e {
            AddAppError::SqlError(e) => {
                tracing::error!("Couldn't add app to database: {}", e);
                return HttpResponse::InternalServerError().json(ErrorResponse {
                    error: "Database error while trying to add new app".into(),
                    translation_key: ErrorTranslationKey::GenericAppRegisterProblem,
                });
            }
            AddAppError::UserError((message, translation_key)) => {
                return HttpResponse::BadRequest().json(ErrorResponse {
                    error: message,
                    translation_key,
                })
            }
        },
    }

    HttpResponse::Ok().finish()
}

enum AddAppError {
    SqlError(sqlx::Error),
    UserError((String, ErrorTranslationKey)),
}

async fn add_app_to_db(
    pool: &PgPool,
    owner: &Uuid,
    app_id: &str,
    repository_url: &str,
    verified: bool,
) -> Result<(), AddAppError> {
    let mut transaction = pool.begin().await.map_err(AddAppError::SqlError)?;

    let db_repo = sqlx::query("SELECT repository FROM apps WHERE id = $1")
        .bind(app_id)
        .map(|r| -> String { r.get("repository") })
        .fetch_optional(&mut transaction)
        .await
        .map_err(AddAppError::SqlError)?;

    if db_repo.is_some() && db_repo.unwrap() != repository_url {
        return Err(AddAppError::UserError((
            "App already exists with a different repository set".into(),
            ErrorTranslationKey::GenericAppRegisterProblem,
        )));
    }

    sqlx::query(
        "INSERT INTO apps (id, repository, is_verified) VALUES ($1, $2, $3)
        ON CONFLICT DO NOTHING",
    )
    .bind(app_id)
    .bind(repository_url)
    .bind(verified)
    .execute(&mut transaction)
    .await
    .map_err(AddAppError::SqlError)?;

    sqlx::query(
        "INSERT INTO app_owners (user_id, app_id, verified_owner) VALUES ($1, $2, $3)
        ON CONFLICT DO NOTHING",
    )
    .bind(owner)
    .bind(app_id)
    .bind(verified)
    .execute(&mut transaction)
    .await
    .map_err(AddAppError::SqlError)?;

    if let Err(e) = transaction.commit().await {
        tracing::error!("Couldn't commit to database: {}", e);
        return Err(AddAppError::SqlError(e));
    }

    Ok(())
}

async fn is_user_admin_member_of_github_org(
    pool: &Data<sqlx::postgres::PgPool>,
    uuid: &Uuid,
    _org_id: String,
) -> bool {
    let _tokens = match get_github_user_tokens(&pool, &uuid).await {
        Some(t) => t,
        None => return false,
    };

    // TODO: Awaiting https://github.com/XAMPPRocky/octocrab/pull/357

    false
}

async fn get_github_user_id(pool: &Data<sqlx::postgres::PgPool>, user: &Uuid) -> Option<String> {
    let mut con = pool.acquire().await.ok()?;

    let id = sqlx::query("SELECT github_user_id FROM github_auth WHERE user_id = $1")
        .bind(user)
        .map(|r: PgRow| r.get("github_user_id"))
        .fetch_one(&mut con)
        .await;

    id.ok()
}

async fn get_github_user_tokens(
    pool: &Data<sqlx::postgres::PgPool>,
    user: &Uuid,
) -> Option<(Option<String>, Option<String>)> {
    let mut con = pool.acquire().await.ok()?;

    let tokens = sqlx::query(
        "SELECT github_access_token, github_refresh_token FROM github_auth WHERE user_id = $1",
    )
    .bind(user)
    .map(|r: PgRow| (r.get("github_access_token"), r.get("github_refresh_token")))
    .fetch_one(&mut con)
    .await;

    tokens.ok()
}

#[derive(Debug, PartialEq)]
enum GithubRdnnValidationResult {
    Valid((String, String)),
    Invalid((String, ErrorTranslationKey)),
}

fn validate_github_url_and_rdnn(url: &Url, rdnn: &str) -> GithubRdnnValidationResult {
    let rdnn_parts = rdnn.split('.').collect::<Vec<&str>>();
    if rdnn_parts.len() != 4 {
        return GithubRdnnValidationResult::Invalid((
            "GitHub RDNNs must have exactly 4 sections/components".into(),
            ErrorTranslationKey::AppRegisterNonMatchingGithubRDNN,
        ));
    }

    let path_segments = match url.path_segments() {
        Some(s) => s,
        None => {
            return GithubRdnnValidationResult::Invalid((
                "Invalid GitHub repository URL passed in `git_repo_url`".into(),
                ErrorTranslationKey::AppRegisterInvalidRepositoryUrl,
            ));
        }
    }
    .collect::<Vec<&str>>();

    let path_repo_name = path_segments.get(1).unwrap();
    let path_repo_name = if path_repo_name.ends_with(".git") {
        path_repo_name.strip_suffix(".git").unwrap()
    } else {
        path_repo_name
    };

    if url.host_str() != Some("github.com") {
        return GithubRdnnValidationResult::Invalid((
            "GitHub RDNN repositories must be served from GitHub".into(),
            ErrorTranslationKey::AppRegisterNonMatchingGithubRDNN,
        ));
    }

    if path_segments.len() != 2 {
        return GithubRdnnValidationResult::Invalid((
            "Invalid GitHub repository URL passed in `git_repo_url`".into(),
            ErrorTranslationKey::AppRegisterInvalidRepositoryUrl,
        ));
    }

    if *rdnn_parts.get(2).unwrap() != *path_segments.get(0).unwrap() {
        return GithubRdnnValidationResult::Invalid((
            "RDNN owner doesn't match GitHub URL owner".into(),
            ErrorTranslationKey::AppRegisterNonMatchingGithubRDNN,
        ));
    }

    if *rdnn_parts.get(3).unwrap() != path_repo_name {
        return GithubRdnnValidationResult::Invalid((
            "RDNN repo doesn't match GitHub URL repo".into(),
            ErrorTranslationKey::AppRegisterNonMatchingGithubRDNN,
        ));
    }

    GithubRdnnValidationResult::Valid((
        (*path_segments.get(0).unwrap()).to_owned(),
        path_repo_name.to_owned(),
    ))
}

#[cfg(test)]
mod tests {
    use sqlx::PgPool;

    use super::*;

    #[test]
    fn github_rdnn_validation() {
        let url = Url::parse("https://github.com/davidmhewitt/torrential.git")
            .expect("Couldn't parse URL");
        assert_eq!(
            validate_github_url_and_rdnn(&url, "com.github.davidmhewitt.torrential"),
            GithubRdnnValidationResult::Valid(("davidmhewitt".into(), "torrential".into()))
        );
    }

    #[sqlx::test]
    async fn app_database_tests(pool: PgPool) -> sqlx::Result<()> {
        let apps = get_apps_from_db(&pool, &Uuid::new_v4()).await?;
        assert!(apps.is_empty());

        let mut transaction = pool.begin().await?;

        let user1_id: Uuid = sqlx::query(
            "INSERT INTO users (email, password, is_active) VALUES ($1, NULL, TRUE) RETURNING id",
        )
        .bind("test1@example.com")
        .map(|row: sqlx::postgres::PgRow| -> uuid::Uuid { row.get("id") })
        .fetch_one(&mut transaction)
        .await?;

        let user2_id: Uuid = sqlx::query(
            "INSERT INTO users (email, password, is_active) VALUES ($1, NULL, TRUE) RETURNING id",
        )
        .bind("test2@example.com")
        .map(|row: sqlx::postgres::PgRow| -> uuid::Uuid { row.get("id") })
        .fetch_one(&mut transaction)
        .await?;

        transaction.commit().await?;

        match add_app_to_db(
            &pool,
            &user1_id,
            "com.github.davidmhewitt.torrential",
            "https://github.com/davidmhewitt/torrential",
            true,
        )
        .await
        {
            Err(AddAppError::SqlError(e)) => return Err(e),
            Err(AddAppError::UserError(e)) => panic!("{}", e.0),
            Ok(_) => {}
        }

        let apps = get_apps_from_db(&pool, &user1_id).await?;
        assert_eq!(apps.len(), 1);

        let apps = get_apps_from_db(&pool, &user2_id).await?;
        assert!(apps.is_empty());

        Ok(())
    }
}
