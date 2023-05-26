use actix_web::{get, HttpResponse};

use crate::extractors::AuthedUser;

#[tracing::instrument(name = "Test Auth", skip(_user))]
#[utoipa::path(
    path = "/users/test_auth",
    responses(
        (status = 200, description = "User is authenticated"),
        (status = 403, description = "User is not authenticated")
    )
)]
#[get("/test_auth")]
async fn test_auth(_user: AuthedUser) -> actix_web::HttpResponse {
    HttpResponse::Ok().finish()
}
