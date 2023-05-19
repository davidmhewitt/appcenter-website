// @generated automatically by Diesel CLI.

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

diesel::allow_tables_to_appear_in_same_query!(app_owners, apps, github_auth, user_profile, users,);
