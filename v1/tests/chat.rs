use byteorder::{LittleEndian, WriteBytesExt};
use std::io::Cursor;
use v1::utils::binary_helper::*;
use v1::models::{
    chat_messages::FrontDisplayP2pChatMessage,
    chat_messages::FrontDisplayChatMessageUnreadCount,
    chat_messages::FrontDisplayKingdomChatMessage,
    chat_messages::FrontDisplayGroupChatMessage,
    chat_messages::FrontDisplayP2pChatMessageCount,
};
use v1::ChatMessageUnReadCount;

pub mod helper;

use helper::{build_header_req, get_tcp_conn};
use v1::deserialize_binary;

#[tokio::test]
async fn get_chat_channel_unread_count() {
    let req = || -> Vec<u8> {
        let mut body = vec![];
        WriteBytesExt::write_i16::<LittleEndian>(&mut body, 3).unwrap();
        WriteBytesExt::write_i64::<LittleEndian>(&mut body, 5986398665897825204).unwrap();
        WriteBytesExt::write_i64::<LittleEndian>(&mut body, 3455115140489977330).unwrap();

        let req_ctx = build_header_req(2008, body);

        req_ctx
    };
    let res = |body: &[u8]| {
        let mut cursor = Cursor::new(body);
        binary_read_msg(&mut cursor, body);

        let item_length = binary_read_i16(&mut cursor).unwrap();
        if item_length > 0 {
            let data: FrontDisplayChatMessageUnreadCount = deserialize_binary(&mut cursor, body).unwrap();

            let res = serde_json::to_string(&data).expect("failed json encode.");

            println!("Content:{}", res);
        } else {
            println!("No Content");
        }
    };

    get_tcp_conn(req, res).await;
}

#[tokio::test]
async fn get_p2p_user_message_content() {
    let req = || -> Vec<u8> {
        let mut body = vec![];
        binary_write_i64(&mut body, 0).unwrap();
        binary_write_i16(&mut body, 10).unwrap();
        binary_write_i16(&mut body, 0).unwrap();
        binary_write_i64(&mut body, 5335993962540561541).unwrap();
        binary_write_i64(&mut body, 119226146583795989).unwrap();

        let req_ctx = build_header_req(2007, body);

        req_ctx
    };
    let res = |body: &[u8]| {
        let mut cursor = Cursor::new(body);
        binary_read_msg(&mut cursor, body);

        let datas: Vec<FrontDisplayP2pChatMessage> = deserialize_binary(&mut cursor, body).unwrap();

        let res = serde_json::to_string(&datas).expect("failed json encode.");

        println!("Content:{}", res);
    };

    get_tcp_conn(req, res).await;
}

#[tokio::test]
async fn send_message_content() {
    let req = || -> Vec<u8> {
        let mut body = vec![];

        let content = "test sddffff send three.";
        binary_write_i64(&mut body, 3455115140489977330).unwrap();
        binary_write_i8(&mut body, 1).unwrap();
        binary_write_i64(&mut body, 1001).unwrap();
        binary_write_string(&mut body, content).unwrap();

        let req_ctx = build_header_req(2002, body);

        req_ctx
    };
    let res = |body: &[u8]| {
        let mut cursor = Cursor::new(body);
        binary_read_msg(&mut cursor, body);

        let item_length = binary_read_i16(&mut cursor).unwrap();
        if item_length > 0 {
            let datas: FrontDisplayP2pChatMessageCount = deserialize_binary(&mut cursor, body).unwrap();

            let res = serde_json::to_string(&datas).expect("failed json encode.");

            println!("Content:{}", res);
        } else {
            println!("No Content");
        }
    };

    get_tcp_conn(req, res).await;
}

#[tokio::test]
async fn get_kingdom_message_content() {
    let req = || -> Vec<u8> {
        let mut body = vec![];
        binary_write_i64(&mut body, 0).unwrap();
        binary_write_i16(&mut body, 10).unwrap();
        binary_write_i16(&mut body, 1).unwrap();

        binary_write_i64(&mut body, 8331054938119228637).unwrap();

        let req_ctx = build_header_req(2005, body);

        req_ctx
    };
    let res = |body: &[u8]| {
        let mut cursor = Cursor::new(body);

        binary_read_msg(&mut cursor, body);

        let datas: Vec<FrontDisplayKingdomChatMessage> = deserialize_binary(&mut cursor, body).unwrap();

        let res = serde_json::to_string(&datas).expect("failed json encode.");

        println!("Content:{}", res);
    };

    get_tcp_conn(req, res).await;
}

#[tokio::test]
async fn get_group_message_content() {
    let req = || -> Vec<u8> {
        let mut body = vec![];
        binary_write_i64(&mut body, 0).unwrap();
        binary_write_i16(&mut body, 10).unwrap();
        binary_write_i16(&mut body, 1).unwrap();

        binary_write_i64(&mut body, 964652730319640226).unwrap();
        binary_write_i64(&mut body, 3078113928806103503).unwrap();

        let req_ctx = build_header_req(2006, body);

        req_ctx
    };
    let res = |body: &[u8]| {
        let mut cursor = Cursor::new(body);

        binary_read_msg(&mut cursor, body);

        let datas: Vec<FrontDisplayGroupChatMessage> = deserialize_binary(&mut cursor, body).unwrap();

        let res = serde_json::to_string(&datas).expect("failed json encode.");

        println!("Content:{}", res);
    };

    get_tcp_conn(req, res).await;
}

#[tokio::test]
async fn get_user_message_unread_count() {
    let req = || -> Vec<u8> {
        let mut body = vec![];

        binary_write_i64(&mut body, 1599731395).unwrap();
        binary_write_i64(&mut body, 8331054938119228637).unwrap();

        let req_ctx = build_header_req(2004, body);

        req_ctx
    };

    let res = |body: &[u8]| {
        let mut cursor = Cursor::new(body);
        binary_read_msg(&mut cursor, body);

        let item_length = binary_read_i16(&mut cursor).unwrap();
        if item_length > 0 {
            let datas: ChatMessageUnReadCount = deserialize_binary(&mut cursor, body).unwrap();

            let res = serde_json::to_string(&datas).expect("failed json encode.");

            println!("Content:{}", res);
        } else {
            println!("No Content");
        }
    };

    get_tcp_conn(req, res).await;
}
