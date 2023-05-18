use diesel::ExpressionMethods;
use diesel_async::{
    pooled_connection::bb8::{Pool, PooledConnection},
    scoped_futures::ScopedFutureExt,
    AsyncConnection, AsyncPgConnection, RunQueryDsl,
};

use anyhow::Result;
use uuid::Uuid;

use crate::{
    models::{NewGithubAuth, NewUser},
    types::ErrorTranslationKey,
};

#[derive(serde::Deserialize, Debug)]
pub struct NewUserRequest {
    email: String,
    password: String,
}

#[tracing::instrument(name = "Adding a new user",
skip( pool, new_user, redis_pool),
fields(
    new_user_email = %new_user.email,
))]
#[actix_web::post("/register")]
pub async fn register_user(
    pool: actix_web::web::Data<Pool<AsyncPgConnection>>,
    new_user: actix_web::web::Json<NewUserRequest>,
    redis_pool: actix_web::web::Data<deadpool_redis::Pool>,
) -> actix_web::HttpResponse {
    let mut connection = match pool.get().await {
        Ok(c) => c,
        Err(e) => {
            tracing::event!(target: "backend", tracing::Level::ERROR, "Unable to get DB connection: {:#?}", e);
            return actix_web::HttpResponse::InternalServerError().json(
                crate::types::ErrorResponse {
                    error: "Something unexpected happened. Kindly try again.".to_string(),
                    translation_key: ErrorTranslationKey::GenericRegistrationProblem,
                },
            );
        }
    };

    let hashed_password = crate::utils::auth::password::hash(&new_user.0.password);

    let user = NewUser {
        email: &new_user.0.email,
        password: Some(hashed_password),
        is_active: false,
        is_admin: false,
    };

    let user_id = match insert_user_into_db(
        &mut connection,
        user,
        NewGithubAuth {
            github_user_id: None,
            github_access_token: None,
            github_refresh_token: None,
        },
    )
    .await
    {
        Ok(u) => u,
        Err(e) => {
            tracing::error!("Error inserting new user into DB: {}", e);
            return actix_web::HttpResponse::InternalServerError().json(
                crate::types::ErrorResponse {
                    error: "We cannot create your account at the moment".to_string(),
                    translation_key: ErrorTranslationKey::GenericRegistrationProblem,
                },
            );
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
        new_user.0.email.to_owned(),
        "verification_email.html",
        &mut redis_con,
    )
    .await
    .unwrap();

    tracing::event!(target: "backend", tracing::Level::INFO, "User created successfully.");
    actix_web::HttpResponse::Ok().json(crate::types::SuccessResponse {
        message: "Your account was created successfully. Check your email address to activate your account as we just sent you an activation link. Ensure you activate your account before the link expires".to_string(),
    })
}

pub(crate) async fn insert_user_into_db<'a>(
    connection: &mut PooledConnection<'a, AsyncPgConnection>,
    user: NewUser<'a>,
    github: NewGithubAuth,
) -> Result<Uuid> {
    use crate::schema::github_auth;
    use crate::schema::github_auth::{github_access_token, github_refresh_token, github_user_id};
    use crate::schema::user_profile;
    use crate::schema::users;
    use crate::schema::users::dsl::*;

    let user_id: Uuid = connection
        .transaction::<_, diesel::result::Error, _>(|mut transaction| {
            async move {
                let new_user_uuid = diesel::insert_into(users::table)
                    .values(user)
                    .returning(id)
                    .get_result::<Uuid>(transaction)
                    .await?;

                diesel::insert_into(user_profile::table)
                    .values(&user_profile::user_id.eq(new_user_uuid))
                    .execute(&mut transaction)
                    .await?;

                diesel::insert_into(github_auth::table)
                    .values((
                        github_auth::user_id.eq(new_user_uuid),
                        github_user_id.eq(github.github_user_id),
                        github_access_token.eq(github.github_access_token),
                        github_refresh_token.eq(github.github_refresh_token),
                    ))
                    .execute(&mut transaction)
                    .await?;

                Ok(new_user_uuid)
            }
            .scope_boxed()
        })
        .await?;

    Ok(user_id)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_user_insertion() {}
}
