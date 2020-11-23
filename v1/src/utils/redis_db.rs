use guid_create::GUID;
use murmur3::murmur3_x64_128;
use redis::Commands;
use redis::{Connection, RedisResult};
use std::env;
use std::io::Cursor;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::sync::Mutex;
use tracing::error;

pub fn get_connection() -> RedisResult<Arc<Mutex<Connection>>> {
    let addr = env::var("REDIS_URL").expect("REDIS_URL not found.");
    let client = redis::Client::open(addr)?;

    let conn = client.get_connection()?;

    Ok(Arc::new(Mutex::new(conn)))
}

pub fn get_redis_connection_by_url() -> RedisResult<Connection> {
    let addr = env::var("REDIS_URL").expect("REDIS_URL not found.");
    let client = redis::Client::open(addr)?;

    let conn = client.get_connection();

    conn
}

pub const ONLINE_USERS_SETS_REDIS_KEY: &str = "online_users";
pub const CHAT_KINGDOM_MESSAGE_REDIS_KEY_PREFIX: &str = "chat_kingdom_message_";
pub const CHAT_GROUP_MESSAGE_REDIS_KEY_PREFIX: &str = "chat_group_message_";
pub const CHAT_USER_MESSAGE_REDIS_KEY_PREFIX: &str = "chat_user_message_"; //p2p->format(from_uid:to_uid)
pub const CHAT_PUBLISH_CHANNEL_REDIS_KEY: &str = "chat_publish_channel"; //format->(tid:$:value:$:from_uid:$:to_uid:$:content)

pub struct RedisDbConn {
    pub conn: Connection,
}

impl RedisDbConn {
    pub fn new() -> Self {
        let addr = env::var("REDIS_URL").expect("REDIS_URL not found.");
        let client = redis::Client::open(addr).unwrap();

        let conn = client.get_connection().unwrap();

        RedisDbConn { conn }
    }
}

pub fn store_chat_message_redis(tid: u8, from_uid: u64, dst_id: u64, msg: &[u8]) {
    //store message to redis

    let mut redis_conn = get_redis_connection_by_url().unwrap();

    let start = SystemTime::now();
    let since_the_epoch = start
        .duration_since(UNIX_EPOCH)
        .expect("Time went backwards")
        .as_secs();

    //create message id
    let guid = GUID::rand();
    let guid = guid.to_string();
    let guid_str = guid.replace("-", "");
    let hash_result = murmur3_x64_128(&mut Cursor::new(guid_str), 0).unwrap();
    let msg_id = (hash_result >> 65) as u64;

    match tid {
        1 => {
            //kingdom
            let msg = format!("{}:$:{}", msg_id, std::str::from_utf8(msg).unwrap());
            let key = format!("{}{}", CHAT_KINGDOM_MESSAGE_REDIS_KEY_PREFIX, dst_id);
            if let Err(e) = redis_conn.zadd::<&str, u64, &str, u64>(&key, &msg, since_the_epoch) {
                error!("failed exec kingdom zadd data:{} to redis db: {}", msg, e);
                return;
            }

            let kingdom_ttl: i64 = match redis_conn.ttl(&key) {
                Ok(v) => v,
                Err(e) => {
                    error!("failed get kingdom expire time:{} error:{}", &key, e);
                    return;
                }
            };

            if kingdom_ttl <= 0 {
                if let Err(e) = redis_conn.expire::<&str, isize>(&key, 2 * 24 * 3600) {
                    error!("failed set kingdom {} expire time error:{}", &key, e);
                    return;
                }
            }
        }
        2 => {
            //group chat
            let msg = format!("{}:$:{}", msg_id, std::str::from_utf8(msg).unwrap());
            let key = format!("{}{}", CHAT_GROUP_MESSAGE_REDIS_KEY_PREFIX, dst_id);
            if let Err(e) = redis_conn.zadd::<&str, u64, &str, u64>(&key, &msg, since_the_epoch) {
                error!("failed exec group zadd data:{} to redis db: {}", msg, e);
                return;
            }

            let group_ttl: i64 = match redis_conn.ttl(&key) {
                Ok(v) => v,
                Err(e) => {
                    error!("failed get group expire time:{} error:{}", &key, e);
                    return;
                }
            };

            if group_ttl <= 0 {
                if let Err(e) = redis_conn.expire::<&str, isize>(&key, 2 * 24 * 3600) {
                    error!("failed set group {} expire time error:{}", &key, e);
                    return;
                }
            }
        }
        3 => {
            //p2p
            let msg = format!("{}:$:{}", msg_id, std::str::from_utf8(msg).unwrap());
            let key = format!(
                "{}{}:{}",
                CHAT_USER_MESSAGE_REDIS_KEY_PREFIX, from_uid, dst_id
            );
            if let Err(e) = redis_conn.zadd::<&str, u64, &str, u64>(&key, &msg, since_the_epoch) {
                error!("failed exec user zadd data:{} to redis db: {}", msg, e);
                return;
            }

            let p2p_ttl: i64 = match redis_conn.ttl(&key) {
                Ok(v) => v,
                Err(e) => {
                    error!("failed get user expire time:{} error:{}", &key, e);
                    return;
                }
            };

            if p2p_ttl <= 0 {
                if let Err(e) = redis_conn.expire::<&str, isize>(&key, 2 * 24 * 3600) {
                    error!("failed set user {} expire time error:{}", &key, e);
                    return;
                }
            }
        }
        4 => {}
        _ => {}
    }
}
