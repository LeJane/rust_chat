use crate::diesel::Connection;
use crate::models::{
    chat_groups_uids::ChatGroupsUid, chat_messages::ChatMessage,
    chat_messages::FrontDisplayChatMessageUnreadCount, servers::Server, user::User,
    chat_user_unread_counts::ChatUserUnreadCount,
    blacklist::Blacklist,
    chat_messages::FrontDisplayP2pChatMessageCount,
};
use crate::ResponseResult;
use crate::{
    get_connection, ChatMessageUnReadCount, Clients, Connection as LocalConn, GroupUnReadCountMsg,
    KingdomUnReadCountMsg, MessageStateCode, ONLINE_USERS_SETS_REDIS_KEY,
};
use anyhow::{anyhow, Error};
use byteorder::{LittleEndian, ReadBytesExt};
use redis::Commands;
use tracing::{error, info};
use crate::default_log_pre;
use function_name::named;

#[named]
pub async fn connection_state(clients: Clients, conn: LocalConn) -> ResponseResult {
    let redis_conn = get_connection()?;
    let mut cursor = std::io::Cursor::new(conn.msg.body.clone());

    let uid = cursor.read_u64::<LittleEndian>();

    let uid = match uid {
        Ok(uid) => uid,
        Err(e) => {
            error!("{}\tinvalid uid param reason:{}.",
                   default_log_pre!(conn.msg.code as i16,""),
                   e
            );
            let m = "invaild user param.";
            return conn.get_general_error(m);
        }
    };

    info!(
        "{}\tsubmit content\tuuid:{}",
        default_log_pre!(conn.msg.code as i16,uid),
        uid,
    );

    //insert redis
    match redis_conn
        .lock()
        .await
        .sadd::<&str, u64, u64>(ONLINE_USERS_SETS_REDIS_KEY, uid)
    {
        Ok(_) => {}
        Err(e) => {
            error!(
                "{}\tfailed add uid to redis db: {}",
                default_log_pre!(conn.msg.code as i16,uid),
                e
            );
            let m = "failed set uid data.";
            return conn.get_general_error(m);
        }
    }

    let m = "success";
    let resp = conn.get_bin_code(MessageStateCode::Ok, m, "")?;

    clients.lock().await.insert(uid, conn);
    Ok(resp)
}

#[named]
pub async fn send_message(clients: Clients, conn: LocalConn) -> ResponseResult {
    let new_body = conn.msg.body.clone();
    //get message
    let mut cursor = std::io::Cursor::new(conn.msg.body.to_vec());
    let uid = cursor.read_u64::<LittleEndian>();
    let uid = match uid {
        Ok(uid) => uid,
        Err(e) => {
            error!(
                "{}\tinvalid uid param reason:{}.",
                default_log_pre!(conn.msg.code as i16,""),
                e
            );
            let m = "invaild uid param.";
            return conn.get_general_error(m);
        }
    };

    //chat channel type.
    let tid = cursor.read_u8();
    let tid = match tid {
        Ok(v) => v,
        Err(e) => {
            error!(
                "{}\tinvalid tid param reason:{}.",
                default_log_pre!(conn.msg.code as i16,uid),
                e
            );
            let m = "invaild tid param.";
            return conn.get_general_error(m);
        }
    };

    let dst_id = cursor.read_u64::<LittleEndian>();
    let dst_id = match dst_id {
        Ok(v) => v,
        Err(e) => {
            error!(
                "{}\tinvalid dst_id param reason:{}.",
                default_log_pre!(conn.msg.code as i16,uid),
                e
            );
            let m = "invaild dst_id param.";
            return conn.get_general_error(m);
        }
    };

    let msg_type = cursor.read_u16::<LittleEndian>();
    let msg_type = match msg_type {
        Ok(v) => v,
        Err(e) => {
            error!(
                "{}\tinvalid msg_type param reason:{}.",
                default_log_pre!(conn.msg.code as i16,uid),
                e
            );
            let m = "invaild msg type param.";
            return conn.get_general_error(m);
        }
    };
    let content_length = cursor.read_u16::<LittleEndian>();
    let _content_length = match content_length {
        Ok(v) => v,
        Err(e) => {
            error!(
                "{}\tinvalid dst_id param reason:{}.",
                default_log_pre!(conn.msg.code as i16,uid),
                e
            );
            let m = "invaild content length param.";
            return conn.get_general_error(m);
        }
    };

    //message
    let message = &new_body[cursor.position() as usize..];

    info!("{}\tsubmit content\tuid:{}\ttid:{}\tdst_id:{}\tmessage:{:?}", default_log_pre!(conn.msg.code as i16,uid), uid, &tid, &dst_id, std::str::from_utf8(message));

    match tid {
        1 => kingdom_chat(clients, &conn, tid, uid, dst_id, message, msg_type), //kd
        2 => group_chat(clients, &conn, tid, uid as i64, dst_id, message, msg_type), //group
        3 => p2p_chat(clients, &conn, tid, uid, dst_id, message, msg_type),     //p2p
        4 => Err(anyhow!("not finished.")),                           //allience
        _ => Err(anyhow!("invalid tid.")),
    }
}

