use crate::models::chat_groups::ChatGroup;
use crate::models::user::{FrontDisplayChatUser, User};
use crate::schema::chat_messages;
use crate::{get_guid_value, BinaryEncode, BinaryDecode, deserialize_binary, utils::binary_helper::*};
use anyhow::{Result, Context};
use byteorder::{LittleEndian, WriteBytesExt};
use chrono::{NaiveDateTime, Utc};
use diesel::prelude::*;
use std::io::Cursor;
use serde::{Serialize,Deserialize};

#[derive(Debug, QueryableByName, Clone, Identifiable, Queryable, Associations)]
#[primary_key(mid)]
pub struct ChatMessage {
    #[sql_type = "::diesel::sql_types::BigInt"]
    pub mid: i64,
    #[sql_type = "::diesel::sql_types::BigInt"]
    pub send_id: i64,
    #[sql_type = "::diesel::sql_types::BigInt"]
    pub to_id: i64,
    #[sql_type = "::diesel::sql_types::VarChar"]
    pub content: String,
    #[sql_type = "::diesel::sql_types::BigInt"]
    pub created_timestamp: i64,
    #[sql_type = "::diesel::sql_types::SmallInt"]
    pub kind: i16,
    #[sql_type = "::diesel::sql_types::Timestamp"]
    pub modify_time: NaiveDateTime,
    #[sql_type = "::diesel::sql_types::Timestamp"]
    pub created_time: NaiveDateTime,
    #[sql_type = "::diesel::sql_types::SmallInt"]
    pub msg_type: i16,      //1:文本消息,2:系统消息,3:图片，4：位置，5：表情
}

#[derive(Debug, Clone, Queryable,Serialize,Deserialize)]
pub struct FrontDisplayChatMessage {
    pub mid: i64,
    pub send_id: i64,
    pub to_id: i64,
    pub content: String,
    pub created_timestamp: i64,
    pub kind: i16,
    pub msg_type: i16,
}

#[derive(Debug, Clone, Queryable,Serialize,Deserialize)]
pub struct FrontDisplayKingdomChatMessage {
    pub mid: i64,
    pub send_user: FrontDisplayChatUser,
    pub to_id: i64,
    pub content: String,
    pub created_timestamp: i64,
    pub kind: i16,
    pub msg_type: i16,
}

#[derive(Debug, Clone, Queryable,Serialize,Deserialize)]
pub struct FrontDisplayGroupChatMessage {
    pub mid: i64,
    pub send_user: FrontDisplayChatUser,
    pub gid: i64,
    pub group_name: String,
    pub group_thumbnail: String,
    pub content: String,
    pub created_timestamp: i64,
    pub kind: i16,
    pub msg_type: i16,
}

#[derive(Debug, Clone, Queryable,Serialize,Deserialize)]
pub struct FrontDisplayP2pChatMessage {
    pub mid: i64,
    pub send_user: FrontDisplayChatUser,
    pub dst_user: FrontDisplayChatUser,
    pub content: String,
    pub created_timestamp: i64,
    pub kind: i16,
    pub msg_type: i16,
}

#[derive(Debug, Clone, Queryable,Serialize,Deserialize)]
pub struct FrontDisplayChatMessageUnreadCount {
    pub unread_count: i16,
    pub kind: i16,
}

#[derive(Debug, Clone, Queryable,Serialize,Deserialize)]
pub struct FrontDisplayP2pChatMessageCount {
    pub mid: i64,
    pub content: String,
    pub created_timestamp: i64,
    pub kind: i16,
    pub msg_type: i16,
}

#[derive(Debug, Insertable)]
#[table_name = "chat_messages"]
pub struct NewChatMessage {
    pub mid: i64,
    pub send_id: i64,
    pub to_id: i64,
    pub content: String,
    pub created_timestamp: i64,
    pub kind: i16,
    pub msg_type: i16,
}

