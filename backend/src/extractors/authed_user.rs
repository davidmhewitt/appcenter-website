use std::{future::Future, pin::Pin};

use actix_session::SessionExt;
use actix_web::{
    error::{Error, ErrorInternalServerError, ErrorUnauthorized},
    web::Data,
    FromRequest,
};
use diesel_async::{pooled_connection::bb8::Pool, AsyncPgConnection};
use serde_json::json;

pub struct AuthedUser {
    pub uuid: uuid::Uuid,
}

impl FromRequest for AuthedUser {
    type Error = Error;

    type Future = Pin<Box<dyn Future<Output = Result<Self, Self::Error>>>>;

    #[inline]
    fn from_request(req: &actix_web::HttpRequest, _: &mut actix_web::dev::Payload) -> Self::Future {
        let req = req.clone();
        Box::pin(async move {
            let session = req.get_session();
            if let Some(pool) = req.app_data::<Data<Pool<AsyncPgConnection>>>() {
                if let Some(user) = crate::utils::auth::check_auth(session, pool).await {
                    Ok(AuthedUser { uuid: user })
                } else {
                    Err(ErrorUnauthorized(
                        json!({"error": "This request requires authorization"}),
                    ))
                }
            } else {
                Err(ErrorInternalServerError(
                    json!({"error": "Error verifying authorization"}),
                ))
            }
        })
    }
}
