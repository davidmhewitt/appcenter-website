use actix_web::{post, HttpResponse};
use stripe::{
    CheckoutSession, CheckoutSessionMode, Client, CreateCheckoutSession,
    CreateCheckoutSessionLineItems, CreateCheckoutSessionLineItemsPriceData,
    CreateCheckoutSessionLineItemsPriceDataProductData, Currency, Expandable,
};

#[post("/start")]
#[cfg_attr(
    not(coverage),
    tracing::instrument(name = "starting a payment", skip(stripe_client))
)]
pub async fn start(stripe_client: actix_web::web::Data<Client>) -> HttpResponse {
    // finally, create a checkout session for this product / price
    let checkout_session = {
        let mut params = CreateCheckoutSession::new("http://test.com/success");
        params.cancel_url = Some("http://test.com/cancel");
        params.mode = Some(CheckoutSessionMode::Payment);
        params.line_items = Some(vec![CreateCheckoutSessionLineItems {
            quantity: Some(3),
            price_data: Some(CreateCheckoutSessionLineItemsPriceData {
                currency: Currency::USD,
                unit_amount: Some(300),
                product_data: Some(CreateCheckoutSessionLineItemsPriceDataProductData {
                    name: "Test Product".into(),
                    ..Default::default()
                }),
                ..Default::default()
            }),
            ..Default::default()
        }]);
        params.expand = &["line_items", "line_items.data.price.product"];

        CheckoutSession::create(&stripe_client, params)
            .await
            .unwrap()
    };

    println!(
        "created a {} checkout session for {} {:?} for {} {} at {}",
        checkout_session.payment_status,
        checkout_session.line_items.data[0].quantity.unwrap(),
        match checkout_session.line_items.data[0]
            .price
            .as_ref()
            .unwrap()
            .product
            .as_ref()
            .unwrap()
        {
            Expandable::Object(p) => p.name.as_ref().unwrap(),
            _ => panic!("product not found"),
        },
        checkout_session.amount_subtotal.unwrap() / 100,
        checkout_session.line_items.data[0]
            .price
            .as_ref()
            .unwrap()
            .currency
            .unwrap(),
        checkout_session.url.unwrap()
    );

    HttpResponse::Ok().finish()
}
