use anyhow::{anyhow, Result};
use byteorder::{LittleEndian, ReadBytesExt};
use cityhash::city_hash_64;
use futures::StreamExt;
use std::collections::HashMap;
use std::env;
use std::sync::Arc;
use tokio::io::AsyncReadExt;
use tokio::io::AsyncWriteExt;
use tokio::net::tcp::OwnedWriteHalf;
use tokio::net::TcpListener;
use tokio::sync::Mutex;
use tracing::error;
use function_name::named;
use v1::default_log_pre;

use v1::{
    build_routers, get_slave_diesel_pool,get_master_diesel_pool, Clients, Connection, Message, MessageStateCode,
    ResponseContext, RouterCode,
};

const KEY: &str = "F9B14CEC-60B6-810F-1FF7-8BAE688466AC";

#[named]
#[tokio::main]
async fn main() -> Result<()> {
    tracing::subscriber::set_global_default(
        tracing_subscriber::FmtSubscriber::builder()
            .with_env_filter(
                tracing_subscriber::EnvFilter::from_default_env()
                    .add_directive(tracing_subscriber::filter::LevelFilter::INFO.into()),
            )
            .finish(),
    )
        .unwrap();

    let chat_api_port = env::var("CHAT_API_PORT").expect("must set CHAT_API_PORT env.");
    let mut listener = TcpListener::bind(format!("0.0.0.0:{}", chat_api_port)).await?;

    let routers = build_routers();
    let clients = Clients::new(Mutex::new(HashMap::new()));
    let master_diesel_pool = get_master_diesel_pool();
    let slave_diesel_pool = get_slave_diesel_pool();

    let mut incoming = listener.incoming();

    while let Some(stream) = incoming.next().await {
        match stream {
            Err(_e) => {
                error!("{}\tfailed tcp socket recv message:{:?}\t", default_log_pre!("",""), _e);
            }
            Ok(sock) => {
                let clients = clients.clone();
                let master_diesel_pool = master_diesel_pool.clone();
                let slave_diesel_pool = slave_diesel_pool.clone();
                let routers = routers.clone();

                tokio::spawn(async move {
                    let clients = clients.clone();
                    let master_diesel_pool = master_diesel_pool.clone();
                    let slave_diesel_pool = slave_diesel_pool.clone();
                    let (mut recv, sender) = sock.into_split();

                    let sender = Arc::new(Mutex::new(sender));

                    loop {
                        let mut stream_buf = [0; 65535];
                        let master_diesel_pool = master_diesel_pool.clone();
                        let slave_diesel_pool = slave_diesel_pool.clone();
                        let receiverd = match AsyncReadExt::read(&mut recv, &mut stream_buf).await {
                            Ok(v) if v == 0 => return,
                            Ok(v) => v,
                            Err(ref e) if e.kind() == tokio::io::ErrorKind::ConnectionReset => {
                                return;
                            }
                            Err(e) => {
                                error!("{}\tfailed tcp socket recv message:{:?}\t", default_log_pre!("",""), e);
                                return;
                            }
                        };
                        let mut cursor = std::io::Cursor::new(&stream_buf[..receiverd]);
                        let code = match ReadBytesExt::read_u16::<LittleEndian>(&mut cursor) {
                            Ok(v) => v,
                            Err(e) => {
                                error!("{}\tinvalid code formats:{:?}\t", default_log_pre!("",""), e);
                                let resp = match ResponseContext::get_bincode(
                                    0,
                                    0,
                                    MessageStateCode::GeneralError,
                                    "invalid code.",
                                    "",
                                ) {
                                    Ok(v) => v,
                                    Err(e) => {
                                        error!("{}\tfialed encode response:{:?}\t", default_log_pre!("",""), e);
                                        return;
                                    }
                                };

                                handle_stream(sender.clone(), resp).await;
                                return;
                            }
                        };

                        let version = match ReadBytesExt::read_u8(&mut cursor) {
                            Ok(v) => v,
                            Err(e) => {
                                error!("{}\tinvalid version formats:{:?}\t", default_log_pre!("",""), e);
                                let resp = match ResponseContext::get_bincode(
                                    code,
                                    0,
                                    MessageStateCode::GeneralError,
                                    "invalid version.",
                                    "",
                                ) {
                                    Ok(v) => v,
                                    Err(e) => {
                                        error!("{}\tfialed encode response:{:?}\t", default_log_pre!("",""), e);
                                        return;
                                    }
                                };

                                handle_stream(sender.clone(), resp).await;
                                return;
                            }
                        };

                        let session_id = match ReadBytesExt::read_u64::<LittleEndian>(&mut cursor) {
                            Ok(v) => v,
                            Err(e) => {
                                error!("{}\tinvalid session id formats:{:?}", default_log_pre!(code,""), e);
                                let resp = match ResponseContext::get_bincode(
                                    code,
                                    0,
                                    MessageStateCode::GeneralError,
                                    "invalid session id.",
                                    "",
                                ) {
                                    Ok(v) => v,
                                    Err(e) => {
                                        error!("{}\tfialed encode response:{:?}\t", default_log_pre!(code,""), e);
                                        return;
                                    }
                                };

                                handle_stream(sender.clone(), resp).await;
                                return;
                            }
                        };

                        let signature = match ReadBytesExt::read_u64::<LittleEndian>(&mut cursor) {
                            Ok(v) => v,
                            Err(e) => {
                                error!("{}\tinvalid signature formats:{:?}", default_log_pre!(code,""), e);
                                let resp = match ResponseContext::get_bincode(
                                    code,
                                    session_id,
                                    MessageStateCode::GeneralError,
                                    "invalid signature format.",
                                    "",
                                ) {
                                    Ok(v) => v,
                                    Err(e) => {
                                        error!("{}\tfialed encode response:{:?}\t", default_log_pre!(code,""), e);
                                        return;
                                    }
                                };

                                handle_stream(sender.clone(), resp).await;
                                return;
                            }
                        };

                        let timestamp = match ReadBytesExt::read_u64::<LittleEndian>(&mut cursor) {
                            Ok(v) => v,
                            Err(e) => {
                                error!("{}\tinvalid timestamp formats:{:?}\t", default_log_pre!(code,""), e);
                                let resp = match ResponseContext::get_bincode(
                                    code,
                                    session_id,
                                    MessageStateCode::GeneralError,
                                    "invalid timestamp.",
                                    "",
                                ) {
                                    Ok(v) => v,
                                    Err(e) => {
                                        error!("{}\tfialed encode response:{:?}\t", default_log_pre!(code,""), e);
                                        return;
                                    }
                                };

                                handle_stream(sender.clone(), resp).await;
                                return;
                            }
                        };

                        let len = match ReadBytesExt::read_u32::<LittleEndian>(&mut cursor) {
                            Ok(v) => v,
                            Err(e) => {
                                error!("{}\tinvalid length formats:{:?}", default_log_pre!(code,""), e);
                                let resp = match ResponseContext::get_bincode(
                                    code,
                                    session_id,
                                    MessageStateCode::GeneralError,
                                    "invalid body length.",
                                    "",
                                ) {
                                    Ok(v) => v,
                                    Err(e) => {
                                        error!("{}\tfialed encode response:{:?}\t", default_log_pre!(code,""), e);
                                        return;
                                    }
                                };

                                handle_stream(sender.clone(), resp).await;
                                return;
                            }
                        };
                        let position = cursor.position() as usize;
                        let mut new_body = Vec::new();

                        if len > 0 {
                            let body = &stream_buf[position..position + (len as usize)];
                            new_body.extend_from_slice(body);
                        }

                        //signature valid
                        if let Err(e) = signature_valid(signature, timestamp, &new_body) {
                            error!("{}\tinvalid signature formats:{:?}\t", default_log_pre!(code,""), e);

                            let resp = match ResponseContext::get_bincode(
                                code,
                                session_id,
                                MessageStateCode::GeneralError,
                                "invalid signature.",
                                "",
                            ) {
                                Ok(v) => v,
                                Err(e) => {
                                    error!("{}\tfialed encode response:{:?}\t", default_log_pre!(code,""), e);
                                    return;
                                }
                            };

                            handle_stream(sender.clone(), resp).await;
                            return;
                        }

                        let code_enum = RouterCode::from_u16(code);
                        let msg = Message {
                            code: code_enum,
                            version,
                            session_id,
                            body_len: len,
                            body: new_body,
                        };

                        let conn = Connection {
                            socket: sender.clone(),
                            master_db: master_diesel_pool.clone(),
                            slave_db: slave_diesel_pool.clone(),
                            msg,
                        };

                        match routers.call(code_enum) {
                            Ok(f) => {
                                let resp = match f(clients.clone(), conn).await {
                                    Ok(v) => v,
                                    Err(e) => {
                                        error!("{}\tfailed method exec:{:?}", default_log_pre!(code,""), e);
                                        let resp = match ResponseContext::get_bincode(
                                            code,
                                            session_id,
                                            MessageStateCode::GeneralError,
                                            &e.to_string(),
                                            "",
                                        ) {
                                            Ok(v) => v,
                                            Err(e) => {
                                                error!("{}\tfialed encode response:{:?}\t", default_log_pre!(code,""), e);
                                                return;
                                            }
                                        };
                                        handle_stream(sender.clone(), resp).await;
                                        return;
                                    }
                                };

                                handle_stream(sender.clone(), resp).await;
                            }

                            Err(e) => {
                                error!("{}\trouter code not found:{:?}.\t", default_log_pre!(code,""), e);
                                let resp = match ResponseContext::get_bincode(
                                    code,
                                    session_id,
                                    MessageStateCode::GeneralError,
                                    anyhow!("router code not found.").to_string().as_str(),
                                    "",
                                ) {
                                    Ok(v) => v,
                                    Err(e) => {
                                        error!("{}\tfialed encode response:{:?}\t", default_log_pre!(code,""), e);
                                        return;
                                    }
                                };
                                handle_stream(sender.clone(), resp).await;
                                return;
                            }
                        }
                    }
                });
            }
        }
    }

    Ok(())
}

async fn handle_stream(socket: Arc<Mutex<OwnedWriteHalf>>, body: Vec<u8>) {
    let mut w = socket.lock().await;
    let _written = match w.write_all(&body).await {
        Ok(v) => v,

        Err(_e) => {
            return;
        }
    };
}

//signature valid
fn signature_valid(sign: u64, timestamp: u64, body: &[u8]) -> Result<()> {
    let mut signature = Vec::new();

    signature.extend_from_slice(KEY.as_bytes());

    byteorder::WriteBytesExt::write_u64::<LittleEndian>(&mut signature, timestamp)?;

    signature.extend_from_slice(body);

    let signature = city_hash_64(&signature);

    if sign != signature {
        return Err(anyhow!("invalid signature."));
    }

    Ok(())
}
