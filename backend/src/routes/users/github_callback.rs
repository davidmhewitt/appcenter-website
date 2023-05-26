use actix_session::Session;
use actix_web::http::header::{ACCEPT, USER_AGENT};
use actix_web::web;
use anyhow::Result;
use diesel::{ExpressionMethods, QueryDsl};
use diesel_async::pooled_connection::bb8::Pool;
use diesel_async::AsyncPgConnection;
use diesel_async::RunQueryDsl;
use oauth2::reqwest::async_http_client;
use oauth2::TokenResponse;
use oauth2::{
    basic::BasicClient, AuthUrl, AuthorizationCode, ClientId, ClientSecret, CsrfToken, TokenUrl,
};
use secrecy::{ExposeSecret, SecretString};
use serde::Deserialize;
use serde_json::Number;
use serde_variant::to_variant_name;

use crate::routes::users::register::insert_user_into_db;
use crate::types::ErrorTranslationKey;
use common::models::{NewGithubAuth, NewUser, User};

#[derive(Debug, Deserialize)]
pub struct CodeResponse {
    code: SecretString,
    state: String,
}

#[derive(Debug, Deserialize)]
pub struct GithubEmail {
    email: String,
    verified: bool,
    primary: bool,
    _visibility: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct GithubUser {
    id: Number,
}

#[tracing::instrument(name = "Github Callback", skip(session, response))]
#[actix_web::get("/github/callback")]
pub async fn github_callback(
    pool: actix_web::web::Data<Pool<AsyncPgConnection>>,
    session: Session,
    response: web::Query<CodeResponse>,
) -> actix_web::HttpResponse {
    let settings = common::settings::get_settings().expect("Failed to read settings.");

    let github_client_id = ClientId::new(settings.github.client_id);
    let github_client_secret =
        ClientSecret::new(settings.github.client_secret.expose_secret().to_owned());
    let auth_url = AuthUrl::new("https://github.com/login/oauth/authorize".to_string())
        .expect("Invalid authorization endpoint URL");
    let token_url = TokenUrl::new("https://github.com/login/oauth/access_token".to_string())
        .expect("Invalid token endpoint URL");

    // Set up the config for the Github OAuth2 process.
    let client = BasicClient::new(
        github_client_id,
        Some(github_client_secret),
        auth_url,
        Some(token_url),
    );

    let state = CsrfToken::new(response.state.to_owned());
    let code = AuthorizationCode::new(response.code.expose_secret().to_owned());

    let csrf_cookie = match session.get::<String>("_github_oauth_csrf") {
        Err(_) => {
            return error_redirect(
                &settings.frontend_url,
                ErrorTranslationKey::GenericRegistrationProblem,
            )
        }
        Ok(c) => c,
    };

    if let Some(c) = csrf_cookie {
        if &c != state.secret() {
            return error_redirect(
                &settings.frontend_url,
                ErrorTranslationKey::GenericRegistrationProblem,
            );
        }
    } else {
        return error_redirect(
            &settings.frontend_url,
            ErrorTranslationKey::GenericRegistrationProblem,
        );
    }

    let token_res = client
        .exchange_code(code)
        .request_async(async_http_client)
        .await;
    if let Ok(token) = token_res {
        let scopes = if let Some(scopes_vec) = token.scopes() {
            scopes_vec
                .iter()
                .flat_map(|comma_separated| comma_separated.split(','))
                .collect::<Vec<_>>()
        } else {
            Vec::new()
        };

        if !scopes.contains(&"user:email") {
            return error_redirect(
                &settings.frontend_url,
                ErrorTranslationKey::RegistrationNoEmailPermission,
            );
        }

        let client = reqwest::Client::default();
        let emails = client
            .get("https://api.github.com/user/emails")
            .header(ACCEPT, "application/vnd.github+json")
            .header(USER_AGENT, "elementary AppCenter Website")
            .header("X-GitHub-Api-Version", "2022-11-28")
            .bearer_auth(token.access_token().secret())
            .send()
            .await;

        let user = client
            .get("https://api.github.com/user")
            .header(ACCEPT, "application/vnd.github+json")
            .header(USER_AGENT, "elementary AppCenter Website")
            .header("X-GitHub-Api-Version", "2022-11-28")
            .bearer_auth(token.access_token().secret())
            .send()
            .await;

        let decoded_emails_response = match emails {
            Ok(x) => x,
            Err(_) => {
                return error_redirect(
                    &settings.frontend_url,
                    ErrorTranslationKey::GenericRegistrationProblem,
                )
            }
        };

        let email_array = match decoded_emails_response.json::<Vec<GithubEmail>>().await {
            Ok(x) => x,
            Err(_) => {
                return error_redirect(
                    &settings.frontend_url,
                    ErrorTranslationKey::GenericRegistrationProblem,
                )
            }
        };

        let primary = match email_array.iter().find(|&e| e.primary && e.verified) {
            Some(x) => &x.email,
            None => {
                return error_redirect(
                    &settings.frontend_url,
                    ErrorTranslationKey::GenericRegistrationProblem,
                )
            }
        };

        let decoded_user_response = match user {
            Ok(x) => x,
            Err(_) => {
                return error_redirect(
                    &settings.frontend_url,
                    ErrorTranslationKey::GenericRegistrationProblem,
                )
            }
        };

        let user_info = match decoded_user_response.json::<GithubUser>().await {
            Ok(x) => x,
            Err(_) => {
                return error_redirect(
                    &settings.frontend_url,
                    ErrorTranslationKey::GenericRegistrationProblem,
                )
            }
        };

        let mut con = match pool.get().await {
            Ok(c) => c,
            Err(_) => {
                return error_redirect(
                    &settings.frontend_url,
                    ErrorTranslationKey::GenericRegistrationProblem,
                )
            }
        };

        if let Ok(user) = get_user_who_is_active(&mut con, primary).await {
            session.remove("_github_oauth_csrf");
            session.renew();
            session
                .insert(crate::types::USER_ID_KEY, user.id)
                .expect("`user_id` cannot be inserted into session");
            session
                .insert(crate::types::USER_EMAIL_KEY, primary)
                .expect("`user_email` cannot be inserted into session");

            return actix_web::HttpResponse::SeeOther()
                .insert_header((actix_web::http::header::LOCATION, settings.frontend_url))
                .finish();
        }

        let user = NewUser {
            email: primary,
            password: None,
            is_active: true,
            is_admin: false,
        };

        let mut connection = match pool.get().await {
            Ok(transaction) => transaction,
            Err(_) => {
                return error_redirect(
                    &settings.frontend_url,
                    ErrorTranslationKey::GenericRegistrationProblem,
                )
            }
        };

        let user_id = match insert_user_into_db(
            &mut connection,
            user,
            NewGithubAuth {
                github_user_id: Some(user_info.id.to_string()),
                github_access_token: Some(token.access_token().secret().to_owned()),
                github_refresh_token: token.refresh_token().map(|t| t.secret().to_owned()),
            },
        )
        .await
        {
            Ok(u) => u,
            Err(_) => {
                return error_redirect(
                    &settings.frontend_url,
                    ErrorTranslationKey::GenericRegistrationProblem,
                )
            }
        };

        session.remove("_github_oauth_csrf");
        session.renew();
        session
            .insert(crate::types::USER_ID_KEY, user_id)
            .expect("`user_id` cannot be inserted into session");
        session
            .insert(crate::types::USER_EMAIL_KEY, primary)
            .expect("`user_email` cannot be inserted into session");
    }

    actix_web::HttpResponse::SeeOther()
        .insert_header((actix_web::http::header::LOCATION, settings.frontend_url))
        .finish()
}

fn error_redirect(
    frontend_url: &str,
    translation_key: ErrorTranslationKey,
) -> actix_web::HttpResponse {
    actix_web::HttpResponse::SeeOther()
        .insert_header((
            actix_web::http::header::LOCATION,
            format!(
                "{}/register?error={}",
                frontend_url,
                to_variant_name(&translation_key).unwrap()
            ),
        ))
        .finish()
}

#[tracing::instrument(name = "Getting a user from DB.", skip(con))]
pub(crate) async fn get_user_who_is_active(
    con: &mut AsyncPgConnection,
    user_email: &str,
) -> Result<common::models::User> {
    use common::schema::users::dsl::*;

    Ok(users
        .filter(email.eq(user_email))
        .filter(is_active.eq(true))
        .get_result::<User>(con)
        .await?)
}
