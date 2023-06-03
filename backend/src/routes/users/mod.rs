pub mod confirm_registration;
pub mod github_callback;
pub mod github_login;
pub mod login;
pub mod logout;
pub mod register;
pub(crate) mod test_auth;

pub fn auth_routes_config(cfg: &mut actix_web::web::ServiceConfig) {
    cfg.service(
        actix_web::web::scope("/api/users")
            .service(confirm_registration::confirm)
            .service(github_callback::github_callback)
            .service(github_login::github_login)
            .service(login::login_user)
            .service(logout::log_out)
            .service(register::register_user)
            .service(test_auth::test_auth),
    );
}
