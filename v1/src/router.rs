use crate::chat_system::chat;
use crate::RouterRegister;
use std::sync::Arc;
// use std::fmt;
#[derive(Debug,Clone, PartialEq, Hash, Eq, Copy)]
pub enum RouterCode {
    ConnectionState = 2001,
    SendMessage = 2002,
    PushMessage = 2003,
    GetUserUnReadMessageCount = 2004,
    GetKingdomMessageContent = 2005,
    GetGroupMessageContent = 2006,
    GetP2pUserMessageContent = 2007,
    GetChannelChatMessageUnreadCount = 2008,
}

impl RouterCode {
    pub fn from_u16(code: u16) -> Self {
        match code {
            2001 => RouterCode::ConnectionState,
            2002 => RouterCode::SendMessage,
            2003 => RouterCode::PushMessage,
            2004 => RouterCode::GetUserUnReadMessageCount,
            2005 => RouterCode::GetKingdomMessageContent,
            2006 => RouterCode::GetGroupMessageContent,
            2007 => RouterCode::GetP2pUserMessageContent,
            2008 => RouterCode::GetChannelChatMessageUnreadCount,
            _ => RouterCode::ConnectionState,
        }
    }
}



// impl fmt::Debug for RouterCode {
//     fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
//         write!(f, "{:?}", self as i16)
//     }
// }

pub fn build_routers() -> Arc<RouterRegister> {
    let mut routers = RouterRegister::new();
    routers.add(RouterCode::ConnectionState, chat::connection_state);

    routers.add(RouterCode::SendMessage, chat::send_message);

    routers.add(
        RouterCode::PushMessage,
        chat::client_feedback_server_message,
    );
    routers.add(
        RouterCode::GetUserUnReadMessageCount,
        chat::get_user_message_unread_count,
    );
    routers.add(
        RouterCode::GetKingdomMessageContent,
        chat::get_kingdom_message_content,
    );
    routers.add(
        RouterCode::GetGroupMessageContent,
        chat::get_group_message_content,
    );
    routers.add(
        RouterCode::GetP2pUserMessageContent,
        chat::get_p2p_user_message_content,
    );
    routers.add(
        RouterCode::GetChannelChatMessageUnreadCount,
        chat::get_chat_channel_unread_count,
    );

    Arc::new(routers)
}
