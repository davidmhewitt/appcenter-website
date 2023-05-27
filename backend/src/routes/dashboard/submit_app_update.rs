use actix_web::{post, web::Json, HttpResponse};
use background_worker::tasks::SubmitAppUpdate;

use crate::{extractors::AuthedUser, types::dashboard::AppUpdateSubmission};

#[cfg_attr(feature = "openapi", utoipa::path(
    path = "/dashboard/submit_app_update",
    request_body = AppUpdateSubmission,
))]
#[post("/submit_app_update")]
#[cfg_attr(not(coverage), tracing::instrument(name = "Submitting app update", skip(user)))]
pub async fn submit(user: AuthedUser, submission: Json<AppUpdateSubmission>) -> HttpResponse {
    let task = SubmitAppUpdate::new(
        submission.app_id.to_owned(),
        submission.version_tag.to_owned(),
        user.uuid,
    );

    if background_worker::insert_task(&task).is_err() {
        return HttpResponse::InternalServerError().finish();
    }

    HttpResponse::Ok().finish()
}