impl ChatMessage {
    pub fn add(
        conn: &PgConnection,
        send_id: i64,
        to_id: i64,
        content: String,
        kind: i16,
        msg_type: i16,
    ) -> QueryResult<Self> {
        let data = NewChatMessage {
            mid: get_guid_value() as i64,
            send_id,
            to_id,
            content,
            created_timestamp: Utc::now().timestamp_millis(),
            kind,
            msg_type,
        };

         diesel::insert_into(chat_messages::table)
            .values(data)
            .get_result(conn)
    }

    pub fn get_kingdom_unread_count(
        conn: &PgConnection,
        kingdom_id: i64,
        t: i64,
    ) -> QueryResult<i64> {
        use diesel::dsl::count;
        let unread_count = chat_messages::table
            .filter(chat_messages::to_id.eq(kingdom_id))
            .filter(chat_messages::kind.eq(1))
            .filter(chat_messages::created_timestamp.gt(t))
            .select(count(chat_messages::mid))
            .first(conn)?;

        Ok(unread_count)
    }

    pub fn get_kingdom_unread_count_and_latest_message(
        conn: &PgConnection,
        to_id: i64,
        t: i64,
    ) -> Result<(i64, FrontDisplayKingdomChatMessage)> {
        let unread_count = Self::get_kingdom_unread_count(conn, to_id, t).with_context(|| format!("failed get kingdom unread count."))?;

        let latest_msg: ChatMessage = chat_messages::table
            .filter(chat_messages::to_id.eq(to_id))
            .filter(chat_messages::kind.eq(1))
            .filter(chat_messages::created_timestamp.gt(t))
            .order(chat_messages::created_timestamp.desc())
            .first(conn).with_context(|| format!("failed get kingdom latest message."))?;

        let send_user = User::get_front_display_chat_user_info(conn, latest_msg.send_id).with_context(|| format!("fialed get user info."))?;
        let kingdom_chat_message = FrontDisplayKingdomChatMessage {
            mid: latest_msg.mid,
            send_user,
            to_id: latest_msg.to_id,
            content: latest_msg.content,
            created_timestamp: latest_msg.created_timestamp,
            kind: latest_msg.kind,
            msg_type: latest_msg.msg_type,
        };

        Ok((unread_count, kingdom_chat_message))
    }

    pub fn get_group_unread_count(conn: &PgConnection, group_id: i64, t: i64) -> QueryResult<i64> {
        use diesel::dsl::count;
        let unread_count = chat_messages::table
            .filter(chat_messages::to_id.eq(group_id))
            .filter(chat_messages::kind.eq(2))
            .filter(chat_messages::created_timestamp.gt(t))
            .select(count(chat_messages::mid))
            .first(conn)?;

        Ok(unread_count)
    }
    pub fn get_group_unread_count_and_latest_message(
        conn: &PgConnection,
        to_id: i64,
        t: i64,
    ) -> QueryResult<(i64, FrontDisplayGroupChatMessage)> {
        let unread_count = Self::get_group_unread_count(conn, to_id, t)?;

        let latest_msg: ChatMessage = chat_messages::table
            .filter(chat_messages::to_id.eq(to_id))
            .filter(chat_messages::kind.eq(2))
            .filter(chat_messages::created_timestamp.gt(t))
            .order(chat_messages::created_timestamp.desc())
            .first(conn)?;

        let group_info = ChatGroup::get_chat_group_by_gid(conn, to_id)?;
        let send_user = User::get_front_display_chat_user_info(conn, latest_msg.send_id)?;
        let group_chat_message = FrontDisplayGroupChatMessage {
            mid: latest_msg.mid,
            send_user,
            gid: group_info.gid,
            group_name: group_info.group_name,
            group_thumbnail: group_info.group_thumbnail,
            content: latest_msg.content,
            created_timestamp: latest_msg.created_timestamp,
            kind: latest_msg.kind,
            msg_type: latest_msg.msg_type,
        };

        Ok((unread_count, group_chat_message))
    }

