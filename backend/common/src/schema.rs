// @generated automatically by Diesel CLI.

pub mod sql_types {
    #[derive(diesel::sql_types::SqlType)]
    #[diesel(postgres_type(name = "fang_task_state"))]
    pub struct FangTaskState;
}

diesel::table! {
    app_owners (user_id, app_id) {
        user_id -> Uuid,
        app_id -> Text,
        verified_owner -> Bool,
    }
}

diesel::table! {
    apps (id) {
        id -> Text,
        repository -> Text,
        is_verified -> Bool,
        last_submitted_version -> Nullable<Text>,
        first_seen -> Nullable<Timestamptz>,
        last_update -> Nullable<Timestamptz>,
    }
}

diesel::table! {
    use diesel::sql_types::*;
    use super::sql_types::FangTaskState;

    fang_tasks (id) {
        id -> Uuid,
        metadata -> Jsonb,
        error_message -> Nullable<Text>,
        state -> FangTaskState,
        task_type -> Varchar,
        uniq_hash -> Nullable<Bpchar>,
        retries -> Int4,
        scheduled_at -> Timestamptz,
        created_at -> Timestamptz,
        updated_at -> Timestamptz,
    }
}

diesel::table! {
    github_auth (user_id) {
        user_id -> Uuid,
        github_user_id -> Nullable<Text>,
        github_access_token -> Nullable<Text>,
        github_refresh_token -> Nullable<Text>,
    }
}

diesel::table! {
    user_profile (id) {
        id -> Uuid,
        user_id -> Uuid,
        profile_picture_url -> Nullable<Text>,
        github_link -> Nullable<Text>,
    }
}

diesel::table! {
    users (id) {
        id -> Uuid,
        email -> Text,
        password -> Nullable<Text>,
        is_active -> Bool,
        is_admin -> Bool,
        date_joined -> Timestamptz,
    }
}

diesel::joinable!(app_owners -> apps (app_id));
diesel::joinable!(app_owners -> users (user_id));
diesel::joinable!(github_auth -> users (user_id));
diesel::joinable!(user_profile -> users (user_id));

diesel::allow_tables_to_appear_in_same_query!(
    app_owners,
    apps,
    fang_tasks,
    github_auth,
    user_profile,
    users,
);
