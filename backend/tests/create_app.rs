mod common;

use ::common::models::{NewGithubAuth, NewUser};
use anyhow::Result;

use backend::routes::dashboard::apps::{add_app_to_db, get_apps_from_db};
use backend::routes::users::register::insert_user_into_db;
use diesel_async::AsyncConnection;
use uuid::Uuid;

#[tokio::test]
async fn add_app_to_database() -> Result<()> {
    let pool = common::get_db_pool().await;

    let mut con = pool.get().await.expect("Unable to get database connection");

    con.begin_test_transaction()
        .await
        .expect("Unable to start test transaction");

    let apps = get_apps_from_db(&mut con, &Uuid::new_v4()).await?;
    assert!(apps.is_empty());

    let user1 = insert_user_into_db(
        &mut con,
        NewUser {
            email: "test1@example.com",
            password: None,
            is_active: true,
            is_admin: false,
        },
        NewGithubAuth {
            github_user_id: None,
            github_access_token: None,
            github_refresh_token: None,
        },
    )
    .await?;

    let user2 = insert_user_into_db(
        &mut con,
        NewUser {
            email: "test2@example.com",
            password: None,
            is_active: true,
            is_admin: false,
        },
        NewGithubAuth {
            github_user_id: None,
            github_access_token: None,
            github_refresh_token: None,
        },
    )
    .await?;

    add_app_to_db(
        &mut con,
        &user1,
        "com.github.fakeorg.fakeapp",
        "https://github.com/fakeorg/fakeapp.git",
        true,
    )
    .await?;

    let apps = get_apps_from_db(&mut con, &user1).await?;
    assert!(apps.len() == 1);

    let apps = get_apps_from_db(&mut con, &user2).await?;
    assert!(apps.is_empty());

    Ok(())
}
