use crate::schema::chat_groups;
use chrono::NaiveDateTime;
use diesel::prelude::*;

#[derive(Debug, Clone, Identifiable, Queryable, Associations)]
#[primary_key(gid)]
pub struct ChatGroup {
    pub gid: i64,
    pub group_name: String,
    pub group_thumbnail: String,
    pub uuid: i64,
    pub person_count: i16,
    pub modify_time: NaiveDateTime,
    pub created_time: NaiveDateTime,
}

#[derive(Debug, Default, Insertable)]
#[table_name = "chat_groups"]
pub struct NewChatGroup<'a> {
    pub gid: i64,
    pub group_name: &'a str,
    pub group_thumbnail: &'a str,
    pub uuid: i64,
    pub person_count: i16,
}

impl ChatGroup {
    pub fn get_chat_group_by_gid(conn: &PgConnection, gid: i64) -> QueryResult<ChatGroup> {
        chat_groups::table
            .filter(chat_groups::gid.eq(gid))
            .first(conn)
    }
}
