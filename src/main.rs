extern crate static_merkle_tree;
extern crate pretty_env_logger;
extern crate rmp_serde as rmps;
#[macro_use]
extern crate serde_derive;
extern crate rocksdb;
extern crate base64;
extern crate log4rs;
extern crate wasmi;
extern crate clap;
extern crate toml;
extern crate rand;
#[macro_use]
extern crate log;
#[cfg(feature = "quantum")]
extern crate glp;
extern crate hex;

pub mod user_client;
pub mod transaction;
pub mod watparser;
pub mod nemezis;
pub mod gendata;
pub mod conset;
pub mod config;
pub mod block;
pub mod error;
pub mod event;
pub mod hash;
pub mod util;
pub mod sync;
pub mod rpc;
pub mod pk;
pub mod vm;

#[cfg(feature = "quantum")]
pub mod qmain;
#[cfg(not(feature = "quantum"))]
pub mod ecmain;

#[cfg(feature = "quantum")]
fn main() -> Result<(), Box<dyn std::error::Error>>{
    qmain::qmain()
}

#[cfg(not(feature = "quantum"))]
fn main() -> Result<(), Box<dyn std::error::Error>>{
    ecmain::ecmain()
}