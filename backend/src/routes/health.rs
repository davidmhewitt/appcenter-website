#[actix_web::get("/health-check")]
pub async fn health_check() -> actix_web::HttpResponse {
    tracing::event!(target: "backend", tracing::Level::DEBUG, "Accessing health-check endpoint.");
    actix_web::HttpResponse::Ok().json("Application is safe and healthy.")
}

#[cfg(test)]
mod tests {
    use super::*;

    use actix_web::{test, App};

    #[test]
    async fn test_health_check() {
        let mut app = test::init_service(App::new().service(health_check)).await;

        let req = test::TestRequest::get().uri("/health-check").to_request();
        let response = test::call_service(&mut app, req).await;

        assert!(response.status().is_success());
    }
}
