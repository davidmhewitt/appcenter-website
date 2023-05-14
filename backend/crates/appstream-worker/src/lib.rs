mod appstream_collection_sorters;
mod appstream_version_utils;
mod appstream_worker;
mod redis_utils;

pub use self::appstream_worker::AppstreamWorker;
pub use self::appstream_worker::ComponentSummary;
pub use self::appstream_worker::Icon;
pub use self::appstream_worker::TranslatableString;

pub const RECENTLY_ADDED_REDIS_KEY: &str = "appstream_worker/recently_added";
pub const RECENTLY_UPDATED_REDIS_KEY: &str = "appstream_worker/recently_updated";
pub const ALL_APP_IDS_REDIS_KEY: &str = "appstream_worker/all_app_ids";
