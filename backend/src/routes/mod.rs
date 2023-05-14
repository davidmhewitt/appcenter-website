use utoipa::OpenApi;

mod apps;
mod dashboard;
mod health;
mod users;

pub use apps::apps_routes_config;
pub use dashboard::dashboard_routes_config;
pub use health::health_check;
pub use users::auth_routes_config;

#[derive(OpenApi)]
#[openapi(
    paths(
        users::test_auth::test_auth,
        apps::all_ids::all_ids,
        apps::recently_added::recently_added,
        apps::recently_updated::recently_updated,
        dashboard::apps::add_app,
        dashboard::apps::get_apps,
        dashboard::submit_app_update::submit,
    ),
    components(schemas(
        appstream_worker::ComponentSummary,
        appstream_worker::TranslatableString,
        appstream_worker::Icon,
        crate::types::dashboard::App,
        crate::types::dashboard::CreateApp,
        crate::types::dashboard::AppUpdateSubmission,
    ))
)]
pub struct ApiDoc;
