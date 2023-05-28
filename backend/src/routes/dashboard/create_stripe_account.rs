use actix_web::{post, HttpResponse};
use stripe::{Account, Client, CreateAccount, StripeError};

use crate::extractors::AuthedUser;

#[post("/create_stripe_account")]
#[cfg_attr(
    not(coverage),
    tracing::instrument(name = "Setting up stripe connect", skip(stripe_client, user))
)]
pub async fn create(stripe_client: actix_web::web::Data<Client>, user: AuthedUser) -> HttpResponse {
    let account = match create_stripe_account(&stripe_client, &user.email).await {
        Ok(a) => a,
        Err(e) => {
            tracing::error!("Error creating stripe connect account: {}", e);
            return HttpResponse::InternalServerError().finish();
        }
    };

    HttpResponse::Ok().finish()
}

async fn create_stripe_account(
    stripe_client: &Client,
    email: &str,
) -> Result<Account, StripeError> {
    let account = CreateAccount {
        email: Some(email),
        ..Default::default()
    };

    Account::create(&stripe_client, account).await
}

#[cfg(test)]
mod tests {
    use crate::utils::stripe_test;
    use super::*;

    #[tokio::test]
    async fn test_create_account() -> anyhow::Result<()> {
        let stripe_client = stripe_test::stripe_client();

        let account = create_stripe_account(&stripe_client, "test@example.com").await?;
        assert_eq!(account.email, Some("test@example.com".into()));

        Ok(())
    }
}
