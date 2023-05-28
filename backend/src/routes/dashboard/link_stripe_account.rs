use std::str::FromStr;

use actix_web::{get, HttpResponse};
use diesel::{result::Error::NotFound, ExpressionMethods, QueryDsl};
use diesel_async::{
    pooled_connection::bb8::{Pool, PooledConnection},
    AsyncPgConnection, RunQueryDsl,
};
use stripe::{AccountId, AccountLink, Client, CreateAccountLink};

use crate::{
    extractors::AuthedUser,
    types::{ErrorResponse, ErrorTranslationKey},
};

#[get("/link_stripe_account")]
#[cfg_attr(
    not(coverage),
    tracing::instrument(name = "Linking stripe connect account", skip(stripe_client, user))
)]
pub async fn link(
    stripe_client: actix_web::web::Data<Client>,
    user: AuthedUser,
    pool: actix_web::web::Data<Pool<AsyncPgConnection>>,
) -> HttpResponse {
    let settings = common::settings::get_settings().expect("Failed to read settings.");

    let mut con = match pool.get().await {
        Ok(c) => c,
        Err(e) => {
            tracing::error!("Unable to get db connection: {}", e);
            return HttpResponse::InternalServerError().finish();
        }
    };

    let account_id = match get_stripe_account_id_for_user(&mut con, user.uuid).await {
        Ok(a) => a,
        Err(e) => {
            if e == NotFound {
                return HttpResponse::BadRequest().json(ErrorResponse {
                    error: "No stripe account created for user".into(),
                    translation_key: ErrorTranslationKey::StripeLinkNoAccount,
                });
            } else {
                return HttpResponse::InternalServerError().json(ErrorResponse {
                    error: "Error while fetching stripe account for user".into(),
                    translation_key: ErrorTranslationKey::GenericServerProblem,
                });
            }
        }
    };

    let base_url = url::Url::parse(&settings.frontend_url).expect("invalid frontend url");

    let link_result = match link_stripe_account(
        &stripe_client,
        &account_id,
        &base_url
            .join("api/dashboard/link_stripe_account")
            .unwrap()
            .to_string(),
        &base_url.join("dashboard").unwrap().to_string(),
    )
    .await
    {
        Ok(a) => a,
        Err(e) => {
            tracing::error!("Error linking stripe account: {}", e);
            return HttpResponse::InternalServerError().json(ErrorResponse {
                error: "Unable to initiate stripe account linking".into(),
                translation_key: ErrorTranslationKey::GenericServerProblem,
            });
        }
    };

    HttpResponse::SeeOther()
        .insert_header(("Location", link_result.url))
        .finish()
}

async fn link_stripe_account(
    stripe_client: &Client,
    account_id: &str,
    refresh_url: &str,
    return_url: &str,
) -> anyhow::Result<AccountLink> {
    let params = CreateAccountLink {
        account: AccountId::from_str(account_id)?,
        collect: Default::default(),
        expand: Default::default(),
        return_url: Some(return_url),
        refresh_url: Some(refresh_url),
        type_: stripe::AccountLinkType::AccountOnboarding,
    };

    Ok(AccountLink::create(stripe_client, params).await?)
}

#[cfg_attr(
    not(coverage),
    tracing::instrument(name = "Getting stripe account for user", skip(con))
)]
pub async fn get_stripe_account_id_for_user(
    con: &mut PooledConnection<'_, AsyncPgConnection>,
    user: uuid::Uuid,
) -> Result<String, diesel::result::Error> {
    use common::schema::stripe_accounts::dsl::*;

    let account_id = stripe_accounts
        .filter(user_id.eq(user))
        .select(stripe_account_id)
        .get_result::<String>(con)
        .await;

    account_id
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::utils::stripe_test;

    #[tokio::test]
    async fn test_link_account() -> anyhow::Result<()> {
        let stripe_client = stripe_test::stripe_client();

        link_stripe_account(
            &stripe_client,
            "acct_1234",
            "http://localhost:3100/api/dashboard/link_stripe_account",
            "http://localhost:3000/dashboard",
        )
        .await?;

        Ok(())
    }
}
