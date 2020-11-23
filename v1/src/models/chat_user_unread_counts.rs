use crate::schema::chat_user_unread_counts;
use crate::{
    get_guid_value, models::chat_messages::ChatMessage,
    models::chat_messages::FrontDisplayP2pChatMessageCount, models::user::FrontDisplayChatUser,
    models::user::User,
    BinaryEncode, BinaryDecode, deserialize_binary, utils::binary_helper::*
};
use anyhow::Result;
use byteorder::{LittleEndian, WriteBytesExt};
use chrono::NaiveDateTime;
use chrono::Utc;
use diesel::prelude::*;
use std::io::Cursor;
use serde::{Serialize,Deserialize};

#[derive(Debug, Clone, Identifiable, Queryable, Associations)]
#[primary_key(ucid)]
pub struct ChatUserUnreadCount {
    pub ucid: i64,
    pub uuid_s: i64, //my message unread count's user id
    pub uuid_d: i64, //sender message user uuid
    pub latest_timestamp: i64,
    pub unread_count: i16,
    pub modify_time: NaiveDateTime,
    pub created_time: NaiveDateTime,
}

#[derive(Debug, Clone, Queryable,Serialize,Deserialize)]
pub struct FrontDisplayChatUserUnreadCount {
    pub sender: FrontDisplayChatUser,
    pub receiver: FrontDisplayChatUser,
    pub latest_timestamp: i64,
    pub unread_count: i16,
    pub latest_msg: FrontDisplayP2pChatMessageCount,
}

#[derive(Debug, Insertable)]
#[table_name = "chat_user_unread_counts"]
pub struct NewChatUserUnreadCount {
    pub ucid: i64,
    pub uuid_s: i64,
    pub uuid_d: i64,
    pub latest_timestamp: i64,
    pub unread_count: i16,
}

impl ChatUserUnreadCount {
    pub fn add(
        conn: &PgConnection,
        uuid_s: i64,
        uuid_d: i64,
        unread_count: i16,
    ) -> QueryResult<()> {
        use diesel::dsl::exists;

        let f = chat_user_unread_counts::table
            .filter(chat_user_unread_counts::uuid_s.eq(uuid_s))
            .filter(chat_user_unread_counts::uuid_d.eq(uuid_d));

        let mut add = false;

        if let Ok(exists) = diesel::select(exists(f)).get_result(conn) {
            if exists {
                let now = Utc::now();
                diesel::update(chat_user_unread_counts::table)
                    .set((
                        // chat_user_unread_counts::latest_timestamp.eq(now.timestamp()),
                        chat_user_unread_counts::unread_count
                            .eq(chat_user_unread_counts::unread_count + 1),
                        chat_user_unread_counts::modify_time.eq(now.naive_local()),
                    ))
                    .filter(chat_user_unread_counts::uuid_s.eq(uuid_s))
                    .filter(chat_user_unread_counts::uuid_d.eq(uuid_d))
                    .execute(conn)?;
            } else {
                add = true;
            }
        } else {
            add = true;
        }

        if add {
            let data = NewChatUserUnreadCount {
                ucid: get_guid_value() as i64,
                uuid_s,
                uuid_d,
                latest_timestamp: 0,
                unread_count,
            };

            let _ = diesel::insert_into(chat_user_unread_counts::table)
                .values(data)
                .execute(conn)?;
        }

        Ok(())
    }

    pub fn get_user_unread_message_count(
        conn: &PgConnection,
        uid_s: i64,
    ) -> QueryResult<Vec<FrontDisplayChatUserUnreadCount>> {
        let p2p_unreads = chat_user_unread_counts::table
            .filter(chat_user_unread_counts::uuid_s.eq(uid_s))
            .filter(chat_user_unread_counts::unread_count.gt(0))
            .get_results::<ChatUserUnreadCount>(conn)?;

        let mut res = Vec::new();

        for p2p_unread in p2p_unreads.into_iter() {
            let msg = ChatMessage::get_p2p_unread_count_latest_message(
                conn,
                p2p_unread.uuid_s,
                p2p_unread.latest_timestamp,
            )?;

            let sender = User::get_front_display_chat_user_info(conn, p2p_unread.uuid_d)?;
            let receiver = User::get_front_display_chat_user_info(conn, p2p_unread.uuid_s)?;

            res.push(FrontDisplayChatUserUnreadCount {
                sender,
                receiver,
                latest_timestamp: p2p_unread.latest_timestamp,
                unread_count: p2p_unread.unread_count,
                latest_msg: msg,
            });
        }

        Ok(res)
    }

    pub fn get_user_unread_count(
        conn: &PgConnection,
        my_uuid: i64,
        send_uid: i64,
    ) -> QueryResult<i16> {
        chat_user_unread_counts::table
            .filter(chat_user_unread_counts::uuid_s.eq(my_uuid))
            .filter(chat_user_unread_counts::uuid_d.eq(send_uid))
            .select(chat_user_unread_counts::unread_count)
            .get_result(conn)
    }

    pub fn update_user_unread_count(
        conn: &PgConnection,
        uid_s: i64,
        uid_d: i64,
    ) -> QueryResult<()> {
        let now = Utc::now();
        diesel::update(chat_user_unread_counts::table)
            .set((
                chat_user_unread_counts::latest_timestamp.eq(now.timestamp_millis()),
                chat_user_unread_counts::unread_count.eq(0),
                chat_user_unread_counts::modify_time.eq(now.naive_local()),
            ))
            .filter(chat_user_unread_counts::uuid_s.eq(uid_s))
            .filter(chat_user_unread_counts::uuid_d.eq(uid_d))
            .execute(conn)?;

        Ok(())
    }
}

impl BinaryEncode for FrontDisplayChatUserUnreadCount {
    fn encode(&self) -> Result<Vec<u8>> {
        let mut encoded = Vec::new();

        let sender = self.sender.encode()?;
        encoded.extend(sender);
        let receiver = self.receiver.encode()?;
        encoded.extend(receiver);

        encoded.write_i64::<LittleEndian>(self.latest_timestamp)?;
        encoded.write_i16::<LittleEndian>(self.unread_count)?;
        let msg = self.latest_msg.encode()?;
        encoded.extend(msg);

        //set item length
        encoded.encode()
    }
}


impl<'a> BinaryDecode<'a> for FrontDisplayChatUserUnreadCount {
    fn decode(
        cursor: &mut Cursor<&'a [u8]>,
        bytes: &'a [u8],
    ) -> Result<FrontDisplayChatUserUnreadCount> {
        let _user_item_length = binary_read_i16(cursor)?;
        let sender: FrontDisplayChatUser = deserialize_binary(cursor, bytes)?;
        let _user_item_length = binary_read_i16(cursor)?;
        let receiver: FrontDisplayChatUser = deserialize_binary(cursor, bytes)?;
        let latest_timestamp = binary_read_i64(cursor)?;
        let unread_count = binary_read_i16(cursor)?;
        let _user_item_length = binary_read_i16(cursor)?;
        let latest_msg: FrontDisplayP2pChatMessageCount = deserialize_binary(cursor, bytes)?;


        let data = FrontDisplayChatUserUnreadCount {
            sender,
            receiver,
            latest_timestamp,
            unread_count,
            latest_msg
        };

        Ok(data)
    }
}