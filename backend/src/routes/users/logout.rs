use crate::extractors::AuthedUser;

#[cfg_attr(not(coverage), tracing::instrument(name = "Log out user", skip(session, _user)))]
#[actix_web::post("/logout")]
pub async fn log_out(
    session: actix_session::Session,
    _user: AuthedUser,
) -> actix_web::HttpResponse {
    session.purge();
    actix_web::HttpResponse::Ok().json(crate::types::SuccessResponse {
        message: "You have successfully logged out".to_string(),
    })
}
