use serde::{Deserialize, Serialize};
#[cfg(feature = "openapi")]
use utoipa::IntoParams;

#[derive(Deserialize, Serialize, Debug)]
#[cfg_attr(feature = "openapi", derive(IntoParams))]
pub struct AppPaymentRequest {
    pub app_name: String,
    pub app_id: String,
    /// The amount to pay for the app in cents USD
    pub amount: u32,
}
