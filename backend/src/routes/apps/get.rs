use actix_web::{get, web::Data, HttpResponse};
use common::models::App;
use diesel::{result::Error::NotFound, ExpressionMethods, QueryDsl};
use diesel_async::{
    pooled_connection::bb8::{Pool, PooledConnection},
    AsyncPgConnection, RunQueryDsl,
};

use time::macros::datetime;

use crate::types::{ErrorResponse, ErrorTranslationKey};

#[cfg_attr(feature = "openapi", utoipa::path(
    path = "/apps/{id}",
    responses((
            status = 200,
            description = "",
            body = App,
            example = json!(App {
                id: "com.github.davidmhewitt.torrential".into(),
                repository: "https://github.com/davidmhewitt/torrential".into(),
                is_verified: true,
                last_submitted_version: Some("3.0.0".into()),
                first_seen: Some(datetime!(2020-01-01 0:00 UTC)),
                last_update: Some(datetime!(2023-03-27 17:22 UTC)),
                is_published: true,
                stripe_connect_id: Some("acct_1NEYZOPEvkLnkEch".into())
            })
        ),
    )
))]
#[cfg_attr(
    not(coverage),
    tracing::instrument(name = "Getting app info", skip(pool))
)]
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
