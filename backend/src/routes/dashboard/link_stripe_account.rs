use std::str::FromStr;

use actix_web::{post, HttpResponse};
use stripe::{AccountId, AccountLink, Client, CreateAccountLink};

use crate::extractors::AuthedUser;

#[post("/link_stripe_account")]
#[cfg_attr(
    not(coverage),
    tracing::instrument(name = "Linking stripe connect account", skip(stripe_client, user))
)]
pub async fn link(stripe_client: actix_web::web::Data<Client>, user: AuthedUser) -> HttpResponse {
    HttpResponse::Ok().finish()
}

async fn link_stripe_account(
    stripe_client: &Client,
    account_id: &str,
    refresh_url: Option<&str>,
    return_url: Option<&str>,
) -> anyhow::Result<AccountLink> {
    let params = CreateAccountLink {
        account: AccountId::from_str(account_id)?,
        collect: Default::default(),
        expand: Default::default(),
        return_url,
        refresh_url,
        type_: stripe::AccountLinkType::AccountOnboarding,
    };

    Ok(AccountLink::create(stripe_client, params).await?)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::utils::stripe_test;

    #[tokio::test]
    async fn test_link_account() -> anyhow::Result<()> {
        let stripe_client = stripe_test::stripe_client();

        link_stripe_account(&stripe_client, "acct_1234", None, None).await?;

        Ok(())
    }
}
