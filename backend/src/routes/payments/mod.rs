pub mod start;

pub fn payments_routes_config(cfg: &mut actix_web::web::ServiceConfig) {
    cfg.service(actix_web::web::scope("/api/payments").service(start::start));
}
