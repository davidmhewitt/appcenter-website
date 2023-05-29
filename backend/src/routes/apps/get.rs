use actix_web::{get, web::Data, HttpResponse};
use common::models::App;
use diesel::{result::Error::NotFound, ExpressionMethods, QueryDsl};
use diesel_async::{
    pooled_connection::bb8::{Pool, PooledConnection},
    AsyncPgConnection, RunQueryDsl,
};

use crate::types::{ErrorResponse, ErrorTranslationKey};

#[get("/{id}")]
pub async fn get(
    path: actix_web::web::Path<(String,)>,
    pool: Data<Pool<AsyncPgConnection>>,
) -> HttpResponse {
    let id = path.into_inner().0;

    let mut con = match pool.get().await {
        Ok(c) => c,
        Err(e) => {
            tracing::error!("Unable to get database connection: {}", e);
            return HttpResponse::InternalServerError().finish();
        }
    };

    let app = match get_app_by_id(&mut con, &id).await {
        Ok(a) => a,
        Err(NotFound) => {
            return HttpResponse::BadRequest().json(ErrorResponse {
                error: "Specified app ID was not found".into(),
                translation_key: ErrorTranslationKey::AppNotFound,
            });
        }
        Err(e) => {
            tracing::error!("Error fetching app from database: {}", e);
            return HttpResponse::InternalServerError().json(ErrorResponse {
                error: "Error fetching app from database".into(),
                translation_key: ErrorTranslationKey::GenericServerProblem,
            });
        }
    };

    HttpResponse::Ok().json(app)
}

pub async fn get_app_by_id(
    con: &mut PooledConnection<'_, AsyncPgConnection>,
    app_id_to_find: &str,
) -> Result<App, diesel::result::Error> {
    use common::schema::apps::dsl::*;

    apps.filter(id.eq(app_id_to_find)).get_result(con).await
}
