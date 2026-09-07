// #[macro_use]
// pub mod macros;
// pub mod io;
// pub mod fs;

// pub mod jwk;
// pub mod crypto;
// pub mod jwt;

// pub mod chunk;
// pub mod driver;
// pub mod multithread;
// pub mod handshake;
// pub mod cors;
// pub mod responder;
// pub mod server;

pub mod uuid;
pub mod json;
pub mod date;

#[cfg(feature = "db")]
pub mod db;

// Applies hand-written .sql files in a caller-supplied order. Behind `db`
// because that is what carries sqlx.
#[cfg(feature = "db")]
pub mod migrate;

#[cfg(feature = "telegram")]
pub mod telegram;

#[cfg(feature = "currency")]
pub mod currency;

#[cfg(feature = "energy")]
pub mod energy;

#[cfg(feature = "auth")]
pub mod auth;

#[cfg(feature = "tasker")]
pub mod tasker;

#[cfg(feature = "s3")]
pub mod S3;

#[cfg(feature = "market")]
pub mod market;

// #[cfg(feature = "websocket")]
// pub mod ws;

pub mod env;

// Ungated: `currency` and `energy` are separate features and both need this
// address, so it cannot live inside either one.
pub mod prestige;

// pub use bapesh_macros::main;
