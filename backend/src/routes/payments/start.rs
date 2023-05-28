use actix_web::{post, HttpResponse};
use stripe::{
    CheckoutSession, CheckoutSessionMode, Client, CreateCheckoutSession,
    CreateCheckoutSessionLineItems, CreateCheckoutSessionLineItemsPriceData,
    CreateCheckoutSessionLineItemsPriceDataProductData, CreateCheckoutSessionPaymentIntentData,
    CreateCheckoutSessionPaymentIntentDataTransferData, Currency, Metadata,
};

use crate::types::payments::AppPaymentRequest;

#[post("/start")]
#[cfg_attr(
    not(coverage),
    tracing::instrument(name = "starting a payment", skip(stripe_client))
)]
pub async fn start(
    stripe_client: actix_web::web::Data<Client>,
    app_information: actix_web::web::Json<AppPaymentRequest>,
) -> HttpResponse {
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
            destination: app_information.stripe_connect_id.to_owned(),
            ..Default::default()
        }),
        on_behalf_of: Some(app_information.stripe_connect_id.to_owned()),
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

#[cfg(test)]
mod tests {
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
        let subscriber = common::telemetry::get_subscriber(false);
        common::telemetry::init_subscriber(subscriber);

        let stripe_client = actix_web::web::Data::new(stripe::Client::from_url(
            "http://stripe:12111",
            "sk_test_123",
        ));
        let mut app =
            actix_web::test::init_service(App::new().service(start).app_data(stripe_client)).await;

        let req = TestRequest::post()
            .set_json(AppPaymentRequest {
                app_name: "Torrential".into(),
                app_id: "com.github.davidmhewitt.torrential".into(),
                amount: 300,
                stripe_connect_id: "1234".into(),
            })
            .uri("/start")
            .to_request();
        let response = actix_web::test::try_call_service(&mut app, req).await?;

        assert!(response.status().is_redirection());
        assert!(response.headers().contains_key("Location"));

        Ok(())
    }
}
