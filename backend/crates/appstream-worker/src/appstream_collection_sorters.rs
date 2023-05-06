use std::cmp::Ordering;

use appstream::{builders::ReleaseBuilder, Component};
use chrono::TimeZone;

pub(crate) fn sort_newly_released_components_first(a: &Component, b: &Component) -> Ordering {
    b.releases
        .first()
        .unwrap_or(
            &ReleaseBuilder::new("0.0.0")
                .date(chrono::Utc.timestamp_opt(0, 0).unwrap())
                .build(),
        )
        .date
        .unwrap_or(chrono::Utc.timestamp_opt(0, 0).unwrap())
        .cmp(
            &a.releases
                .first()
                .unwrap_or(
                    &ReleaseBuilder::new("0.0.0")
                        .date(chrono::Utc.timestamp_opt(0, 0).unwrap())
                        .build(),
                )
                .date
                .unwrap_or(chrono::Utc.timestamp_opt(0, 0).unwrap()),
        )
}

pub(crate) fn sort_recent_initial_release_components_first(
    a: &Component,
    b: &Component,
) -> Ordering {
    b.releases
        .last()
        .unwrap_or(
            &ReleaseBuilder::new("0.0.0")
                .date(chrono::Utc.timestamp_opt(0, 0).unwrap())
                .build(),
        )
        .date
        .unwrap_or(chrono::Utc.timestamp_opt(0, 0).unwrap())
        .cmp(
            &a.releases
                .last()
                .unwrap_or(
                    &ReleaseBuilder::new("0.0.0")
                        .date(chrono::Utc.timestamp_opt(0, 0).unwrap())
                        .build(),
                )
                .date
                .unwrap_or(chrono::Utc.timestamp_opt(0, 0).unwrap()),
        )
}

#[cfg(test)]
mod tests {
    use appstream::{
        builders::{CollectionBuilder, ComponentBuilder, ReleaseBuilder},
        TranslatableString,
    };
    use chrono::TimeZone;

    use super::*;

    #[test]
    fn version_comparison() {
        let c = CollectionBuilder::new("0.8")
            .component(
                ComponentBuilder::default()
                    .id("org.foo.bar".into())
                    .name(TranslatableString::with_default("Foo Bar"))
                    .release(
                        ReleaseBuilder::new("1.0")
                            .date(
                                chrono::Utc
                                    .with_ymd_and_hms(2022, 06, 23, 23, 01, 14)
                                    .unwrap(),
                            )
                            .build(),
                    )
                    .release(
                        ReleaseBuilder::new("0.5")
                            .date(
                                chrono::Utc
                                    .with_ymd_and_hms(2021, 12, 23, 06, 59, 30)
                                    .unwrap(),
                            )
                            .build(),
                    )
                    .build(),
            )
            .component(
                ComponentBuilder::default()
                    .id("org.bar.foo".into())
                    .name(TranslatableString::with_default("Bar Foo"))
                    .release(
                        ReleaseBuilder::new("4.0")
                            .date(
                                chrono::Utc
                                    .with_ymd_and_hms(2023, 06, 23, 23, 01, 14)
                                    .unwrap(),
                            )
                            .build(),
                    )
                    .release(
                        ReleaseBuilder::new("0.1")
                            .date(
                                chrono::Utc
                                    .with_ymd_and_hms(2020, 12, 23, 06, 59, 30)
                                    .unwrap(),
                            )
                            .build(),
                    )
                    .build(),
            )
            .build();

        let mut components = c.components.to_owned();
        components.sort_unstable_by(sort_newly_released_components_first);
        assert_eq!(components.first().unwrap().id.0, "org.bar.foo");

        components.sort_unstable_by(sort_recent_initial_release_components_first);
        assert_eq!(components.first().unwrap().id.0, "org.foo.bar");
    }
}
