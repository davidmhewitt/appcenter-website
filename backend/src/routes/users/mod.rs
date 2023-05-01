mod confirm_registration;
mod github_callback;
mod github_login;
mod login;
mod logout;
mod register;
mod test_auth;

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
