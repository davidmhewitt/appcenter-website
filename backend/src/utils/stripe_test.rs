#[cfg(test)]
#[inline]
pub fn stripe_client() -> stripe::Client
{
    let client = stripe::Client::from_url(
        option_env!("STRIPE_MOCKS_URL").unwrap_or("http://stripe:12111"),
        "sk_test_123",
    );

    client
}