    pub fn get_p2p_unread_count_latest_message(
        conn: &PgConnection,
        to_id: i64,
        t: i64,
    ) -> QueryResult<FrontDisplayP2pChatMessageCount> {
        let latest_msg: ChatMessage = chat_messages::table
            .filter(chat_messages::to_id.eq(to_id))
            .filter(chat_messages::kind.eq(3))
            .filter(chat_messages::created_timestamp.gt(t))
            .order(chat_messages::created_timestamp.desc())
            .first(conn)?;

        let chat_message = FrontDisplayP2pChatMessageCount {
            mid: latest_msg.mid,
            content: latest_msg.content,
            created_timestamp: latest_msg.created_timestamp,
            kind: latest_msg.kind,
            msg_type: latest_msg.msg_type,
        };
        Ok(chat_message)
    }

    pub fn get_kingdom_message(
        conn: &PgConnection,
        to_id: i64,
        timestamp: i64,
        limit: i64,
        order: i16,
    ) -> Result<Vec<FrontDisplayKingdomChatMessage>> {
        let mut query = chat_messages::table
            .filter(chat_messages::to_id.eq(to_id))
            .limit(limit)
            .into_boxed();

        if timestamp > 0 {
            if order == 0 {
                query = query.filter(chat_messages::created_timestamp.gt(timestamp));
            } else if order == 1 {
                query = query.filter(chat_messages::created_timestamp.lt(timestamp));
            }
        }

        if order == 0 {
            query = query.order(chat_messages::created_timestamp.asc());
        } else if order == 1 {
            query = query.order(chat_messages::created_timestamp.desc());
        }

        let chat_msgs = query.load::<ChatMessage>(conn)?;

        let mut datas = Vec::new();

        for chat_msg in chat_msgs.into_iter() {
            //get send user info
            let send_user = match User::get_front_display_chat_user_info(conn, chat_msg.send_id).with_context(|| format!("failed to get user info")) {
                Ok(v) => v,
                Err(_e) => continue,
            };

            let f_chat_msg = FrontDisplayKingdomChatMessage {
                mid: chat_msg.mid,
                send_user,
                to_id: chat_msg.to_id,
                content: chat_msg.content,
                created_timestamp: chat_msg.created_timestamp,
                kind: chat_msg.kind,
                msg_type: chat_msg.msg_type,
            };

            datas.push(f_chat_msg);
        }

        Ok(datas)
    }

    pub fn get_group_message(
        conn: &PgConnection,
        to_id: i64,
        timestamp: i64,
        limit: i64,
        order: i16,
    ) -> QueryResult<Vec<FrontDisplayGroupChatMessage>> {
        let mut query = chat_messages::table
            .filter(chat_messages::to_id.eq(to_id))
            .limit(limit)
            .into_boxed();

        if timestamp > 0 {
            if order == 0 {
                query = query.filter(chat_messages::created_timestamp.gt(timestamp));
            } else if order == 1 {
                query = query.filter(chat_messages::created_timestamp.lt(timestamp));
            }
        }

        if order == 0 {
            query = query.order(chat_messages::created_timestamp.asc());
        } else if order == 1 {
            query = query.order(chat_messages::created_timestamp.desc());
        }

        let chat_msgs = query.load::<ChatMessage>(conn)?;

        let mut datas = Vec::new();

        for chat_msg in chat_msgs.into_iter() {
            //get send user info
            let send_user = User::get_front_display_chat_user_info(conn, chat_msg.send_id)?;
            let group_info = ChatGroup::get_chat_group_by_gid(conn, to_id)?;

            let f_chat_msg = FrontDisplayGroupChatMessage {
                mid: chat_msg.mid,
                send_user,
                gid: group_info.gid,
                group_name: group_info.group_name,
                group_thumbnail: group_info.group_thumbnail,
                content: chat_msg.content,
                created_timestamp: chat_msg.created_timestamp,
                kind: chat_msg.kind,
                msg_type: chat_msg.msg_type,
            };

            datas.push(f_chat_msg);
        }

        Ok(datas)
    }

