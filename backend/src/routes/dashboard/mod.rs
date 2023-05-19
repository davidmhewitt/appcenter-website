pub mod apps;
pub mod submit_app_update;

pub fn dashboard_routes_config(cfg: &mut actix_web::web::ServiceConfig) {
    cfg.service(
        actix_web::web::scope("/api/dashboard")
            .service(apps::add_app)
            .service(apps::get_apps)
            .service(submit_app_update::submit),
    );
}
