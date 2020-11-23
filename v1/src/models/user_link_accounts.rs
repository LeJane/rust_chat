use crate::schema::user_link_accounts;
use chrono::NaiveDateTime;

#[derive(Debug, Identifiable, Queryable, Clone)]
#[primary_key(lid)]
pub struct UserLinkAccount {
    pub lid: u64,
    pub uuid: u64,
    pub account_type: u16,
    pub modify_time: NaiveDateTime,
    pub created_time: NaiveDateTime,
}

#[derive(Insertable, Debug, Default)]
#[table_name = "user_link_accounts"]
pub struct NewUserLinkAccount {
    pub lid: i64,
    pub uuid: i64,
    pub account_type: i16,
}
