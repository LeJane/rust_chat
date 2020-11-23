use crate::{Clients, Connection, RouterCode};
use anyhow::{anyhow, Result};
use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;

type ExecFuture = Pin<Box<dyn Future<Output = ResponseResult> + Send + Sync + 'static>>;
type BoxFn = Box<dyn Fn(Clients, Connection) -> ExecFuture + Send + Sync + 'static>;

pub struct RouterRegister {
    route: HashMap<RouterCode, BoxFn>,
}

impl RouterRegister {
    pub fn new() -> Self {
        RouterRegister {
            route: HashMap::new(),
        }
    }

    pub fn add<F, R>(&mut self, code: RouterCode, callback: F)
    where
        F: Fn(Clients, Connection) -> R + Send + Sync + 'static,
        R: Future<Output = ResponseResult> + Send + Sync + 'static,
    {
        self.route.insert(
            code,
            Box::new(move |clients, conn| Box::pin(callback(clients, conn))),
        );
    }

    pub fn call(&self, code: RouterCode) -> Result<&BoxFn> {
        let f = match self.route.get(&code) {
            Some(h) => h,
            None => {
                return Err(anyhow!("No code found for {:?}", code));
            }
        };

        Ok(f)
    }
}

pub type ResponseResult = Result<Vec<u8>>;
