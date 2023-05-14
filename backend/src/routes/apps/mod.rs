pub(crate) mod all_ids;
pub(crate) mod recently_added;
pub(crate) mod recently_updated;

pub fn apps_routes_config(cfg: &mut actix_web::web::ServiceConfig) {
    cfg.service(
        actix_web::web::scope("/api/apps")
            .service(all_ids::all_ids)
            .service(recently_added::recently_added)
            .service(recently_updated::recently_updated),
    );
}
