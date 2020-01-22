//#![feature(proc_macro_hygiene, decl_macro, async_closure)]

extern crate pretty_env_logger;
#[macro_use]
extern crate log;
#[macro_use]
extern crate serde_derive;
extern crate rmp_serde as rmps;
extern crate static_merkle_tree;
extern crate hex;
extern crate rand;
extern crate rocksdb;
#[cfg(feature = "quantum")]
extern crate glp;
extern crate clap;
extern crate base64;

pub mod user_client;
pub mod transaction;
pub mod nemezis;
pub mod config;
pub mod block;
pub mod event;
pub mod util;
pub mod rpc;
pub mod pk;

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