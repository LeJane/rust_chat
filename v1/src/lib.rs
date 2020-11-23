#![recursion_limit = "256"]
#[macro_use]
extern crate diesel;

use anyhow::Result;
use byteorder::{LittleEndian, WriteBytesExt};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;
use utils::binary_helper::*;
use serde::{Serialize,Deserialize};

pub mod chat_system;
pub mod models;
pub mod router;
pub mod schema;
pub mod utils;

pub use router::{build_routers, RouterCode};
pub use utils::{
    connection::Connection,
    connection::ResponseContext,
    db::{get_slave_diesel_pool, get_master_diesel_pool},
    helper::get_guid_value,
    message::Message,
    message::{MessageNotifyType, MessageStateCode},
    redis_db::get_connection,
    redis_db::get_redis_connection_by_url,
    redis_db::store_chat_message_redis,
    redis_db::CHAT_GROUP_MESSAGE_REDIS_KEY_PREFIX,
    redis_db::CHAT_KINGDOM_MESSAGE_REDIS_KEY_PREFIX,
    redis_db::CHAT_PUBLISH_CHANNEL_REDIS_KEY,
    redis_db::CHAT_USER_MESSAGE_REDIS_KEY_PREFIX,
    redis_db::ONLINE_USERS_SETS_REDIS_KEY,
    router::ResponseResult,
    router::RouterRegister,
    thread_pool::ThreadPool,
};
use std::io::Cursor;


#[macro_export]
macro_rules! default_log_pre {
    ($code:expr,$user:expr) => {{
        format!(
            "code:{}\tuser:{}\tmethod:{}\tline:{:?}",
            $code,
            $user,
            function_name!(),
            line!()
        )
    }};
}



pub type Clients = Arc<Mutex<HashMap<u64, Connection>>>;

pub use models::chat_messages::{
    FrontDisplayChatMessage, FrontDisplayGroupChatMessage, FrontDisplayKingdomChatMessage,
    FrontDisplayP2pChatMessage,
};
pub use models::chat_user_unread_counts::FrontDisplayChatUserUnreadCount;
pub use utils::common::{BinaryEncode, BinaryDecode, deserialize_binary};


#[derive(Debug,Clone,Serialize,Deserialize)]
pub struct ChatMessageUnReadCount {
    pub kingdom: KingdomUnReadCountMsg,
    pub groups: Vec<GroupUnReadCountMsg>,
    pub p2ps: Vec<FrontDisplayChatUserUnreadCount>,
}

#[derive(Debug,Clone,Serialize,Deserialize)]
pub struct KingdomUnReadCountMsg {
    pub unread_count: i32,
    pub latest_message: Option<FrontDisplayKingdomChatMessage>,
}

#[derive(Debug,Clone,Serialize,Deserialize)]
pub struct GroupUnReadCountMsg {
    pub unread_count: i32,
    pub latest_message: FrontDisplayGroupChatMessage,
}

impl BinaryEncode for KingdomUnReadCountMsg {
    fn encode(&self) -> Result<Vec<u8>> {
        let mut encoded = Vec::new();
        encoded.write_i32::<LittleEndian>(self.unread_count)?;
        let msg = self.latest_message.encode()?;
        encoded.extend(msg);

        encoded.encode()
    }
}

impl BinaryEncode for GroupUnReadCountMsg {
    fn encode(&self) -> Result<Vec<u8>> {
        let mut encoded = Vec::new();

        encoded.write_i32::<LittleEndian>(self.unread_count)?;
        let msg = self.latest_message.encode()?;
        encoded.extend(msg);

        encoded.encode()
    }
}

impl BinaryEncode for ChatMessageUnReadCount {
    fn encode(&self) -> Result<Vec<u8>> {
        let mut encoded = Vec::new();

        //kingdoms
        let kingdoms = self.kingdom.encode()?;
        encoded.extend(kingdoms);
        //skills
        let groups = self.groups.encode()?;
        encoded.extend(groups);

        //p2ps
        let p2ps = self.p2ps.encode()?;
        encoded.extend(p2ps);

        //set item length
        encoded.encode()
    }
}


impl<'a> BinaryDecode<'a> for ChatMessageUnReadCount {
    fn decode(
        cursor: &mut Cursor<&'a [u8]>,
        bytes: &'a [u8],
    ) -> Result<ChatMessageUnReadCount> {
        let _item_length = binary_read_i16(cursor)?;
        let kingdom:KingdomUnReadCountMsg = deserialize_binary(cursor, bytes)?;
        let groups:Vec<GroupUnReadCountMsg> = deserialize_binary(cursor, bytes)?;
        let p2ps:Vec<FrontDisplayChatUserUnreadCount> = deserialize_binary(cursor, bytes)?;

        let data = ChatMessageUnReadCount {
            kingdom,
            groups,
            p2ps
        };

        Ok(data)
    }
}


impl<'a> BinaryDecode<'a> for KingdomUnReadCountMsg {
    fn decode(
        cursor: &mut Cursor<&'a [u8]>,
        bytes: &'a [u8],
    ) -> Result<KingdomUnReadCountMsg> {
        let unread_count = binary_read_i32(cursor)?;
        let item_length = binary_read_i16(cursor)?;

        let mut latest_message = None;

        if item_length > 0 {
            let data: FrontDisplayKingdomChatMessage = deserialize_binary(cursor, bytes)?;
            latest_message = Some(data);
        }


        let data = KingdomUnReadCountMsg {
            unread_count,
            latest_message,
        };

        Ok(data)
    }
}


impl<'a> BinaryDecode<'a> for GroupUnReadCountMsg {
    fn decode(
        cursor: &mut Cursor<&'a [u8]>,
        bytes: &'a [u8],
    ) -> Result<GroupUnReadCountMsg> {
        let unread_count = binary_read_i32(cursor)?;
        let _item_length = binary_read_i16(cursor)?;

        let latest_message: FrontDisplayGroupChatMessage = deserialize_binary(cursor, bytes)?;


        let data = GroupUnReadCountMsg {
            unread_count,
            latest_message,
        };

        Ok(data)
    }
}
