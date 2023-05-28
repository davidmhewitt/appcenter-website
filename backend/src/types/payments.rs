use serde::{Deserialize, Serialize};
#[cfg(feature = "openapi")]
use utoipa::ToSchema;

#[derive(Deserialize, Serialize, Debug)]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
pub struct AppPaymentRequest {
    #[cfg_attr(feature = "openapi", schema(example = "Torrential"))]
    pub app_name: String,
    #[cfg_attr(
        feature = "openapi",
        schema(example = "com.github.davidmhewitt.torrential")
    )]
    pub app_id: String,
    #[cfg_attr(feature = "openapi", schema(example = 300))]
    pub amount: u32,
    #[cfg_attr(
        feature = "openapi",
        schema(example = "pk_live_Uhb96elhovWNRGkq07m2FRO9008Ia8OtVa")
    )]
    pub stripe_connect_id: String,
}