    pub fn get_p2p_message(
        conn: &PgConnection,
        send_id: i64,
        to_id: i64,
        timestamp: i64,
        limit: i64,
        order: i16,
    ) -> QueryResult<Vec<FrontDisplayP2pChatMessage>> {
        let mut query = chat_messages::table
            .filter(chat_messages::send_id.eq_any(vec![send_id, to_id]))
            .filter(chat_messages::to_id.eq_any(vec![send_id, to_id]))
            .limit(limit)
            .into_boxed();

        if timestamp > 0 {
            if order == 0 {
                query = query.filter(chat_messages::created_timestamp.gt(timestamp));
            } else if order == 1 {
                query = query.filter(chat_messages::created_timestamp.lt(timestamp));
            }
        }

        if order == 0 {
            query = query.order(chat_messages::created_timestamp.asc());
        } else if order == 1 {
            query = query.order(chat_messages::created_timestamp.desc());
        }

        let chat_msgs = query.load::<ChatMessage>(conn)?;

        let mut datas = Vec::new();

        for chat_msg in chat_msgs.into_iter() {
            //get send user info
            let send_user = User::get_front_display_chat_user_info(conn, chat_msg.send_id)?;
            let dst_user = User::get_front_display_chat_user_info(conn, chat_msg.to_id)?;

            let f_chat_msg = FrontDisplayP2pChatMessage {
                mid: chat_msg.mid,
                send_user,
                dst_user,
                content: chat_msg.content,
                created_timestamp: chat_msg.created_timestamp,
                kind: chat_msg.kind,
                msg_type: chat_msg.msg_type,
            };

            datas.push(f_chat_msg);
        }

        Ok(datas)
    }
}

impl BinaryEncode for FrontDisplayChatMessage {
    fn encode(&self) -> Result<Vec<u8>> {
        let mut encoded = Vec::new();

        binary_write_i64(&mut encoded, self.mid)?;
        binary_write_i64(&mut encoded, self.send_id)?;
        binary_write_i64(&mut encoded, self.to_id)?;
        binary_write_string(&mut encoded, self.content.as_str())?;
        binary_write_i64(&mut encoded, self.created_timestamp)?;
        binary_write_i16(&mut encoded, self.kind)?;
        binary_write_i16(&mut encoded, self.msg_type)?;

        //set item length
        encoded.encode()
    }
}


impl<'a> BinaryDecode<'a> for FrontDisplayChatMessage {
    fn decode(
        cursor: &mut Cursor<&'a [u8]>,
        bytes: &'a [u8],
    ) -> Result<FrontDisplayChatMessage> {
        let mid = binary_read_i64(cursor)?;
        let send_id = binary_read_i64(cursor)?;
        let to_id = binary_read_i64(cursor)?;
        let content = binary_read_string(cursor, bytes)?;
        let created_timestamp = binary_read_i64(cursor)?;
        let kind = binary_read_i16(cursor)?;
        let msg_type = binary_read_i16(cursor)?;


        let data = FrontDisplayChatMessage {
            mid,
            send_id,
            to_id,
            content,
            created_timestamp,
            kind,
            msg_type,
        };

        Ok(data)
    }
}

impl BinaryEncode for FrontDisplayKingdomChatMessage {
    fn encode(&self) -> Result<Vec<u8>> {
        let mut encoded = Vec::new();

        binary_write_i64(&mut encoded, self.mid)?;
        let send_user = self.send_user.encode()?;
        encoded.extend(send_user);
        binary_write_i64(&mut encoded, self.to_id)?;
        binary_write_string(&mut encoded, self.content.as_str())?;
        binary_write_i64(&mut encoded, self.created_timestamp)?;
        binary_write_i16(&mut encoded, self.kind)?;
        binary_write_i16(&mut encoded, self.msg_type)?;

        //set item length
        encoded.encode()
    }
}

