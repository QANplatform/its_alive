use serde_json::json;
use serde_derive::Deserialize;

use crate::block::Block;
use crate::event::Event;

use jsonrpc_http_server::jsonrpc_core::{self, IoHandler, Value, Params};
use jsonrpc_http_server::{ServerBuilder};
use rocksdb::DB;
use std::{
    sync::{Arc, RwLock},
    collections::HashMap,
};

#[derive(Deserialize)]
struct HashGetter {
	hash: String,
}
#[derive(Deserialize)]
struct IntGetter {
	height: u64,
}
#[derive(Deserialize)]
struct RawTransaction {
	tx: crate::transaction::Transaction,
}
#[derive(Deserialize)]
struct PublishTransaction {
	to: [u8;32],
    data:Vec<u8>,
}

pub fn start_rpc(
    sendr           : std::sync::mpsc::SyncSender<Event>, 
    blocks_db       : Arc<DB>, 
    tx_db           : Arc<DB>, 
    amempool        : Arc<RwLock<HashMap<String, crate::transaction::Transaction>>>,
    accounts        : Arc<RwLock<HashMap<String, u64>>>,
){
    std::thread::spawn(move||{ 
        let mut io = IoHandler::new();
        let txpub_sender= sendr.clone();
        io.add_method("publish_transaction", move |_params: Params| {
            let parsed : PublishTransaction = _params.parse().expect("56: cant parse publishtransaction");
            match txpub_sender.clone().send(Event::PublishTx(parsed.to,parsed.data)){
                Err(_e) => return Err(jsonrpc_core::Error::internal_error()),
                Ok(_) => return Ok(Value::String("transaction_sent".to_string())),
            }
        });
        let rawtxpub_sender= sendr.clone();
        io.add_method("publish_raw_transaction", move |_params: Params| {
            let parsed : RawTransaction = _params.parse().expect("64: cant parse rawtransaction");
            match rawtxpub_sender.clone().send(Event::RawTransaction(parsed.tx)){
                Err(_e) => return Err(jsonrpc_core::Error::internal_error()),
                Ok(_) => return Ok(Value::String("transaction_sent".to_string())),
            }
        });
        let byh_blocks_db = blocks_db.clone();
        io.add_method("block_by_height", move |_params: Params| {
            let parsed : IntGetter = _params.parse().expect("72: cant parse intgetter");
            let bh = match byh_blocks_db.get("block".to_string()+&parsed.height.to_string()) {
                Ok(Some(value)) => value,
                Ok(None) => return Err(jsonrpc_core::Error::internal_error()),
                Err(_e) => return Err(jsonrpc_core::Error::internal_error()),
            };
            match byh_blocks_db.get(&bh) {
                Ok(Some(value)) => {
                    let value : Block = serde_json::from_slice(&value).expect("80: cant deserialize block");
                    return Ok(json![value])
                },
                Ok(None) => return Err(jsonrpc_core::Error::internal_error()),
                Err(_e) => return Err(jsonrpc_core::Error::internal_error()),
            };
        });
        io.add_method("get_account", move |_params: Params| {
            let parsed : HashGetter = _params.parse().expect("88: cant parse hashgetter");
            loop{
                match accounts.try_read(){
                    Ok(accounts)=>{
                        match accounts.get(&parsed.hash).clone(){
                            Some(x) => {
                                return Ok(json![x])
                            },
                            None => {
                                return Ok(json![0])
                            }
                        }
                    },Err(_)=>continue
                }
            };
        });
        io.add_method("get_transaction", move |_params: Params| {
            let parsed : HashGetter = _params.parse().expect("105: cant parse hashgetter");
            loop{
                match amempool.try_read(){
                    Ok(amempool)=>{
                        match amempool.get(&parsed.hash).clone(){
                            Some(x) => {
                                return Ok(json![&x])
                            },
                            None => {
                                match tx_db.get(&parsed.hash) {
                                    Ok(Some(value)) => return Ok(json![crate::transaction::Transaction::deserialize_slice(&value)]),
                                    Ok(None) => return Err(jsonrpc_core::Error::internal_error()),
                                    Err(_e) => return Err(jsonrpc_core::Error::internal_error()),
                                }
                            }
                        }
                    },Err(_)=>continue
                }
            };
        });
        io.add_method("block_by_hash", move |_params: Params| {
            let parsed: HashGetter = _params.parse().expect("126: cant parse hashgetter");
            match blocks_db.get(&parsed.hash) {
                Ok(Some(value)) => {
                    let value : Block = serde_json::from_slice(&value).expect("130: cant deserialize block");
                    return Ok(json![value])
                },
                Ok(None) => return Err(jsonrpc_core::Error::internal_error()),
                Err(_e) => return Err(jsonrpc_core::Error::internal_error()),
            };
        });

        let server = ServerBuilder::new(io)
            .threads(3)
            .start_http(&"127.0.0.1:8000".parse().expect("139: cant parse rpc start addr"))
            .expect("140: cant start server");

        server.wait();
    });
}