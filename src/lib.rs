pub mod api;
pub mod bird;
pub mod config;
pub mod parsers;
pub mod state;

pub fn version() -> String {
    env!("CARGO_PKG_VERSION").to_string()
}
