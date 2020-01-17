use crate::{
    transaction::Transaction,
    block::Block,
};

#[derive(Debug, Hash)]
pub enum Event {
    Transaction(Transaction),
    PublishTx([u8;32], Vec<u8>),
    RawTransaction(Transaction),
    Block(Block),
    PubKey(Vec<u8>),
    String(String),
    Chat(String),
    Request(String),
}