use actix_web::HttpResponse;
use appstream_worker::ComponentSummary;

#[tracing::instrument(name = "Getting recently updated apps", skip(redis_pool))]
#[actix_web::get("/recently_updated")]
pub async fn recently_updated(
    redis_pool: actix_web::web::Data<deadpool_redis::Pool>,
) -> actix_web::HttpResponse {
    let mut redis_con = redis_pool
        .get()
        .await
        .map_err(|e| {
            tracing::event!(target: "backend", tracing::Level::ERROR, "{}", e);

            actix_web::HttpResponse::InternalServerError().finish()
        })
        .expect("Redis connection cannot be gotten.");

    let apps: Vec<ComponentSummary> = match deadpool_redis::redis::Cmd::lrange(
        appstream_worker::RECENTLY_UPDATED_REDIS_KEY,
        0,
        19,
    )
    .query_async::<_, Vec<String>>(&mut redis_con)
    .await
    {
        Ok(a) => a,
        Err(e) => {
            tracing::error!("Error getting recently updated apps from redis: {}", e);
            return actix_web::HttpResponse::InternalServerError().finish();
        }
    }
    .into_iter()
    .filter_map(
        |s| match serde_json::de::from_str::<appstream_worker::ComponentSummary>(&s) {
            Ok(c) => Some(c),
            Err(e) => {
                tracing::warn!("Error deserializing component summary from redis: {}", e);
                None
            }
        },
    ).collect();

    HttpResponse::Ok().json(apps)
}