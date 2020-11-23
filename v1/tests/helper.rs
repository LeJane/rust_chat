use byteorder::{LittleEndian, ReadBytesExt, WriteBytesExt};
use cityhash::city_hash_64;
use flate2::read::ZlibDecoder;
use std::io::prelude::*;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::io::AsyncReadExt;
use tokio::io::AsyncWriteExt;
use tokio::net::TcpStream;
use tokio::sync::Mutex;
use tracing::error;

const KEY: &str = "F9B14CEC-60B6-810F-1FF7-8BAE688466AC";

//signature
pub fn signature(timestamp: u64, body: &[u8]) -> u64 {
    let mut signature = Vec::new();

    signature.extend_from_slice(KEY.as_bytes());

    WriteBytesExt::write_u64::<LittleEndian>(&mut signature, timestamp).unwrap();

    signature.extend_from_slice(body);

    city_hash_64(&signature)
}

pub fn build_header_req(code: u16, body: Vec<u8>) -> Vec<u8> {
    let mut req = vec![];
    WriteBytesExt::write_u16::<LittleEndian>(&mut req, code).unwrap();
    WriteBytesExt::write_u8(&mut req, 1).unwrap();
    WriteBytesExt::write_u64::<LittleEndian>(&mut req, 782348283).unwrap();
    let timestamp = 1599561154;
    let sign = signature(timestamp, &body);
    WriteBytesExt::write_u64::<LittleEndian>(&mut req, sign).unwrap();
    WriteBytesExt::write_u64::<LittleEndian>(&mut req, timestamp).unwrap();
    WriteBytesExt::write_u32::<LittleEndian>(&mut req, body.len() as u32).unwrap();
    req.extend_from_slice(&body);

    req
}

pub async fn get_tcp_conn<F, E>(req: F, res: E)
where
    F: FnOnce() -> Vec<u8>,
    E: FnOnce(&[u8]),
{
    let addr = "192.168.1.43:9933".parse::<SocketAddr>().unwrap();
    let stream = TcpStream::connect(addr).await.unwrap();

    let stream_arc = Arc::new(Mutex::new(stream));
    send_tcp_data(stream_arc.clone(), req).await;
    read_tcp_data(stream_arc.clone(), res).await;
}

async fn send_tcp_data<F>(stream_arc: Arc<Mutex<TcpStream>>, req: F)
where
    F: FnOnce() -> Vec<u8>,
{
    let req = req();

    stream_arc.lock().await.write(&req).await.unwrap();
}

async fn read_tcp_data<F>(stream_arc: Arc<Mutex<TcpStream>>, res: F)
where
    F: FnOnce(&[u8]),
{
    let mut buf = vec![0; 1024];

    let n = match stream_arc.lock().await.read(&mut buf).await {
        Ok(n) => n,
        Err(e) => {
            error!("tcp server read error:{:?}", e);
            return;
        }
    };

    let mut new_buf = vec![];
    new_buf.extend_from_slice(&buf[..n]);

    let two_new_buf = new_buf.clone();

    let mut cursor = std::io::Cursor::new(new_buf);
    let code = ReadBytesExt::read_u16::<LittleEndian>(&mut cursor).unwrap();
    let ssession_id = ReadBytesExt::read_u64::<LittleEndian>(&mut cursor).unwrap();
    let state = ReadBytesExt::read_u16::<LittleEndian>(&mut cursor).unwrap();
    let body_len = ReadBytesExt::read_u32::<LittleEndian>(&mut cursor).unwrap();
    let zlib_len = ReadBytesExt::read_u32::<LittleEndian>(&mut cursor).unwrap();
    let position = cursor.position() as usize;
    let mut new_body = Vec::new();

    println!(
        "two_new_buf:{},{},P{},",
        two_new_buf.len(),
        position,
        position + (zlib_len as usize)
    );

    if body_len > 0 {
        let body = &two_new_buf[position..position + (zlib_len as usize)];
        let mut zlib = ZlibDecoder::new(&body[..]);
        let mut s = Vec::new();
        zlib.read_to_end(&mut s).unwrap();

        new_body.extend_from_slice(&s);
    }

    println!(
        "recv content->code:{},session_id:{},state:{},len:{},position:{},total:{}",
        code,
        ssession_id,
        state,
        body_len,
        position,
        position + (zlib_len as usize)
    );

    res(&new_body);
}