#[named]
fn kingdom_chat(
    _clients: Clients,
    conn: &LocalConn,
    tid: u8,
    from_uid: u64,
    dst_id: u64,
    msg: &[u8],
    msg_type: u16,
) -> ResponseResult {
    let master_db_conn = conn.db_conn(true);
    let slave_db_conn = conn.db_conn(false);

    let decode_msg = std::str::from_utf8(msg);
    if let Err(e) = decode_msg {
        error!(
            "{}\tfailed parse message to utf8 reason:{}.",
            default_log_pre!(conn.msg.code as i16,from_uid),
            e
        );
        let m = "invalid utf8 encode.";
        return conn.get_general_error(m);
    }

    let kingdom_id = match Server::get_server_id(&slave_db_conn, dst_id as i32) {
        Ok(v) => v,
        Err(e) => {
            error!(
                "{}\tget kingdom id error:{:?}",
                default_log_pre!(conn.msg.code as i16,from_uid),
                &e
            );
            return conn.get_general_error(e.to_string().as_str());
        }
    };

    let msg_content = match ChatMessage::add(
        &master_db_conn,
        from_uid as i64,
        kingdom_id,
        decode_msg.unwrap().into(),
        tid as i16,
        msg_type as i16,
    ) {
        Ok(v) => v,

        Err(e) => {
            error!(
                "{}\tfailed add kingdom message content:{}.",
                default_log_pre!(conn.msg.code as i16,from_uid),
                e
            );
            let m = "server error.";
            return conn.get_general_error(m);
        }
    };

    let data = FrontDisplayP2pChatMessageCount {
        mid: msg_content.mid,
        content: msg_content.content,
        created_timestamp: msg_content.created_timestamp,
        kind: msg_content.kind,
        msg_type: msg_content.msg_type,
    };

    conn.get_bin_code(MessageStateCode::Ok, "success.", data)
}

#[named]
fn group_chat(
    _clients: Clients,
    conn: &LocalConn,
    tid: u8,
    from_uid: i64,
    dst_id: u64,
    msg: &[u8],
    msg_type: u16,
) -> ResponseResult {
    let master_db_conn = conn.db_conn(true);

    let decode_msg = std::str::from_utf8(msg);
    if let Err(e) = decode_msg {
        error!(
            "{}\tfailed parse message to utf8 reason:{}.",
            default_log_pre!(conn.msg.code as i16,from_uid),
            e
        );
        let m = "invalid utf8 encode.";
        return conn.get_general_error(m);
    }

    let msg_content = match master_db_conn.transaction::<ChatMessage, Error, _>(|| {
        let chat_message_content = ChatMessage::add(
            &master_db_conn,
            from_uid as i64,
            dst_id as i64,
            decode_msg.unwrap().into(),
            tid as i16,
            msg_type as i16,
        )?;

        //update user and group table unread count
        ChatGroupsUid::update_unread_count(&master_db_conn, dst_id as i64, from_uid as i64)?;

        Ok(chat_message_content)
    }) {
        Ok(v) => v,
        Err(e) => {
            error!(
                "{}\tfailed add user unread count reason:{}.",
                default_log_pre!(conn.msg.code as i16,from_uid),
                e
            );
            let m = "server error.";
            return conn.get_general_error(m);
        }
    };


    let data = FrontDisplayP2pChatMessageCount {
        mid: msg_content.mid,
        content: msg_content.content,
        created_timestamp: msg_content.created_timestamp,
        kind: msg_content.kind,
        msg_type: msg_content.msg_type,
    };

    conn.get_bin_code(MessageStateCode::Ok, "success.", data)
}

