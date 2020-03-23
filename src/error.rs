use std::error::Error;
use std::fs;
use std::io;
use std::num;

#[derive(Debug)]
pub enum QanError{
    Io(io::Error),
    Rpc(jsonrpc_http_server::jsonrpc_core::types::error::Error),
    Hash(core::convert::Infallible),
    Nats(natsclient::error::Error),
    Serde(serde_json::Error),
    // Crypto(ed25519_dalek::SignatureError),
    Database(rocksdb::Error),
    Internal(String)
}

impl std::fmt::Display for QanError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match *self {
            QanError::Io(ref err) => write!(f, "IO error: {}", err),
            QanError::Rpc(ref err) => write!(f, "Rpc error: {}", err),
            QanError::Hash(ref err) => write!(f, "Hash error: {}", err),
            QanError::Nats(ref err) => write!(f, "Nats error: {}", err),
            QanError::Serde(ref err) => write!(f, "Serde error: {}", err),
            // QanError::Crypto(ref err) => write!(f, "Crypto error: {}", err),
            QanError::Database(ref err) => write!(f, "Database error: {}", err),
            QanError::Internal(ref err)  => write!(f, "Internal error: {}", err),
        }
    }
}

impl std::error::Error for QanError {
    fn cause(&self) -> Option<&std::error::Error> {
        match *self {
            QanError::Io(ref err) => Some(err),
            QanError::Rpc(ref err) => Some(err),
            QanError::Hash(ref err) => Some(err),
            QanError::Nats(ref err) => Some(err),
            QanError::Serde(ref err) => Some(err),
            // QanError::Crypto(ref err) => Some(err),
            QanError::Database(ref err) => Some(err),
            QanError::Internal(_) => None
        }
    }
}