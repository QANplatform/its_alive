use serde::{Serialize, Deserialize};

use crate::{
    transaction::Transaction,
    block::Block,
};

pub enum Event {
    Transaction(Transaction),
    #[cfg(not(feature = "quantum"))]
    PublishTx([u8;32], Vec<u8>,ed25519_dalek::Keypair),
    #[cfg(feature = "quantum")]
    PublishTx([u8;32], Vec<u8>,glp::glp::GlpSk),
    RawTransaction(Transaction),
    Block(Block),
    String(String),
    Chat(String),
    PubKey(Vec<u8>),
    Synchronize(Vec<u8>, String),
    GetHeight(std::sync::mpsc::SyncSender<u64>),
    VmBuild(std::string::String, std::sync::mpsc::SyncSender<String>),
    GetTx(String, std::sync::mpsc::SyncSender<Transaction>),
}

#[derive(Debug, Serialize, Deserialize)]
pub enum SyncType {
    GetHeight,
    GetNemezis,
    Height(u64),
    AtHeight(u64),
    BlockHash(String),
    TransactionAtHash(String),
    BlockAtHash(String),
    Transaction(Vec<u8>),
    Block(Vec<u8>),
}
