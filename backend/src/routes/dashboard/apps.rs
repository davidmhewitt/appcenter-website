use actix_web::web::{Data, Json};
use actix_web::{get, post, HttpResponse};
use anyhow::{anyhow, Result};
use diesel::{ExpressionMethods, OptionalExtension, QueryDsl};
use diesel_async::pooled_connection::bb8::{Pool, PooledConnection};
use diesel_async::{AsyncPgConnection, RunQueryDsl};
use url::Url;
use uuid::Uuid;

use crate::extractors::AuthedUser;
use crate::types::dashboard::CreateApp;
use crate::types::{ErrorResponse, ErrorTranslationKey};
use common::models::App;

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
                            last_submitted_version: Some("3.0.1".into()),
                            first_seen: None,
                            last_update: None,
                            is_published: true,
                        },
                        App {
                            id: "io.elementary.photos".into(),
                            repository: "https://github.com/elementary/photos.git".into(),
                            is_verified: false,
                            last_submitted_version: None,
                            first_seen: None,
                            last_update: None,
                            is_published: true,
                        }
                    ]
                )))
            )
        )
    )
)]
#[get("/apps")]
#[tracing::instrument(name = "Fetching apps for dashboard", skip(user, pool))]
pub async fn get_apps(user: AuthedUser, pool: Data<Pool<AsyncPgConnection>>) -> HttpResponse {
    let mut con = match pool.get().await {
        Ok(c) => c,
        Err(e) => {
            tracing::error!("Error getting databsae connection: {}", e);
            return HttpResponse::InternalServerError().finish();
        }
    };

    let apps = get_apps_from_db(&mut con, &user.uuid).await;

    match apps {
        Ok(a) => HttpResponse::Ok().json(a),
        Err(_) => HttpResponse::InternalServerError().finish(),
    }
}

pub async fn get_apps_from_db(
    con: &mut PooledConnection<'_, AsyncPgConnection>,
    uuid: &Uuid,
) -> Result<Vec<App>> {
    use common::schema::app_owners::dsl::*;
    use common::schema::apps::dsl::*;

    Ok(apps
        .inner_join(app_owners)
        .select((
            id,
            repository,
            verified_owner,
            last_submitted_version,
            first_seen,
            last_update,
            is_published,
        ))
        .filter(user_id.eq(uuid))
        .get_results::<App>(con)
        .await?)
}

