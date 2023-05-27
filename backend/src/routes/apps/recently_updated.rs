use actix_web::{get, web::Data, HttpResponse};
use diesel::{ExpressionMethods, QueryDsl};
use diesel_async::{pooled_connection::bb8::Pool, AsyncPgConnection, RunQueryDsl};

use common::models::{App, ComponentSummary};

#[cfg(feature = "openapi")]
const EXAMPLE_JSON: &str = include_str!("examples/recently_updated.json");

#[cfg_attr(feature = "openapi", utoipa::path(
    path = "/apps/recently_updated",
    responses(
        (
            status = 200,
            description = "List of recently updated applications",
            body = Vec<ComponentSummary>,
            examples(
                ("example" = (value = json!(serde_json::from_str::<Vec<ComponentSummary>>(EXAMPLE_JSON).unwrap())))
            )
        ),
    )
))]
#[cfg_attr(not(coverage), tracing::instrument(name = "Getting recently updated apps", skip(pool, redis_pool)))]
#[get("/recently_updated")]
pub async fn recently_updated(
    pool: Data<Pool<AsyncPgConnection>>,
    redis_pool: actix_web::web::Data<deadpool_redis::Pool>,
) -> actix_web::HttpResponse {
    use common::schema::apps::dsl::*;

    let mut redis_con = redis_pool
        .get()
        .await
        .map_err(|e| {
            tracing::event!(target: "backend", tracing::Level::ERROR, "{}", e);

            actix_web::HttpResponse::InternalServerError().finish()
        })
        .expect("Redis connection cannot be gotten.");

    let mut con = match pool.get().await {
        Ok(c) => c,
        Err(e) => {
            tracing::error!("Unable to get DB connection for recent apps: {}", e);
            return HttpResponse::InternalServerError().finish();
        }
    };

    let recent_apps = match apps
        .filter(is_published.eq(true))
        .order(last_update.desc())
        .limit(20)
        .load::<App>(&mut con)
        .await
    {
        Ok(r) => match deadpool_redis::redis::cmd("hmget")
            .arg(common::APP_SUMMARIES_REDIS_KEY)
            .arg(r.iter().map(|a| a.id.to_owned()).collect::<Vec<_>>())
            .query_async::<_, Vec<Option<String>>>(&mut redis_con)
            .await
        {
            Ok(a) => a,
            Err(e) => {
                tracing::error!("Error getting recently updated apps from redis: {}", e);
                return actix_web::HttpResponse::InternalServerError().finish();
            }
        }
        .into_iter()
        .flatten()
        .filter_map(|s| match serde_json::de::from_str::<ComponentSummary>(&s) {
            Ok(c) => Some(c),
            Err(e) => {
                tracing::warn!("Error deserializing component summary from redis: {}", e);
                None
            }
        })
        .collect::<Vec<_>>(),
        Err(e) => {
            tracing::error!("Unable to get DB connection for recent apps: {}", e);
            return HttpResponse::InternalServerError().finish();
        }
    };

    HttpResponse::Ok().json(recent_apps)
}
