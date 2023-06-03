mod apps;
pub mod dashboard;
mod health;
pub mod payments;
pub mod users;

pub use apps::apps_routes_config;
pub use dashboard::dashboard_routes_config;
pub use health::health_check;
pub use payments::payments_routes_config;
pub use users::auth_routes_config;

#[cfg_attr(feature = "openapi", derive(utoipa::OpenApi))]
#[cfg_attr(
    feature = "openapi",
    openapi(
        paths(
            users::test_auth::test_auth,
            apps::all_ids::all_ids,
            apps::get::get,
            apps::recently_added::recently_added,
            apps::recently_updated::recently_updated,
            dashboard::apps::add_app,
            dashboard::apps::get_apps,
            dashboard::stripe_account::get_stripe_account,
            dashboard::submit_app_update::submit,
            payments::start::start,
        ),
        components(schemas(
            common::models::App,
            common::models::ComponentSummary,
            common::models::TranslatableString,
            common::models::Icon,
            common::models::StripeAccount,
            crate::types::general::ErrorResponse,
            crate::types::general::ErrorTranslationKey,
            crate::types::dashboard::CreateApp,
            crate::types::dashboard::AppUpdateSubmission,
        ))
    )
)]
pub struct ApiDoc;
