use appstream::Component;
use semver::Version;

pub fn get_latest_component_version(component: &Component) -> Option<Version> {
    let mut versions = component.releases.to_owned();
    versions.sort_unstable_by(|a, b| {
        if let (Some(a), Some(b)) = (a.date, b.date) {
            if a != b {
                return b.cmp(&a);
            }
        }

        let a_ver = lenient_semver::parse(&a.version).unwrap_or_else(|_| Version::new(0, 0, 0));
        let b_ver = lenient_semver::parse(&b.version).unwrap_or_else(|_| Version::new(0, 0, 0));

        b_ver.cmp(&a_ver)
    });

    if let Some(v) = versions.first() {
        if let Ok(v) = lenient_semver::parse(&v.version) {
            return Some(v);
        }
    }

    None
}

#[cfg(test)]
mod tests {
    use appstream::{
        builders::{ComponentBuilder, ReleaseBuilder},
        TranslatableString,
    };
    use chrono::TimeZone;

    use super::*;

    #[test]
    fn version_comparison() {
        let c1: appstream::Component = ComponentBuilder::default()
            .id("com.example.foobar".into())
            .name(TranslatableString::with_default("Foo Bar"))
            .metadata_license("CC0-1.0".into())
            .summary(TranslatableString::with_default("A foo-ish bar"))
            .release(ReleaseBuilder::new("1.2").build())
            .release(ReleaseBuilder::new("1.3").build())
            .release(ReleaseBuilder::new("1.3.19").build())
            .build();

        assert_eq!(
            Some(lenient_semver::parse("1.3.19").unwrap()),
            get_latest_component_version(&c1)
        );

        let c2: appstream::Component = ComponentBuilder::default()
            .id("com.example.foobar".into())
            .name(TranslatableString::with_default("Foo Bar"))
            .metadata_license("CC0-1.0".into())
            .summary(TranslatableString::with_default("A foo-ish bar"))
            .release(ReleaseBuilder::new("0.1").build())
            .release(ReleaseBuilder::new("1.0").build())
            .release(
                ReleaseBuilder::new("1.0.2")
                    .date(
                        chrono::Utc
                            .with_ymd_and_hms(2023, 01, 01, 12, 12, 13)
                            .unwrap(),
                    )
                    .build(),
            )
            .release(
                ReleaseBuilder::new("1.0.12")
                    .date(
                        chrono::Utc
                            .with_ymd_and_hms(2023, 01, 01, 12, 12, 10)
                            .unwrap(),
                    )
                    .build(),
            )
            .build();

        assert_eq!(
            Some(lenient_semver::parse("1.0.2").unwrap()),
            get_latest_component_version(&c2)
        );
    }
}
