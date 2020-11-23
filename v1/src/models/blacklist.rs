use crate::schema::blacklists;
use chrono::NaiveDateTime;
use diesel::prelude::*;

#[derive(Debug, Clone, Identifiable, Queryable)]
#[primary_key(bid)]
pub struct Blacklist {
    pub bid: u64,
    pub uuid_a: u64,
    pub uuid_b: u64,
    pub modify_time: NaiveDateTime,
    pub created_time: NaiveDateTime,
}

#[derive(Debug, Default, Insertable)]
#[table_name = "blacklists"]
pub struct NewBlacklist {
    pub bid: i64,
    pub uuid_a: i64,
    pub uuid_b: i64,
}

impl Blacklist{
    pub fn find_user_black_list_exists(
        conn: &PgConnection,
        uid: i64,
        black_uid: i64,
    ) -> QueryResult<bool> {
        use diesel::dsl::exists;

        diesel::select(exists(
            blacklists::table
                .filter(blacklists::uuid_a.eq(uid))
                .filter(blacklists::uuid_b.eq(black_uid)),
        ))
            .get_result(conn)
    }
}