#[named]
fn p2p_chat(
    _clients: Clients,
    conn: &LocalConn,
    tid: u8,
    from_uid: u64,
    dst_uid: u64,
    msg: &[u8],
    msg_type: u16,
) -> ResponseResult {
    //add user unread count to pgsql
    //check is black list
    let master_db_conn = conn.db_conn(true);
    let slave_db_conn = conn.db_conn(false);

    if let Ok(exists) = Blacklist::find_user_black_list_exists(&slave_db_conn, dst_uid as i64,from_uid as i64 ) {
        if exists {
            let m = "you are blacklisted.";
            return conn.get_general_error(m);
        }
    }

    let decode_msg = std::str::from_utf8(msg);
    if let Err(e) = decode_msg {
        error!(
            "{}\tfailed parse message to utf8 reason:{}.",
            default_log_pre!(conn.msg.code as i16,from_uid),
            e
        );
        let m = "invalid utf8 encode.";
        return conn.get_general_error(m);
    }

    let msg_content = match master_db_conn.transaction::<ChatMessage, Error, _>(|| {
        let msg_content = ChatMessage::add(
            &master_db_conn,
            from_uid as i64,
            dst_uid as i64,
            decode_msg.unwrap().into(),
            tid as i16,
            msg_type as i16,
        )?;
        ChatUserUnreadCount::add(&master_db_conn, dst_uid as i64, from_uid as i64, 1)?;
        Ok(msg_content)
    }) {
        Ok(v) => v,
        Err(e) => {
            error!(
                "{}\tfailed add user unread count reason:{}.",
                default_log_pre!(conn.msg.code as i16,from_uid),
                e
            );
            let m = "server error.";
            return conn.get_general_error(m);
        }
    };

    let data = FrontDisplayP2pChatMessageCount {
        mid: msg_content.mid,
        content: msg_content.content,
        created_timestamp: msg_content.created_timestamp,
        kind: msg_content.kind,
        msg_type: msg_content.msg_type,
    };

    conn.get_bin_code(MessageStateCode::Ok, "success.", data)
}

pub async fn client_feedback_server_message(_clients: Clients, _conn: LocalConn) -> ResponseResult {
    Err(anyhow!("not uknown."))
}

