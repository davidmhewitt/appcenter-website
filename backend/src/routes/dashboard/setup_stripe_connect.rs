use actix_web::{post, HttpResponse};
use stripe::{Client, Account, CreateAccount};

use crate::extractors::AuthedUser;

#[post("/setup_stripe_connect")]
#[cfg_attr(
    not(coverage),
    tracing::instrument(name = "Setting up stripe connect", skip(stripe_client, user))
)]
pub async fn start(
    stripe_client: actix_web::web::Data<Client>,
    user: AuthedUser,
) -> HttpResponse {
    let account = CreateAccount {
        email: Some(&user.email),
        ..Default::default()
    };

    let account = Account::create(&stripe_client, account).await;

    HttpResponse::Ok().finish()
}

#[cfg(test)]
mod tests {
    use super::*;


}