use std::collections::BTreeMap;

use appstream::Component;
use serde::{Serialize, Deserialize};
use utoipa::ToSchema;

#[derive(Serialize, Deserialize, ToSchema)]
#[schema(example = json!({"C": "Welcome", "ja": "いらっしゃいませ"}))]
pub struct TranslatableString(pub BTreeMap<String, String>);

impl TranslatableString {
    pub fn from(original: appstream::TranslatableString) -> Self {
        Self(original.0)
    }
}

#[derive(Serialize, Deserialize, ToSchema)]

pub struct Icon {
    #[schema(example = "com.github.alexkdeveloper.bmi.png")]
    path: String,
    #[schema(example = 64)]
    width: Option<u32>,
    #[schema(example = 64)]
    height: Option<u32>,
}

#[derive(Serialize, Deserialize, ToSchema)]
pub struct ComponentSummary {
    #[schema(example = "com.github.davidmhewitt.torrential")]
    id: String,
    name: TranslatableString,
    summary: Option<TranslatableString>,
    icons: Vec<Icon>,
}

impl From<&Component> for ComponentSummary {
    fn from(value: &Component) -> Self {
        Self {
            id: value.id.0.to_owned(),
            name: TranslatableString::from(value.name.to_owned()),
            summary: value.summary.to_owned().map(TranslatableString::from),
            icons: value
                .icons
                .iter()
                .filter_map(|i| match i {
                    appstream::enums::Icon::Cached {
                        path,
                        width,
                        height,
                    } => Some(Icon {
                        path: path.to_string_lossy().into_owned(),
                        width: *width,
                        height: *height,
                    }),
                    _ => None,
                })
                .collect(),
        }
    }
}