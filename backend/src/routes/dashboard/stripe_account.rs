use std::str::FromStr;

use actix_web::{get, HttpResponse};
use diesel::result::Error::NotFound;
use diesel_async::{pooled_connection::bb8::Pool, AsyncPgConnection};
use stripe::{Account, AccountId, Client};

use common::models::StripeAccount;

use crate::{
    extractors::AuthedUser,
    routes::dashboard::link_stripe_account::get_stripe_account_id_for_user,
    types::{ErrorResponse, ErrorTranslationKey},
};

#[cfg_attr(feature = "openapi", utoipa::path(
    path = "/dashboard/stripe_account",
    responses(
        (
            status = 200,
            description = "Stripe account details",
            body = StripeAccount,
            examples(
                ("example" = (value = json!(
                    StripeAccount {
                        account_id: "acct_1NCliJPBGjCwUDHc".to_string(),
                        charges_enabled: true,
                    }
                )))
            )
        ),
    )
))]
#[get("/stripe_account")]
#[cfg_attr(
    not(coverage),
    tracing::instrument(name = "Linking stripe connect account", skip(stripe_client, user))
)]
pub async fn get_stripe_account(
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

    let account_id = match AccountId::from_str(&account_id) {
        Ok(a) => a,
        Err(_) => {
            return HttpResponse::InternalServerError().json(ErrorResponse {
                error: "Error while fetching stripe account for user".into(),
                translation_key: ErrorTranslationKey::GenericServerProblem,
            });
        }
    };

    let account = match Account::retrieve(&stripe_client, &account_id, &[]).await {
        Ok(a) => a,
        Err(e) => {
            tracing::error!("Error fetching stripe account: {}", e);
            return HttpResponse::InternalServerError().json(ErrorResponse {
                error: "Error while fetching stripe account for user".into(),
                translation_key: ErrorTranslationKey::GenericServerProblem,
            });
        }
    };

    HttpResponse::Ok().json(StripeAccount {
        account_id: account.id.as_str().to_owned(),
        charges_enabled: account.charges_enabled.unwrap_or(false),
    })
}
