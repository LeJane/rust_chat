use super::db::{DbConnPool, DieselPool};
use super::message::{Message, MessageStateCode};
use crate::BinaryEncode;
use anyhow::Result;
use byteorder::{LittleEndian, WriteBytesExt};
use flate2::write::ZlibEncoder;
use flate2::Compression;
use std::io::Write;
use std::sync::Arc;
use tokio::net::tcp::OwnedWriteHalf;
use tokio::sync::Mutex;

pub struct Connection {
    pub socket: Arc<Mutex<OwnedWriteHalf>>,
    pub master_db: Arc<DieselPool>,
    pub slave_db: Arc<DieselPool>,
    pub msg: Message,
}

impl Connection {
    pub fn get_bin_code<T>(&self, state: MessageStateCode, msg: &str, body: T) -> Result<Vec<u8>>
        where
            T: BinaryEncode + std::fmt::Debug,
    {
        ResponseContext::get_bincode(self.msg.code as u16, self.msg.session_id, state, msg, body)
    }

    pub fn get_general_error(&self, msg: &str) -> Result<Vec<u8>> {
        self.get_bin_code(MessageStateCode::GeneralError, msg, "")
    }

    pub fn db_conn(&self, master: bool) -> DbConnPool {
        if master {
            self.master_db.get().unwrap()
        } else {
            self.slave_db.get().unwrap()
        }
    }
}

#[derive(Debug)]
pub struct ResponseContext<'a, T: BinaryEncode + std::fmt::Debug> {
    pub msg: &'a str,
    pub body: T,
}

impl<'a, T: BinaryEncode + std::fmt::Debug> ResponseContext<'a, T> {
    pub fn get_bincode(
        code: u16,
        session_id: u64,
        state: MessageStateCode,
        msg: &str,
        body: T,
    ) -> Result<Vec<u8>> {
        let mut gz = ZlibEncoder::new(Vec::new(), Compression::default());
        let mut resp = vec![];

        let mut res_body = Vec::new();
        let msg_bytes = msg.as_bytes();
        res_body.write_i16::<LittleEndian>(msg_bytes.len() as i16)?;
        res_body.extend_from_slice(msg_bytes);

        res_body.extend(body.encode()?);

        resp.write_u16::<LittleEndian>(code)?;
        resp.write_u64::<LittleEndian>(session_id)?;
        resp.write_u16::<LittleEndian>(state as u16)?;
        resp.write_u32::<LittleEndian>(res_body.len() as u32)?;

        gz.write_all(&res_body)?;

        let gz_resp = gz.finish()?;

        resp.write_u32::<LittleEndian>(gz_resp.len() as u32)?;

        resp.extend_from_slice(&gz_resp);

        Ok(resp)
    }
}
