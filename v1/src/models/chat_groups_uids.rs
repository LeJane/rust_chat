use crate::schema::{chat_groups_uids, users};
use chrono::NaiveDateTime;
use chrono::Utc;
use diesel::prelude::*;

#[derive(Debug, Clone, Identifiable, Queryable, Associations)]
#[primary_key(guid)]
pub struct ChatGroupsUid {
    pub guid: i64,
    pub gid: i64,
    pub uuid: i64,
    pub latest_timestamp: i64,
    pub unread_count: i16,
    pub modify_time: NaiveDateTime,
    pub created_time: NaiveDateTime,
}

#[derive(Debug, Default, Insertable)]
#[table_name = "chat_groups_uids"]
pub struct NewChatGroupUid {
    pub guid: i64,
    pub gid: i64,
    pub uuid: i64,
    pub latest_timestamp: i64,
    pub unread_count: i16,
}

impl ChatGroupsUid {
    pub fn get_groups_users_by_gid(
        conn: &PgConnection,
        gid: i64,
    ) -> QueryResult<Vec<(i64, i64, i32)>> {
        chat_groups_uids::table
            .filter(chat_groups_uids::gid.eq(gid))
            .inner_join(users::table.on(users::uuid.eq(chat_groups_uids::uuid)))
            .select((
                chat_groups_uids::gid,
                chat_groups_uids::uuid,
                users::server_id,
            ))
            .load(conn)
    }

    pub fn get_groups_info_by_gid(conn: &PgConnection, gid: i64) -> QueryResult<Self> {
        chat_groups_uids::table
            .filter(chat_groups_uids::gid.eq(gid))
            .get_result(conn)
    }
    pub fn get_groups_users_by_uid(
        conn: &PgConnection,
        uuid: i64,
    ) -> QueryResult<Vec<ChatGroupsUid>> {
        chat_groups_uids::table
            .filter(chat_groups_uids::uuid.eq(uuid))
            .load(conn)
    }

    pub fn update_unread_count(conn: &PgConnection, gid: i64, uid: i64) -> QueryResult<()> {
        diesel::update(chat_groups_uids::table)
            .set(chat_groups_uids::unread_count.eq(chat_groups_uids::unread_count + 1))
            .filter(chat_groups_uids::gid.eq(gid))
            .filter(chat_groups_uids::uuid.ne(uid))
            .execute(conn)?;

        Ok(())
    }
    pub fn update_unread_count_and_timestamp(
        conn: &PgConnection,
        gid: i64,
        timestamp: i64,
        uid:i64,
    ) -> QueryResult<()> {
        let now = Utc::now();
        diesel::update(chat_groups_uids::table)
            .set((
                chat_groups_uids::unread_count.eq(0),
                chat_groups_uids::latest_timestamp.eq(timestamp),
                chat_groups_uids::modify_time.eq(now.naive_local()),
            ))
            .filter(chat_groups_uids::gid.eq(gid))
            .filter(chat_groups_uids::uuid.eq(uid))
            .execute(conn)?;

        Ok(())
    }

    pub fn get_gids_by_uid(conn: &PgConnection, uuid: i64) -> QueryResult<Vec<ChatGroupsUid>> {
        chat_groups_uids::table
            .filter(chat_groups_uids::uuid.eq(uuid))
            .filter(chat_groups_uids::unread_count.gt(0))
            .load(conn)
    }

    pub fn get_group_user_info(
        conn: &PgConnection,
        gid: i64,
        uuid: i64,
    ) -> QueryResult<ChatGroupsUid> {
        chat_groups_uids::table
            .filter(chat_groups_uids::gid.eq(gid))
            .filter(chat_groups_uids::uuid.eq(uuid))
            .first(conn)
    }
}
