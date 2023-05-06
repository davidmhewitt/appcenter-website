use deadpool_redis::Connection;

pub(crate) async fn del(redis_con: &mut Connection, key: &str) {
    if let Err(e) = deadpool_redis::redis::Cmd::del(key)
        .query_async::<_, i32>(redis_con)
        .await
    {
        tracing::warn!("Error deleting key from redis: {}", e);
    }
}

pub(crate) async fn rpush(redis_con: &mut Connection, key: &str, value: &str) {
    if let Err(e) = deadpool_redis::redis::Cmd::rpush(key, value)
        .query_async::<_, i32>(redis_con)
        .await
    {
        tracing::warn!("Error with redis rpush command: {}", e);
    }
}

pub(crate) async fn lpush(redis_con: &mut Connection, key: &str, value: &str) {
    if let Err(e) = deadpool_redis::redis::Cmd::lpush(key, value)
        .query_async::<_, i32>(redis_con)
        .await
    {
        tracing::warn!("Error with redis lpush command: {}", e);
    }
}

pub(crate) async fn ltrim(redis_con: &mut Connection, key: &str, start: isize, stop: isize) {
    if let Err(e) = deadpool_redis::redis::Cmd::ltrim(key, start, stop)
        .query_async::<_, String>(redis_con)
        .await
    {
        tracing::warn!("Error with redis ltrim command: {}", e);
    }
}
