use actix_web::{get, web::Data, HttpResponse};
use common::models::App;
use diesel::{query_dsl::methods::FilterDsl, ExpressionMethods};
use diesel_async::{pooled_connection::bb8::Pool, AsyncPgConnection, RunQueryDsl};

#[cfg(feature = "openapi")]

const EXAMPLE_JSON: &str = include_str!("examples/all_ids.json");

#[cfg_attr(feature = "openapi", utoipa::path(
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
))]
#[cfg_attr(not(coverage), tracing::instrument(name = "Getting all app ids", skip(pool)))]
#[get("/all_ids")]
pub async fn all_ids(pool: Data<Pool<AsyncPgConnection>>) -> actix_web::HttpResponse {
    use common::schema::apps::dsl::*;

    let mut con = match pool.get().await {
        Ok(c) => c,
        Err(e) => {
            tracing::error!("Unable to get DB connection for recent apps: {}", e);
            return HttpResponse::InternalServerError().finish();
        }
    };

    let all_apps: Vec<String> = match apps
        .filter(is_published.eq(true))
        .load::<App>(&mut con)
        .await
    {
        Ok(a) => a,
        Err(e) => {
            tracing::error!("Error getting all apps from db: {}", e);
            return HttpResponse::InternalServerError().finish();
        }
    }
    .iter()
    .map(|a| a.id.to_owned())
    .collect();

    HttpResponse::Ok().json(all_apps)
}
