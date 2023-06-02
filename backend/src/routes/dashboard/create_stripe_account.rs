use actix_web::{post, HttpResponse};
use diesel::ExpressionMethods;
use diesel_async::{
    pooled_connection::bb8::{Pool, PooledConnection},
    AsyncPgConnection, RunQueryDsl,
};
use stripe::{Account, AccountId, AccountType, Client, CreateAccount, StripeError};

use crate::{
    extractors::AuthedUser,
    types::{ErrorResponse, ErrorTranslationKey},
};

#[post("/create_stripe_account")]
#[cfg_attr(
    not(coverage),
    tracing::instrument(name = "Setting up stripe connect", skip(stripe_client, user))
)]
pub async fn create(
    stripe_client: actix_web::web::Data<Client>,
    user: AuthedUser,
    pool: actix_web::web::Data<Pool<AsyncPgConnection>>,
) -> HttpResponse {
    let mut con = match pool.get().await {
        Ok(c) => c,
        Err(e) => {
            tracing::error!("Unable to get db connection: {}", e);
            return HttpResponse::InternalServerError().finish();
        }
    };

    let account = match create_stripe_account(&stripe_client, &user.email).await {
        Ok(a) => a,
        Err(e) => {
            tracing::error!("Error creating stripe connect account: {}", e);
            return HttpResponse::InternalServerError().finish();
        }
    };

    if let Err(e) = add_stripe_account_to_db(&mut con, user.uuid, &account.id).await {
        tracing::error!("Error adding stripe account to database: {}", e);
        return HttpResponse::InternalServerError().json(ErrorResponse {
            error: "Error adding stripe account to database".into(),
            translation_key: ErrorTranslationKey::GenericServerProblem,
        });
    }

    HttpResponse::Ok().finish()
}

async fn create_stripe_account(
    stripe_client: &Client,
    email: &str,
) -> Result<Account, StripeError> {
    let account = CreateAccount {
        email: Some(email),
        type_: Some(AccountType::Standard),
        ..Default::default()
    };

    Account::create(stripe_client, account).await
}

#[cfg_attr(
    not(coverage),
    tracing::instrument(name = "Add stripe account to user", skip(con))
)]
pub async fn add_stripe_account_to_db(
    con: &mut PooledConnection<'_, AsyncPgConnection>,
    user: uuid::Uuid,
    account_id: &AccountId,
) -> anyhow::Result<()> {
    use common::schema::stripe_accounts::dsl::*;

    diesel::insert_into(stripe_accounts)
        .values((user_id.eq(user), stripe_account_id.eq(account_id.as_str())))
        .execute(con)
        .await?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use std::str::FromStr;

    use diesel_async::AsyncConnection;

    use super::*;
    use crate::utils::{db_test::create_user, stripe_test};

    #[tokio::test]
    async fn test_create_account() -> anyhow::Result<()> {
        let stripe_client = stripe_test::stripe_client();

        let account = create_stripe_account(&stripe_client, "test@example.com").await?;
        assert_eq!(account.email, Some("test@example.com".into()));

        Ok(())
    }

    #[tokio::test]
    async fn test_put_account_in_db() -> anyhow::Result<()> {
        let db_pool = crate::utils::db_test::db_pool().await;

        let mut con = db_pool.get().await?;

        con.begin_test_transaction().await?;

        let user = create_user(&mut con, true).await?;

        add_stripe_account_to_db(&mut con, user, &AccountId::from_str("acct_1234")?).await?;

        Ok(())
    }
}
