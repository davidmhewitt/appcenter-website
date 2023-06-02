use serde::{Serialize, Deserialize};

#[cfg(feature = "openapi")]
use utoipa::ToSchema;

#[derive(Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
pub struct StripeAccount {
    #[cfg_attr(feature = "openapi", schema(example = "acct_1NCliJPBGjCwUDHc"))]
    pub account_id: String,
    #[cfg_attr(feature = "openapi", schema(example = true))]
    pub charges_enabled: bool,
}