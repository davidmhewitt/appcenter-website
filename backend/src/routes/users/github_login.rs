use actix_session::Session;
use actix_web::get;
use oauth2::{basic::BasicClient, AuthUrl, ClientId, ClientSecret, CsrfToken, Scope, TokenUrl};
use secrecy::ExposeSecret;

#[cfg_attr(feature = "openapi", utoipa::path(
    path = "/users/github/login",
    responses(
        (status = 303),
    )
))]
#[cfg_attr(
    not(coverage),
    tracing::instrument(name = "Github Login", skip(session))
)]
#[get("/github/login")]
pub async fn github_login(session: Session) -> actix_web::HttpResponse {
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

    let (authorize_url, csrf_state) = client
        .authorize_url(CsrfToken::new_random)
        .add_scope(Scope::new("user:email".into()))
        .url();

    session
        .insert("_github_oauth_csrf", csrf_state.secret())
        .expect("Failed to serialize csrf state");

    actix_web::HttpResponse::SeeOther()
        .insert_header((actix_web::http::header::LOCATION, authorize_url.as_str()))
        .finish()
}
