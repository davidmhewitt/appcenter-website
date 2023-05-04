use actix_session::Session;
use actix_web::http::header::{ACCEPT, USER_AGENT};
use actix_web::web;
use oauth2::reqwest::async_http_client;
use oauth2::TokenResponse;
use oauth2::{
    basic::BasicClient, AuthUrl, AuthorizationCode, ClientId, ClientSecret, CsrfToken, TokenUrl,
};
use secrecy::{ExposeSecret, SecretString};
use serde::Deserialize;
use serde_variant::to_variant_name;
use sqlx::Row;

use crate::routes::users::register::insert_created_user_into_db;
use crate::types::{CreateNewUser, ErrorTranslationKey};

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

#[tracing::instrument(name = "Github Callback", skip(session, response))]
#[actix_web::get("/github/callback")]
pub async fn github_callback(
    pool: actix_web::web::Data<sqlx::postgres::PgPool>,
    session: Session,
    response: web::Query<CodeResponse>,
) -> actix_web::HttpResponse {
    let settings = crate::settings::get_settings().expect("Failed to read settings.");

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

    let csrf_cookie = session.get::<String>("_github_oauth_csrf");
    if csrf_cookie.is_err() || !csrf_cookie.unwrap().unwrap().eq(state.secret()) {
        return error_redirect(ErrorTranslationKey::GenericRegistrationProblem);
    }

    let token_res = client
        .exchange_code(code)
        .request_async(async_http_client)
        .await;
    if let Ok(token) = token_res {
        let scopes = if let Some(scopes_vec) = token.scopes() {
            scopes_vec
                .iter()
                .map(|comma_separated| comma_separated.split(','))
                .flatten()
                .collect::<Vec<_>>()
        } else {
            Vec::new()
        };

        if !scopes.contains(&"user:email") {
            return error_redirect(ErrorTranslationKey::RegistrationNoEmailPermission);
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

        let decoded_emails_response = match emails {
            Ok(x) => x,
            Err(_) => return error_redirect(ErrorTranslationKey::GenericRegistrationProblem),
        };

        let email_array = match decoded_emails_response.json::<Vec<GithubEmail>>().await {
            Ok(x) => x,
            Err(_) => return error_redirect(ErrorTranslationKey::GenericRegistrationProblem),
        };

        let primary = match email_array.iter().find(|&e| e.primary && e.verified) {
            Some(x) => &x.email,
            None => return error_redirect(ErrorTranslationKey::GenericRegistrationProblem),
        };

        if let Ok(user) = get_user_who_is_active(&pool, primary).await {
            session.remove("_github_oauth_csrf");
            session.renew();
            session
                .insert(crate::types::USER_ID_KEY, user.id)
                .expect("`user_id` cannot be inserted into session");
            session
                .insert(crate::types::USER_EMAIL_KEY, primary)
                .expect("`user_email` cannot be inserted into session");

                return actix_web::HttpResponse::SeeOther()
                    .insert_header((actix_web::http::header::LOCATION, "/"))
                    .finish()
        }

        let user = CreateNewUser {
            email: primary.to_owned(),
            password: None,
            is_active: true,
        };

        let mut transaction = match pool.begin().await {
            Ok(transaction) => transaction,
            Err(_) => return error_redirect(ErrorTranslationKey::GenericRegistrationProblem),
        };

        let user_id = match insert_created_user_into_db(&mut transaction, &user).await {
            Ok(u) => u,
            Err(_) => return error_redirect(ErrorTranslationKey::GenericRegistrationProblem),
        };

        if transaction.commit().await.is_err() {
            return error_redirect(ErrorTranslationKey::GenericRegistrationProblem);
        }

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
        .insert_header((actix_web::http::header::LOCATION, "/"))
        .finish()
}

fn error_redirect(translation_key: ErrorTranslationKey) -> actix_web::HttpResponse {
    actix_web::HttpResponse::SeeOther()
        .insert_header((
            actix_web::http::header::LOCATION,
            format!(
                "/register?error={}",
                to_variant_name(&translation_key).unwrap()
            ),
        ))
        .finish()
}

#[tracing::instrument(name = "Getting a user from DB.", skip(pool, email),fields(user_email = %email))]
async fn get_user_who_is_active(
    pool: &sqlx::postgres::PgPool,
    email: &String,
) -> Result<crate::types::User, sqlx::Error> {
    match sqlx::query("SELECT id, email, password, is_admin, date_joined FROM users WHERE email = $1 AND is_active = TRUE")
        .bind(email)
        .map(|row: sqlx::postgres::PgRow| crate::types::User {
            id: row.get("id"),
            email: row.get("email"),
            password_hash: row.get::<Option<String>, &str>("password").map(|p| SecretString::from(p)),
            is_active: true,
            is_admin: row.get("is_admin"),
            date_joined: row.get("date_joined"),
        })
        .fetch_one(pool)
        .await
    {
        Ok(user) => Ok(user),
        Err(e) => {
            tracing::event!(target: "sqlx",tracing::Level::ERROR, "User not found in DB: {:#?}", e);
            Err(e)
        }
    }
}
