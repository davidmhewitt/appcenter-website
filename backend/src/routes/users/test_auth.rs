use actix_session::Session;
use actix_web::HttpResponse;

use crate::utils;

#[tracing::instrument(name = "Test Auth", skip(session, pool))]
#[actix_web::get("/test_auth")]
pub async fn test_auth(
    session: Session,
    pool: actix_web::web::Data<sqlx::postgres::PgPool>,
) -> actix_web::HttpResponse {
    match utils::auth::check_auth(session, &pool).await {
        Some(_) => HttpResponse::Ok().finish(),
        None => HttpResponse::Unauthorized().finish(),
    }
}