//pull get user message unread count
#[named]
pub async fn get_user_message_unread_count(_clients: Clients, conn: LocalConn) -> ResponseResult {
    let db_conn = conn.db_conn(false);
    let mut cursor = std::io::Cursor::new(&conn.msg.body);

    let kingdom_read_timestamp = cursor.read_i64::<LittleEndian>().unwrap_or(0);
    let uid = cursor.read_i64::<LittleEndian>();
    let uid = match uid {
        Ok(uid) => uid,
        Err(e) => {
            error!(
                "{}\tinvalid uid param reason:{}.",
                default_log_pre!(conn.msg.code as i16,""),
                e
            );
            let m = "invaild user param.";
            return conn.get_general_error(m);
        }
    };

    info!("{}\tsubmit content\tkingdom_read_timestamp:{}\tuuid:{}", default_log_pre!(conn.msg.code as i16,uid), kingdom_read_timestamp, uid);

    let kingdom_id = match User::get_kingdom_id(&db_conn, uid) {
        Ok(v) => v,
        Err(e) => {
            error!(
                "{}\tget kingdom id error:{:?}",
                default_log_pre!(conn.msg.code as i16,uid),
                &e
            );
            return conn.get_general_error(e.to_string().as_str());
        }
    };

    let (kingdom_unread_count, kingdom_msg) =
        match ChatMessage::get_kingdom_unread_count_and_latest_message(
            &db_conn,
            kingdom_id,
            kingdom_read_timestamp,
        ) {
            Ok(v) => (v.0, Some(v.1)),
            Err(e) => {
                error!("{}\tget kingdom unread count and latest message error:{:?}", default_log_pre!(conn.msg.code as i16,uid), &e);
                (0, None)
            }
        };
    //find unread count >0 for group
    let gids: Vec<ChatGroupsUid> = match ChatGroupsUid::get_gids_by_uid(&db_conn, uid) {
        Ok(v) => v,
        Err(e) => {
            error!("{}\tget group chat id related uuid list error:{:?}", default_log_pre!(conn.msg.code as i16,uid), &e);
            vec![]
        }
    };

    let mut groups = Vec::new();

    for gid in gids.into_iter() {
        let (group_unread_count, group_msg) =
            match ChatMessage::get_group_unread_count_and_latest_message(
                &db_conn,
                gid.gid,
                gid.latest_timestamp,
            ) {
                Ok(v) => v,
                Err(e) => {
                    error!("{}\tget group chat unread and latest message list error:{:?}", default_log_pre!(conn.msg.code as i16,uid), &e);
                    continue;
                }
            };
        groups.push(GroupUnReadCountMsg {
            unread_count: group_unread_count as i32,
            latest_message: group_msg,
        });
    }

    //p2p
    let p2p_msg = match ChatUserUnreadCount::get_user_unread_message_count(&db_conn, uid) {
        Ok(v) => v,
        Err(e) => {
            error!(
                "{}\tget user unread message count error:{:?}",
                default_log_pre!(conn.msg.code as i16,uid),
                &e
            );
            vec![]
        }
    };

    let res_data = ChatMessageUnReadCount {
        kingdom: KingdomUnReadCountMsg {
            unread_count: kingdom_unread_count as i32,
            latest_message: kingdom_msg,
        },
        groups,
        p2ps: p2p_msg,
    };

    conn.get_bin_code(MessageStateCode::Ok, "success.", res_data)
}

#[named]
pub async fn get_kingdom_message_content(_clients: Clients, conn: LocalConn) -> ResponseResult {
    let db_conn = conn.db_conn(false);
    let mut cursor = std::io::Cursor::new(&conn.msg.body);

    let timestamp = cursor.read_i64::<LittleEndian>().unwrap_or(0);
    let mut limit = cursor.read_i16::<LittleEndian>().unwrap_or(10);

    if limit > 50 {
        limit = 50;
    }

    //0:asc,1:desc
    let order = cursor.read_i16::<LittleEndian>().unwrap_or(1);

    let uid = cursor.read_i64::<LittleEndian>();
    let uid = match uid {
        Ok(uid) => uid,
        Err(e) => {
            error!(
                "{}\tinvalid uid param reason:{}.",
                default_log_pre!(conn.msg.code as i16,""),
                e
            );
            let m = "invaild user param.";
            return conn.get_general_error(m);
        }
    };

    info!("{}\tsubmit content\ttimestamp:{}\tlimit:{}\torder:{}\tuuid:{}", default_log_pre!(conn.msg.code as i16,uid), timestamp, limit, order, uid);

    let kingdom_id = match User::get_kingdom_id(&db_conn, uid) {
        Ok(v) => v,
        Err(e) => {
            error!(
                "{}\tget kingdom id error:{}.",
                default_log_pre!(conn.msg.code as i16,uid),
                e
            );
            return conn.get_general_error(e.to_string().as_str());
        }
    };

    let res_data = match ChatMessage::get_kingdom_message(
        &db_conn,
        kingdom_id,
        timestamp,
        limit as i64,
        order,
    ) {
        Ok(v) => v,
        Err(e) => {
            error!(
                "{}\tget kingdom message content error:{}.",
                default_log_pre!(conn.msg.code as i16,uid),
                e
            );
            return conn.get_general_error(e.to_string().as_str());
        }
    };

    conn.get_bin_code(MessageStateCode::Ok, "success.", res_data)
}

