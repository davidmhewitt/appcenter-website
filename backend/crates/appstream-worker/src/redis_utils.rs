use redis::Connection;

pub(crate) async fn hset(redis_con: &mut Connection, key: &str, field: &str, value: &str) {
    if let Err(e) = redis::Cmd::hset(key, field, value).query::<i32>(redis_con) {
        tracing::warn!("Error with redis hset command: {}", e);
    }
}