impl<'a> BinaryDecode<'a> for FrontDisplayKingdomChatMessage {
    fn decode(
        cursor: &mut Cursor<&'a [u8]>,
        bytes: &'a [u8],
    ) -> Result<FrontDisplayKingdomChatMessage> {
        let mid = binary_read_i64(cursor)?;

        let _user_item_length = binary_read_i16(cursor)?;
        let send_user: FrontDisplayChatUser = deserialize_binary(cursor, bytes)?;
        let to_id=binary_read_i64(cursor)?;
        let content = binary_read_string(cursor, bytes)?;
        let created_timestamp = binary_read_i64(cursor)?;
        let kind = binary_read_i16(cursor)?;
        let msg_type = binary_read_i16(cursor)?;


        let data = FrontDisplayKingdomChatMessage {
            mid,
            send_user,
            to_id,
            content,
            created_timestamp,
            kind,
            msg_type,
        };

        Ok(data)
    }
}

impl BinaryEncode for Option<FrontDisplayKingdomChatMessage> {
    fn encode(&self) -> Result<Vec<u8>> {
        let mut encoded = Vec::new();
        if self.is_some() {
            binary_write_i64(&mut encoded, self.as_ref().unwrap().mid)?;
            let send_user = self.as_ref().unwrap().send_user.encode()?;
            encoded.extend(send_user);
            binary_write_i64(&mut encoded, self.as_ref().unwrap().to_id)?;
            binary_write_string(&mut encoded, self.as_ref().unwrap().content.as_str())?;
            binary_write_i64(&mut encoded, self.as_ref().unwrap().created_timestamp)?;
            binary_write_i16(&mut encoded, self.as_ref().unwrap().kind)?;
            binary_write_i16(&mut encoded, self.as_ref().unwrap().msg_type)?;

        }
        //set item length
        encoded.encode()
    }
}

impl BinaryEncode for FrontDisplayGroupChatMessage {
    fn encode(&self) -> Result<Vec<u8>> {
        let mut encoded = Vec::new();

        binary_write_i64(&mut encoded, self.mid)?;
        let send_user = self.send_user.encode()?;
        encoded.extend(send_user);

        binary_write_i64(&mut encoded, self.gid)?;
        binary_write_string(&mut encoded, self.group_name.as_str())?;
        binary_write_string(&mut encoded, self.group_thumbnail.as_str())?;
        binary_write_string(&mut encoded, self.content.as_str())?;
        binary_write_i64(&mut encoded, self.created_timestamp)?;
        binary_write_i16(&mut encoded, self.kind)?;
        binary_write_i16(&mut encoded, self.msg_type)?;

        //set item length
        encoded.encode()
    }
}


impl<'a> BinaryDecode<'a> for FrontDisplayGroupChatMessage {
    fn decode(
        cursor: &mut Cursor<&'a [u8]>,
        bytes: &'a [u8],
    ) -> Result<FrontDisplayGroupChatMessage> {
        let mid = binary_read_i64(cursor)?;
        let _user_item_length = binary_read_i16(cursor)?;
        let send_user: FrontDisplayChatUser = deserialize_binary(cursor, bytes)?;
        let gid=binary_read_i64(cursor)?;
        let group_name = binary_read_string(cursor, bytes)?;
        let group_thumbnail = binary_read_string(cursor, bytes)?;
        let content = binary_read_string(cursor, bytes)?;
        let created_timestamp = binary_read_i64(cursor)?;
        let kind = binary_read_i16(cursor)?;
        let msg_type = binary_read_i16(cursor)?;


        let data = FrontDisplayGroupChatMessage {
            mid,
            send_user,
            gid,
            group_name,
            group_thumbnail,
            content,
            created_timestamp,
            kind,
            msg_type,
        };

        Ok(data)
    }
}

