use actix_web::{get, HttpResponse};

const EXAMPLE_JSON: &str = include_str!("examples/all_ids.json");

#[utoipa::path(
    path = "/apps/all_ids",
    responses(
        (
            status = 200,
            description = "List of all applications ids",
            body = Vec<String>,
            examples(
                ("example" = (value = json!(serde_json::from_str::<Vec<String>>(EXAMPLE_JSON).unwrap())))
            )
        ),
    )
)]
#[tracing::instrument(name = "Getting all app ids", skip(redis_pool))]
#[get("/all_ids")]
pub async fn all_ids(
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

    let apps: Vec<String> =
        match deadpool_redis::redis::Cmd::lrange(appstream_worker::ALL_APP_IDS_REDIS_KEY, 0, -1)
            .query_async::<_, Vec<String>>(&mut redis_con)
            .await
        {
            Ok(a) => a,
            Err(e) => {
                tracing::error!("Error getting recently updated apps from redis: {}", e);
                return actix_web::HttpResponse::InternalServerError().finish();
            }
        };

    HttpResponse::Ok().json(apps)
}
