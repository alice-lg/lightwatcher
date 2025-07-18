mod error;
use error::Error;

mod health;
mod protocols;
mod responses;
mod routes;
mod status;

pub mod cache;
pub mod rate_limit;
pub mod server;