impl BinaryEncode for FrontDisplayP2pChatMessage {
    fn encode(&self) -> Result<Vec<u8>> {
        let mut encoded = Vec::new();

        binary_write_i64(&mut encoded, self.mid)?;
        let send_user = self.send_user.encode()?;
        encoded.extend(send_user);
        let dst_user = self.dst_user.encode()?;
        encoded.extend(dst_user);
        binary_write_string(&mut encoded, self.content.as_str())?;
        binary_write_i64(&mut encoded, self.created_timestamp)?;
        binary_write_i16(&mut encoded, self.kind)?;
        binary_write_i16(&mut encoded, self.msg_type)?;

        //set item length
        encoded.encode()
    }
}

impl<'a> BinaryDecode<'a> for FrontDisplayP2pChatMessage {
    fn decode(
        cursor: &mut Cursor<&'a [u8]>,
        bytes: &'a [u8],
    ) -> Result<FrontDisplayP2pChatMessage> {
        let mid = binary_read_i64(cursor)?;
        let _user_item_length = binary_read_i16(cursor)?;
        let send_user: FrontDisplayChatUser = deserialize_binary(cursor, bytes)?;
        let _user_item_length = binary_read_i16(cursor)?;
        let dst_user: FrontDisplayChatUser = deserialize_binary(cursor, bytes)?;
        let content = binary_read_string(cursor, bytes)?;
        let created_timestamp = binary_read_i64(cursor)?;
        let kind = binary_read_i16(cursor)?;
        let msg_type = binary_read_i16(cursor)?;


        let data = FrontDisplayP2pChatMessage {
            mid,
            send_user,
            dst_user,
            content,
            created_timestamp,
            kind,
            msg_type,
        };

        Ok(data)
    }
}

impl BinaryEncode for FrontDisplayP2pChatMessageCount {
    fn encode(&self) -> Result<Vec<u8>> {
        let mut encoded = Vec::new();
        encoded.write_i64::<LittleEndian>(self.mid)?;
        let content = self.content.as_bytes();
        encoded.write_i16::<LittleEndian>(content.len() as i16)?;
        encoded.extend_from_slice(content);
        encoded.write_i64::<LittleEndian>(self.created_timestamp)?;
        encoded.write_i16::<LittleEndian>(self.kind)?;
        encoded.write_i16::<LittleEndian>(self.msg_type)?;

        //set item length
        encoded.encode()
    }
}
impl<'a> BinaryDecode<'a> for FrontDisplayP2pChatMessageCount {
    fn decode(
        cursor: &mut Cursor<&'a [u8]>,
        bytes: &'a [u8],
    ) -> Result<FrontDisplayP2pChatMessageCount> {
        let mid = binary_read_i64(cursor)?;
        let content = binary_read_string(cursor, bytes)?;
        let created_timestamp = binary_read_i64(cursor)?;
        let kind = binary_read_i16(cursor)?;
        let msg_type = binary_read_i16(cursor)?;


        let data = FrontDisplayP2pChatMessageCount {
            mid,
            content,
            created_timestamp,
            kind,
            msg_type,
        };

        Ok(data)
    }
}

impl BinaryEncode for FrontDisplayChatMessageUnreadCount {
    fn encode(&self) -> Result<Vec<u8>> {
        let mut encoded = Vec::new();

        binary_write_i16(&mut encoded, self.unread_count)?;
        binary_write_i16(&mut encoded, self.kind)?;

        //set item length
        encoded.encode()
    }
}


impl<'a> BinaryDecode<'a> for FrontDisplayChatMessageUnreadCount {
    fn decode(
        cursor: &mut Cursor<&'a [u8]>,
        _bytes: &'a [u8],
    ) -> Result<FrontDisplayChatMessageUnreadCount> {
        let unread_count = binary_read_i16(cursor)?;
        let kind = binary_read_i16(cursor)?;

        let data = FrontDisplayChatMessageUnreadCount {
            unread_count,
            kind
        };

        Ok(data)
    }
}
