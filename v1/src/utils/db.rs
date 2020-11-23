use diesel::prelude::*;
use diesel::r2d2::{self, ConnectionManager};
use std::env;
use std::sync::Arc;

pub type DieselPool = r2d2::Pool<ConnectionManager<PgConnection>>;
pub type DbConnPool = r2d2::PooledConnection<ConnectionManager<PgConnection>>;


pub fn get_master_diesel_pool() -> Arc<DieselPool> {
    let database_url = env::var("POSTGRESQL_WRITE_URL").expect("POSTGRESQL_WRITE_URL must be set");
    let manager = ConnectionManager::new(database_url);

    Arc::new(
        r2d2::Pool::builder()
            .build(manager)
            .expect("failed init master diesel pool."),
    )
}

pub fn get_slave_diesel_pool() -> Arc<DieselPool> {
    let database_url = env::var("POSTGRESQL_READ_URL").expect("POSTGRESQL_READ_URL must be set");
    let manager = ConnectionManager::new(database_url);

    Arc::new(
        r2d2::Pool::builder()
            .build(manager)
            .expect("failed init slave diesel pool."),
    )
}
