use crate::RouterCode;

#[derive(Debug, Clone)]
pub struct Message {
    pub code: RouterCode,
    pub version: u8,
    pub session_id: u64,
    pub body_len: u32,
    pub body: Vec<u8>,
}

pub enum MessageNotifyType {
    PushType = 1,
    NotifyType = 2,
}

#[derive(Debug, Clone)]
pub enum MessageStateCode {
    Ok = 200,
    NotFound = 403,
    NoContent = 204,
    GeneralError = 503,
}
