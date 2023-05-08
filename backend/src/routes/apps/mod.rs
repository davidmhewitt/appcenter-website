mod recently_added;
mod recently_updated;
mod submit_app;

pub fn apps_routes_config(cfg: &mut actix_web::web::ServiceConfig) {
    cfg.service(
        actix_web::web::scope("/api/apps")
            .service(recently_added::recently_added)
            .service(recently_updated::recently_updated)
            .service(submit_app::submit_app),
    );
}
