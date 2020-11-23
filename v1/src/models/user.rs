use crate::schema::{servers, users};
use anyhow::Result;
use byteorder::{LittleEndian, WriteBytesExt};
use chrono::NaiveDateTime;
use diesel::prelude::*;
use crate::{BinaryEncode, BinaryDecode, utils::binary_helper::*};
use std::io::Cursor;
use serde::{Serialize,Deserialize};


#[derive(Clone, Debug, Identifiable, Queryable, Associations)]
#[primary_key(uuid)]
pub struct User {
    pub uuid: i64,
    pub uid: i32,
    pub name: String,
    pub avatar: String,
    pub login_days: i32,
    pub server_id: i32,
    pub modify_time: NaiveDateTime,
    pub created_time: NaiveDateTime,
    pub action_points: i32,
    pub max_action_points: i32,
    pub action_points_latest_timestamp: i64,
}

#[derive(Debug, Default, Insertable)]
#[table_name = "users"]
pub struct NewUser<'a> {
    pub uuid: i64,
    pub uid: i32,
    pub name: &'a str,
    pub avatar: &'a str,
    pub login_days: i32,
    pub server_id: i32,
    pub action_points: i32,
    pub max_action_points: i32,
    pub action_points_latest_timestamp: i64,
}

#[derive(Clone, Debug, Queryable,Serialize,Deserialize)]
pub struct FrontDisplayChatUser {
    pub uuid: i64,
    pub uid: i32,
    pub name: String,
    pub avatar: String,
    pub server_id: i32,
    pub action_points: i32,
}


impl User {
    pub fn get_kingdom_id(conn: &PgConnection, uuid: i64) -> QueryResult<i64> {
        users::table
            .filter(users::uuid.eq(uuid))
            .inner_join(servers::table.on(users::server_id.eq(servers::server_number)))
            .select(servers::sid)
            .first(conn)
    }

    pub fn get_front_display_chat_user_info(
        conn: &PgConnection,
        uuid: i64,
    ) -> QueryResult<FrontDisplayChatUser> {
        users::table
            .filter(users::uuid.eq(uuid))
            .select((
                users::uuid,
                users::uid,
                users::name,
                users::avatar,
                users::server_id,
                users::action_points,
            ))
            .first(conn)
    }
}

impl BinaryEncode for FrontDisplayChatUser {
    fn encode(&self) -> Result<Vec<u8>> {
        let mut encoded = Vec::new();
        encoded.write_i64::<LittleEndian>(self.uuid)?;
        encoded.write_i32::<LittleEndian>(self.uid)?;

        let name = self.name.as_bytes();
        encoded.write_i16::<LittleEndian>(name.len() as i16)?;
        encoded.extend_from_slice(name);

        let avatar = self.avatar.as_bytes();
        encoded.write_i16::<LittleEndian>(avatar.len() as i16)?;
        encoded.extend_from_slice(avatar);
        encoded.write_i32::<LittleEndian>(self.server_id)?;
        encoded.write_i32::<LittleEndian>(self.action_points)?;

        //set item length
        encoded.encode()
    }
}

impl<'a> BinaryDecode<'a> for FrontDisplayChatUser {
    fn decode(cursor: &mut Cursor<&'a [u8]>, bytes: &'a [u8]) -> Result<FrontDisplayChatUser> {
        let uuid = binary_read_i64(cursor)?;
        let uid = binary_read_i32(cursor)?;
        let name = binary_read_string(cursor, bytes)?;
        let avatar = binary_read_string(cursor, bytes)?;
        let server_id = binary_read_i32(cursor)?;
        let action_points = binary_read_i32(cursor)?;

        let data = FrontDisplayChatUser {
            uuid,
            uid,
            name,
            avatar,
            server_id,
            action_points,
        };
        Ok(data)
    }
}