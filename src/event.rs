use serde::{Serialize, Deserialize};

use crate::{
    transaction::Transaction,
    block::Block,
};

///Enum for internal data diferentiation during internal transmissions.
pub enum Event {
    Transaction(Vec<u8>),
    #[cfg(not(feature = "quantum"))]
    PublishTx([u8;32], Vec<u8>,ed25519_dalek::Keypair),
    #[cfg(feature = "quantum")]
    PublishTx([u8;32], Vec<u8>,glp::glp::GlpSk),
    RawTransaction(Vec<u8>),
    Block(Vec<u8>),
    Chat(Vec<u8>),
    PubKey(Vec<u8>, Option<String>),
    Synchronize(Vec<u8>, String),
    GetHeight(std::sync::mpsc::SyncSender<u64>),
    VmBuild(std::string::String, std::sync::mpsc::SyncSender<String>),
    GetTx([u8;32], std::sync::mpsc::SyncSender<Vec<u8>>),
}

///Enum for cross network sync transmissions.
#[derive(Debug, Serialize, Deserialize)]
pub enum SyncType {
    GetHeight,
    GetNemezis,
    Height(u64),
    AtHeight(u64),
    BlockHash([u8;32]),
    TransactionAtHash([u8;32]),
    BlockAtHash([u8;32]),
}