#[named]
pub async fn get_group_message_content(_clients: Clients, conn: LocalConn) -> ResponseResult {
    let master_db_conn = conn.db_conn(true);
    let slave_db_conn = conn.db_conn(false);
    let mut cursor = std::io::Cursor::new(&conn.msg.body);

    let timestamp = cursor.read_i64::<LittleEndian>().unwrap_or(0);
    let mut limit = cursor.read_i16::<LittleEndian>().unwrap_or(10);

    if limit > 50 {
        limit = 50;
    }

    //0:asc,1:desc
    let order = cursor.read_i16::<LittleEndian>().unwrap_or(1);

    let gid = cursor.read_i64::<LittleEndian>().unwrap_or(0);

    if gid <= 0 {
        error!(
            "{}\tinvalid gid param.",
            default_log_pre!(conn.msg.code as i16,""),
        );
        let m = "invaild gid param.";
        return conn.get_general_error(m);
    }

    let uid = cursor.read_i64::<LittleEndian>().unwrap_or(0);

    if uid <= 0 {
        error!(
            "{}\tinvalid uid param.",
            default_log_pre!(conn.msg.code as i16,""),
        );
        let m = "invaild gid param.";
        return conn.get_general_error(m);
    }

    info!("{}\tsubmit content\tuid:{}\ttimestamp:{}\tlimit:{}\torder:{}\tgid:{}", default_log_pre!(conn.msg.code as i16,uid), uid, timestamp, limit, order, gid);

    let res_data =
        match ChatMessage::get_group_message(&slave_db_conn, gid, timestamp, limit as i64, order) {
            Ok(v) => v,
            Err(e) => return conn.get_general_error(e.to_string().as_str()),
        };

    //update group
    if let Err(e) = ChatGroupsUid::update_unread_count_and_timestamp(&master_db_conn, gid, timestamp, uid)
    {
        error!("{}\tfailed update user group unread count and latest timestamp:{:?}",
               default_log_pre!(conn.msg.code as i16,uid), e
        );
    };

    conn.get_bin_code(MessageStateCode::Ok, "success", res_data)
}

#[named]
pub async fn get_p2p_user_message_content(_clients: Clients, conn: LocalConn) -> ResponseResult {
    let master_db_conn = conn.db_conn(true);
    let slave_db_conn = conn.db_conn(false);
    let mut cursor = std::io::Cursor::new(&conn.msg.body);

    let timestamp = cursor.read_i64::<LittleEndian>().unwrap_or(0);
    let mut limit = cursor.read_i16::<LittleEndian>().unwrap_or(10);

    if limit > 50 {
        limit = 50;
    }

    //0:asc,1:desc
    let order = cursor.read_i16::<LittleEndian>().unwrap_or(1);

    let send_uid = cursor.read_i64::<LittleEndian>();
    let send_uid = match send_uid {
        Ok(uid) => uid,
        Err(_e) => {
            error!(
                "{}\tinvalid send id param.",
                default_log_pre!(conn.msg.code as i16,""),
            );
            let m = "invaild send id param.";
            return conn.get_general_error(m);
        }
    };
    let my_uid = cursor.read_i64::<LittleEndian>();
    let my_uid = match my_uid {
        Ok(uid) => uid,
        Err(_e) => {
            error!(
                "{}\tinvalid self uuid param.",
                default_log_pre!(conn.msg.code as i16,""),
            );
            let m = "invaild uid param.";
            return conn.get_general_error(m);
        }
    };

    info!("{}\tsubmit content\ttimestamp:{}\tlimit:{}\torder:{}\tsend_uid:{}\tmy_uid:{}", default_log_pre!(conn.msg.code as i16,my_uid), timestamp, limit, order, send_uid, my_uid);

    let res_data = match ChatMessage::get_p2p_message(
        &slave_db_conn,
        send_uid,
        my_uid,
        timestamp,
        limit as i64,
        order,
    ) {
        Ok(v) => v,
        Err(e) => return conn.get_general_error(e.to_string().as_str()),
    };

    //update latest timestamp
    if let Err(e) = ChatUserUnreadCount::update_user_unread_count(&master_db_conn, my_uid, send_uid) {
        error!("{}\tfailed update user group unread count and latest timestamp:{:?}",
               default_log_pre!(conn.msg.code as i16,my_uid), e
        );
    };

    conn.get_bin_code(MessageStateCode::Ok, "success", res_data)
}

