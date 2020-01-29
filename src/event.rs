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
    PubKey(Vec<u8>),
    String(String),
    Chat(String),
    Request(String),
    VmBuild(std::string::String, std::sync::mpsc::SyncSender<String>),
}