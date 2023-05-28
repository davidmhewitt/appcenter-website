pub mod apps;
pub mod create_stripe_account;
pub mod link_stripe_account;
pub mod stripe_account;
pub mod submit_app_update;

pub fn dashboard_routes_config(cfg: &mut actix_web::web::ServiceConfig) {
    cfg.service(
        actix_web::web::scope("/api/dashboard")
            .service(apps::add_app)
            .service(apps::get_apps)
            .service(create_stripe_account::create)
            .service(link_stripe_account::link)
            .service(stripe_account::get_stripe_account)
            .service(submit_app_update::submit),
    );
}