#[utoipa::path(
    path = "/dashboard/apps",
    request_body = CreateApp,
)]
#[post("/apps")]
#[tracing::instrument(name = "Adding dashboard app", skip(user, pool))]
pub async fn add_app(
    user: AuthedUser,
    pool: Data<Pool<AsyncPgConnection>>,
    app: Json<CreateApp>,
) -> HttpResponse {
    let github_user_id = get_github_user_id(&pool, &user.uuid).await;

    let url = match Url::parse(&app.repository) {
        Ok(u) => u,
        Err(_) => {
            return HttpResponse::BadRequest().json(ErrorResponse {
                error: "Invalid URL passed in `git_repo_url`".into(),
                translation_key: ErrorTranslationKey::AddAppInvalidRepositoryUrl,
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

        if let Ok(Some(github_user_id)) = github_user_id {
            let owner_id = github_utils::get_github_repo_owner_id(&owner, &path_repo_name)
                .await
                .ok();

            if let Some(owner_id) = owner_id {
                if let github_utils::GithubOwner::User(repo_owner_id) = owner_id {
                    verified = repo_owner_id == github_user_id;
                } else if let github_utils::GithubOwner::Org(org_id) = owner_id {
                    verified = is_user_admin_member_of_github_org(&pool, &user.uuid, org_id).await;
                }
            }
        }
    }

    let mut con = match pool.get().await {
        Ok(c) => c,
        Err(e) => {
            tracing::error!("Error getting database connection: {}", e);
            return HttpResponse::InternalServerError().finish();
        }
    };

    match add_app_to_db(&mut con, &user.uuid, &app.app_id, &app.repository, verified).await {
        Ok(_) => {}
        Err(e) => {
            tracing::error!("Error adding app to db: {}", e);
            return HttpResponse::InternalServerError().finish();
        }
    }

    HttpResponse::Ok().finish()
}

pub async fn add_app_to_db(
    con: &mut PooledConnection<'_, AsyncPgConnection>,
    owner: &Uuid,
    new_app_id: &str,
    repository_url: &str,
    verified: bool,
) -> Result<()> {
    use common::schema::app_owners::dsl::*;
    use common::schema::apps::dsl::*;

    if let Some(existing_repository) = apps
        .filter(id.eq(new_app_id))
        .select(repository)
        .get_result::<String>(con)
        .await
        .optional()?
    {
        if existing_repository != repository_url {
            return Err(anyhow!("App already exists with a different repository"));
        }
    }

    diesel::insert_into(apps)
        .values((
            id.eq(new_app_id),
            repository.eq(repository_url),
            is_verified.eq(verified),
        ))
        .on_conflict_do_nothing()
        .execute(con)
        .await?;

    diesel::insert_into(app_owners)
        .values((
            user_id.eq(owner),
            app_id.eq(new_app_id),
            verified_owner.eq(verified),
        ))
        .on_conflict_do_nothing()
        .execute(con)
        .await?;

    Ok(())
}

async fn is_user_admin_member_of_github_org(
    pool: &Data<Pool<AsyncPgConnection>>,
    uuid: &Uuid,
    _org_id: String,
) -> bool {
    let _tokens = match get_github_user_tokens(pool, uuid).await {
        Ok(t) => t,
        Err(_) => return false,
    };

    // TODO: Awaiting https://github.com/XAMPPRocky/octocrab/pull/357

    false
}

async fn get_github_user_id(
    pool: &Data<Pool<AsyncPgConnection>>,
    user: &Uuid,
) -> Result<Option<String>> {
    use common::schema::github_auth::dsl::*;

    let mut con = pool.get().await?;

    Ok(github_auth
        .select(github_user_id)
        .filter(user_id.eq(user))
        .get_result(&mut con)
        .await?)
}

async fn get_github_user_tokens(
    pool: &Data<Pool<AsyncPgConnection>>,
    user: &Uuid,
) -> Result<(Option<String>, Option<String>)> {
    use common::schema::github_auth::dsl::*;

    let mut con = pool.get().await?;

    Ok(github_auth
        .select((github_access_token, github_refresh_token))
        .filter(user_id.eq(user))
        .get_result(&mut con)
        .await?)
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
            ErrorTranslationKey::AddAppNonMatchingGithubRDNN,
        ));
    }

    let path_segments = match url.path_segments() {
        Some(s) => s,
        None => {
            return GithubRdnnValidationResult::Invalid((
                "Invalid GitHub repository URL passed in `git_repo_url`".into(),
                ErrorTranslationKey::AddAppInvalidRepositoryUrl,
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
            ErrorTranslationKey::AddAppNonMatchingGithubRDNN,
        ));
    }

    if path_segments.len() != 2 {
        return GithubRdnnValidationResult::Invalid((
            "Invalid GitHub repository URL passed in `git_repo_url`".into(),
            ErrorTranslationKey::AddAppInvalidRepositoryUrl,
        ));
    }

    if *rdnn_parts.get(2).unwrap() != *path_segments.get(0).unwrap() {
        return GithubRdnnValidationResult::Invalid((
            "RDNN owner doesn't match GitHub URL owner".into(),
            ErrorTranslationKey::AddAppNonMatchingGithubRDNN,
        ));
    }

    if *rdnn_parts.get(3).unwrap() != path_repo_name {
        return GithubRdnnValidationResult::Invalid((
            "RDNN repo doesn't match GitHub URL repo".into(),
            ErrorTranslationKey::AddAppNonMatchingGithubRDNN,
        ));
    }

    GithubRdnnValidationResult::Valid((
        (*path_segments.get(0).unwrap()).to_owned(),
        path_repo_name.to_owned(),
    ))
}

#[cfg(test)]
mod tests {
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
}
