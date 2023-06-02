use actix_web::{post, web::Data, HttpResponse};
use anyhow::anyhow;
use diesel::{dsl::count, ExpressionMethods, QueryDsl};
use diesel_async::{
    pooled_connection::bb8::{Pool, PooledConnection},
    AsyncPgConnection, RunQueryDsl,
};
use uuid::Uuid;

use crate::extractors::AuthedUser;

use super::link_stripe_account::get_stripe_account_id_for_user;

#[cfg_attr(
    not(coverage),
    tracing::instrument(name = "Enabling payments for app", skip(user))
)]
#[post("/enable_app_payments/{app_id}")]
pub async fn enable_app_payments(
    user: AuthedUser,
    app_id: actix_web::web::Path<(String,)>,
    pool: Data<Pool<AsyncPgConnection>>,
) -> HttpResponse {
    let mut con = match pool.get().await {
        Ok(c) => c,
        Err(e) => {
            tracing::error!("Unable to get DB connection for app payments: {}", e);
            return HttpResponse::InternalServerError().finish();
        }
    };

    let stripe_id = match get_stripe_account_id_for_user(&mut con, user.uuid).await {
        Ok(a) => a,
        Err(e) => {
            tracing::error!("Couldn't get stripe account ID for user: {}", e);
            return HttpResponse::InternalServerError().finish();
        }
    };

    if let Err(e) =
        set_stripe_token_on_app(&mut con, &app_id.into_inner().0, &user.uuid, &stripe_id).await
    {
        tracing::error!("Error setting stripe account id on app: {}", e);
        return HttpResponse::InternalServerError().finish();
    }

    HttpResponse::Ok().finish()
}

async fn set_stripe_token_on_app(
    con: &mut PooledConnection<'_, AsyncPgConnection>,
    app_id: &str,
    uuid: &Uuid,
    stripe_acc_id: &str,
) -> anyhow::Result<()> {
    use common::schema::apps;

    if !user_owns_app(con, app_id, uuid).await? {
        return Err(anyhow!("Current user does not own this app"));
    }

    let rows_updated = diesel::update(apps::table.filter(apps::id.eq(app_id)))
        .set(apps::stripe_connect_id.eq(stripe_acc_id))
        .execute(con)
        .await?;

    if rows_updated < 1 {
        return Err(anyhow!("App ID not found"));
    }

    Ok(())
}

async fn user_owns_app(
    con: &mut PooledConnection<'_, AsyncPgConnection>,
    app_id: &str,
    uuid: &Uuid,
) -> Result<bool, diesel::result::Error> {
    use common::schema::app_owners;
    use common::schema::apps::dsl::*;

    let num_apps = apps
        .inner_join(app_owners::table)
        .select(count(id))
        .filter(app_owners::user_id.eq(uuid))
        .filter(id.eq(app_id))
        .filter(app_owners::verified_owner.eq(true))
        .get_result::<i64>(con)
        .await?;

    Ok(num_apps > 0)
}

#[cfg(test)]
mod tests {
    use diesel_async::{AsyncConnection, RunQueryDsl};

    use crate::utils::db_test::{create_app, create_user};

    use super::*;

    #[tokio::test]
    async fn test_update_app() -> anyhow::Result<()> {
        use common::schema::apps::dsl::*;

        let db_pool = crate::utils::db_test::db_pool().await;
        let mut con = db_pool.get().await?;

        con.begin_test_transaction().await?;

        let user = create_user(&mut con, true).await?;
        let app = create_app(&mut con, Some(&user)).await?;

        set_stripe_token_on_app(&mut con, &app, &user, "acc_1234").await?;

        let returned_token = apps
            .filter(id.eq(app))
            .select(stripe_connect_id)
            .get_result::<Option<String>>(&mut con)
            .await?;

        assert_eq!(returned_token, Some("acc_1234".into()));

        Ok(())
    }
}