#[named]
pub async fn get_chat_channel_unread_count(_clients: Clients, conn: LocalConn) -> ResponseResult {
    let db_conn = conn.db_conn(false);
    let mut cursor = std::io::Cursor::new(&conn.msg.body);

    let tid = cursor.read_i16::<LittleEndian>();
    let tid = match tid {
        Ok(uid) => uid,
        Err(_e) => {
            error!(
                "{}\tinvalid tid param.",
                default_log_pre!(conn.msg.code as i16,""),
            );
            let m = "invaild tid param.";
            return conn.get_general_error(m);
        }
    };

    let dst_id_or_kingdom_timestamp = cursor.read_i64::<LittleEndian>();
    let dst_id_or_kingdom_timestamp = match dst_id_or_kingdom_timestamp {
        Ok(uid) => uid,
        Err(_e) => {
            error!(
                "{}\tinvaild dst id or kingdom_timestamp param.",
                default_log_pre!(conn.msg.code as i16,""),
            );
            let m = "invaild dst id or kingdom_timestamp param.";
            return conn.get_general_error(m);
        }
    };
    let uid = cursor.read_i64::<LittleEndian>();
    let uid = match uid {
        Ok(uid) => uid,
        Err(_e) => {
            error!(
                "{}\tinvaild uid param.",
                default_log_pre!(conn.msg.code as i16,""),
            );
            let m = "invaild uid param.";
            return conn.get_general_error(m);
        }
    };

    info!("{}\tsubmit content\ttid:{}\tdst_id_or_kingdom_timestamp:{}\tuid:{}", default_log_pre!(conn.msg.code as i16,uid), tid, dst_id_or_kingdom_timestamp, uid);

    let mut unread_count: i64 = 0;
    match tid {
        1 => {
            //kingdom

            let kingdom_id = match User::get_kingdom_id(&db_conn, uid) {
                Ok(v) => v,
                Err(e) => {
                    error!(
                        "{}\tget kingdom id:{:?}",
                        default_log_pre!(conn.msg.code as i16,uid),
                        e
                    );
                    return conn.get_general_error(e.to_string().as_str());
                }
            };
            if let Ok(v) = ChatMessage::get_kingdom_unread_count(
                &db_conn,
                kingdom_id,
                dst_id_or_kingdom_timestamp,
            ) {
                unread_count = v;
            }
        }
        2 => {
            if let Ok(group_info) =
            ChatGroupsUid::get_groups_info_by_gid(&db_conn, dst_id_or_kingdom_timestamp)
            {
                if let Ok(v) = ChatMessage::get_group_unread_count(
                    &db_conn,
                    dst_id_or_kingdom_timestamp,
                    group_info.latest_timestamp,
                ) {
                    unread_count = v;
                }
            }
        }
        3 => {
            if let Ok(user_unread_count) = ChatUserUnreadCount::get_user_unread_count(
                &db_conn,
                uid,
                dst_id_or_kingdom_timestamp,
            ) {
                unread_count = user_unread_count as i64;
            };
        }
        _ => error!(
            "{}\tinvalid tid param.",
            default_log_pre!(conn.msg.code as i16,uid),
        ),
    }

    let res_data = FrontDisplayChatMessageUnreadCount {
        unread_count: unread_count as i16,
        kind: tid,
    };

    conn.get_bin_code(MessageStateCode::Ok, "success", res_data)
}
