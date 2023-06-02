use actix_web::{get, HttpResponse};
use anyhow::anyhow;
use diesel::{ExpressionMethods, QueryDsl};
use diesel_async::{
    pooled_connection::bb8::{Pool, PooledConnection},
    AsyncPgConnection, RunQueryDsl,
};
use stripe::{
    CheckoutSession, CheckoutSessionMode, Client, CreateCheckoutSession,
    CreateCheckoutSessionLineItems, CreateCheckoutSessionLineItemsPriceData,
    CreateCheckoutSessionLineItemsPriceDataProductData, CreateCheckoutSessionPaymentIntentData,
    CreateCheckoutSessionPaymentIntentDataTransferData, Currency, Metadata,
};

use crate::types::payments::AppPaymentRequest;

#[get("/start")]
#[cfg_attr(
    not(coverage),
    tracing::instrument(name = "starting a payment", skip(stripe_client))
)]
pub async fn start(
    stripe_client: actix_web::web::Data<Client>,
    app_information: actix_web::web::Query<AppPaymentRequest>,
    pool: actix_web::web::Data<Pool<AsyncPgConnection>>,
) -> HttpResponse {
    let mut con = match pool.get().await {
        Ok(c) => c,
        Err(e) => {
            tracing::error!("Unable to get db connection: {}", e);
            return HttpResponse::InternalServerError().finish();
        }
    };

    let stripe_account_id =
        match get_stripe_account_for_app(&mut con, &app_information.app_id).await {
            Ok(a) => a,
            Err(e) => {
                tracing::error!("Error fetching stripe account for app: {}", e);
                return HttpResponse::InternalServerError().finish();
            }
        };

    // TODO: Fix these URLs
    let mut params = CreateCheckoutSession::new("http://test.com/success");
    params.cancel_url = Some("http://test.com/cancel");
    params.mode = Some(CheckoutSessionMode::Payment);
    params.line_items = Some(vec![CreateCheckoutSessionLineItems {
        quantity: Some(1),
        price_data: Some(CreateCheckoutSessionLineItemsPriceData {
            currency: Currency::USD,
            unit_amount: Some(app_information.amount.into()),
            product_data: Some(CreateCheckoutSessionLineItemsPriceDataProductData {
                name: app_information.app_name.to_owned(),
                ..Default::default()
            }),
            ..Default::default()
        }),
        ..Default::default()
    }]);
    params.metadata = Some(Metadata::from([(
        "app_id".into(),
        app_information.app_id.to_owned(),
    )]));
    params.payment_intent_data = Some(CreateCheckoutSessionPaymentIntentData {
        application_fee_amount: Some(calculate_fee(app_information.amount).into()),
        transfer_data: Some(CreateCheckoutSessionPaymentIntentDataTransferData {
            destination: stripe_account_id.to_owned(),
            ..Default::default()
        }),
        on_behalf_of: Some(stripe_account_id),
        ..Default::default()
    });

    let checkout_session = match CheckoutSession::create(&stripe_client, params).await {
        Ok(s) => s,
        Err(e) => {
            tracing::error!("Error creating stripe checkout session: {}", e);
            return HttpResponse::InternalServerError().finish();
        }
    };

    HttpResponse::SeeOther()
        .insert_header((
            actix_web::http::header::LOCATION,
            checkout_session.url.unwrap(),
        ))
        .finish()
}

fn calculate_fee(amount: u32) -> u32 {
    let fee = ((amount as f64) * 0.3).round() as u32;
    if fee >= 50 {
        fee
    } else {
        50
    }
}

async fn get_stripe_account_for_app(
    con: &mut PooledConnection<'_, AsyncPgConnection>,
    app_id: &str,
) -> anyhow::Result<String> {
    use common::schema::apps::dsl::*;

    Ok(apps
        .filter(id.eq(app_id))
        .select(stripe_connect_id)
        .get_result::<Option<String>>(con)
        .await?
        .ok_or(anyhow!("Couldn't find app"))?)
}

#[cfg(test)]
mod tests {
    use crate::utils::{
        db_test::{create_app, db_pool},
        stripe_test,
    };

    use super::*;
    use actix_web::{test::TestRequest, App};

    #[test]
    fn test_fee_calculations() {
        assert_eq!(50, calculate_fee(100));
        assert_eq!(50, calculate_fee(150));
        assert_eq!(51, calculate_fee(170));
        assert_eq!(60, calculate_fee(200));
        assert_eq!(150, calculate_fee(500));
    }

    #[actix_web::test]
    async fn test_start_payment() -> Result<(), actix_web::Error> {
        use common::schema::apps;

        let pool = db_pool().await;

        let stripe_client = actix_web::web::Data::new(stripe_test::stripe_client());

        let mut server = actix_web::test::init_service(
            App::new()
                .service(start)
                .app_data(stripe_client)
                .app_data(actix_web::web::Data::new(pool.clone())),
        )
        .await;

        let mut con = pool.get().await.expect("Unable to get pool connection");
        let app = create_app(&mut con, None, Some("acct_1234"))
            .await
            .expect("Unable to create test app");

        let req = TestRequest::get()
            .uri(&format!(
                "/start?app_id={}&app_name=Torrential&amount=300",
                app
            ))
            .to_request();

        let response = actix_web::test::try_call_service(&mut server, req).await;

        diesel::delete(apps::table.filter(apps::id.eq(app)))
            .execute(&mut con)
            .await
            .ok();

        assert!(response.is_ok());

        let response = response.unwrap();
        assert!(response.status().is_redirection());
        assert!(response.headers().contains_key("Location"));

        Ok(())
    }
}
