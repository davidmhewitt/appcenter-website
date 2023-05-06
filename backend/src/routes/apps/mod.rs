mod recently_updated;

pub fn apps_routes_config(cfg: &mut actix_web::web::ServiceConfig) {
    cfg.service(
        actix_web::web::scope("/api/apps")
            .service(recently_updated::recently_updated)
    );
}