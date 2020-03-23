use serde_json::json;
use serde_derive::Deserialize;

use crate::block::Block;
use crate::event::Event;
use crate::error::QanError;

use jsonrpc_http_server::jsonrpc_core::{self, MetaIoHandler, Metadata, Value, Params};
use jsonrpc_http_server::{ServerBuilder, cors::AccessControlAllowHeaders, hyper, RestApi,};
use rocksdb::DB;
use std::{
    sync::{Arc, RwLock},
    collections::HashMap,
};

#[derive(Deserialize)]
struct HashGetter {
	hash: [u8;32],
}
#[derive(Deserialize)]
struct IntGetter {
	height: u64,
}
#[derive(Deserialize)]
struct RawTransaction {
    tx: Vec<u8>
	// tx: crate::transaction::Transaction,
}

// #[derive(Deserialize)]
// struct PublishTransaction {
// 	to:     [u8;32],
//     data:   Option<crate::transaction::VmCall>,
//     secret: String,
// }

#[derive(Default, Clone)]
struct Meta {
	auth: Option<String>,
    token : String
}

impl Metadata for Meta {}

impl Meta{
    fn check(&self)->bool{
        match &self.auth {
            Some(auth) => if auth == &self.token{
                return true
            },
            None => (),
        };
        return false
    }
}

/// Starter function for the JSON-RPC. Methods are explained and exampled separately.
pub fn start_rpc(
    sendr           : std::sync::mpsc::SyncSender<Event>, 
    blocks_db       : Arc<DB>, 
    tx_db           : Arc<DB>, 
    accounts        : Arc<DB>,
    auth_token      : String,
    tvm             : Arc<RwLock<crate::vm::VM>>
){
    std::thread::spawn(move||{ 
        let mut io = MetaIoHandler::default();
        let txpub_sender = sendr.clone();
        // io.add_method_with_meta("publish_transaction", move |params: Params, meta: Meta| {
        //     if !meta.check(){return Err(jsonrpc_core::Error::new(jsonrpc_core::ErrorCode::ServerError(403)))}
        //     let parsed : PublishTransaction = params.parse().expect("66: cant parse publishtransaction");
        //     let secret = match parsed.secret.get(0..2).expect("get"){
        //         #[cfg(not(feature = "quantum"))]
        //         _ => crate::pk::PetKey::from_pem(&parsed.secret).unwrap().ec,
        //         #[cfg(feature = "quantum")]
        //         _ => crate::pk::PetKey::from_pem(&parsed.secret).unwrap().glp,
        //     };
        //     match txpub_sender.clone().send(Event::PublishTx(parsed.to,parsed.data, secret)){
        //         Err(_e) => return Err(jsonrpc_core::Error::internal_error()),
        //         Ok(_) => return Ok(Value::String("transaction_sent".to_string())),
        //     }
        // });

        let rawtxpub_sender= sendr.clone();
        io.add_method_with_meta("publish_raw_transaction", move |params: Params, meta: Meta| {
            if !meta.check(){return Err(jsonrpc_core::Error::new(jsonrpc_core::ErrorCode::ServerError(403)))}
            let parsed : RawTransaction = params.parse().expect("76: cant parse rawtransaction");
            match rawtxpub_sender.clone().send(Event::RawTransaction(parsed.tx)){
                Err(_e) => return Err(jsonrpc_core::Error::internal_error()),
                Ok(_) => return Ok(Value::String("transaction_sent".to_string())),
            }
        });

        let byh_blocks_db = blocks_db.clone();
        io.add_method_with_meta("block_by_height", move |params: Params, meta: Meta| {
            if !meta.check(){return Err(jsonrpc_core::Error::new(jsonrpc_core::ErrorCode::ServerError(403)))}
            let parsed : IntGetter = params.parse().expect("86: cant parse intgetter");
            let bh = match byh_blocks_db.get("block".to_string()+&parsed.height.to_string()) {
                Ok(Some(value)) => value,
                Ok(None) => return Err(jsonrpc_core::Error::internal_error()),
                Err(_e) => return Err(jsonrpc_core::Error::internal_error()),
            };
            match byh_blocks_db.get(&bh) {
                Ok(Some(value)) => {
                    let value : Block = serde_json::from_slice(&value).unwrap();
                    // println!("{}",value);
                    return Ok(json![value])
                },
                Ok(None) => return Err(jsonrpc_core::Error::internal_error()),
                Err(_e) => return Err(jsonrpc_core::Error::internal_error()),
            };
        });

        io.add_method_with_meta("get_account", move |params: Params, meta: Meta| {
            if !meta.check(){return Err(jsonrpc_core::Error::new(jsonrpc_core::ErrorCode::ServerError(403)))}
            let parsed : HashGetter = params.parse().expect("104: cant parse hashgetter");
            let bh = match accounts.get(&parsed.hash) {
                Ok(Some(value)) => return Ok(json![String::from_utf8(value).unwrap()]),
                Ok(None) => return Err(jsonrpc_core::Error::internal_error()),
                Err(_e) => return Err(jsonrpc_core::Error::internal_error()),
            };
        });

        let gettx_sender = sendr.clone();
        io.add_method_with_meta("get_transaction", move |params: Params, meta: Meta| {
            if !meta.check(){return Err(jsonrpc_core::Error::new(jsonrpc_core::ErrorCode::ServerError(403)))}
            let parsed : HashGetter = params.parse().expect("114: cant parse hashgetter");
            let (main_send, from_main) = std::sync::mpsc::sync_channel(1);
            gettx_sender.send(Event::GetTx(parsed.hash, main_send)).unwrap();
            let ret = from_main.recv().unwrap();
            return Ok(json![&ret]);
        });

        io.add_method_with_meta("block_by_hash", move | params: Params, meta: Meta| {
            if !meta.check(){return Err(jsonrpc_core::Error::new(jsonrpc_core::ErrorCode::ServerError(403)))}
            let parsed: HashGetter = params.parse().expect("137: cant parse hashgetter");
            match blocks_db.get(&parsed.hash) {
                Ok(Some(value)) => {
                    let value : Block = serde_json::from_slice(&value).unwrap();
                    return Ok(json![value])
                },
                Ok(None) => return Err(jsonrpc_core::Error::internal_error()),
                Err(_e) => return Err(jsonrpc_core::Error::internal_error()),
            };
        });

        let height_sender = sendr.clone();
        io.add_method_with_meta("getChainHeight", move | params: Params, meta: Meta|{
            if !meta.check(){return Err(jsonrpc_core::Error::new(jsonrpc_core::ErrorCode::ServerError(403)))}
            let (main_send, from_main) = std::sync::mpsc::sync_channel(1);
            height_sender.send(Event::GetHeight(main_send)).unwrap();
            let ret = from_main.recv().unwrap();
            return Ok(json![ret])
        });

        let vm_sender = sendr.clone();
        io.add_method_with_meta("fileLoadContract", move | params: Params, meta: Meta|{
            if !meta.check(){return Err(jsonrpc_core::Error::new(jsonrpc_core::ErrorCode::ServerError(403)))}
            let (main_send, from_main) = std::sync::mpsc::sync_channel(1);
            let parsed : Vec<String> = params.parse().expect("56: cant parse publishtransaction");
            vm_sender.send(Event::VmBuild(parsed[0].clone(), main_send)).unwrap();
            let ret = from_main.recv().unwrap();
            return Ok(Value::String(ret))
        });

        io.add_method_with_meta("callVm", move | params: Params, meta: Meta|{
            if !meta.check(){return Err(jsonrpc_core::Error::new(jsonrpc_core::ErrorCode::ServerError(403)))}
            match params{
                Params::Array(arr) =>{
                    match arr.len(){
                        0 | 1=> return Err(jsonrpc_core::Error::invalid_request()),
                        _ => {
                            let (sc, fun, arr) = crate::vm::VM::handle_rpc_in(arr).expect("none from rpc");
                            let ret = tvm.read().unwrap().call_fun(sc, fun, arr);
                            return Ok(json![serde_json::to_string(&ret).unwrap()])
                        }
                    }
                },
                _ => {return Err(jsonrpc_core::Error::invalid_request())}
            };
        });

        let server = ServerBuilder::new(io)
        .threads(3)
        .cors_allow_headers(AccessControlAllowHeaders::Only(vec!["Authorization".to_owned()]))
        .rest_api(RestApi::Unsecure)
        .meta_extractor(move |req: &hyper::Request<hyper::Body>| {
            let auth = req
                .headers()
                .get(hyper::header::AUTHORIZATION)
                .map(|h| h.to_str().unwrap_or("").to_owned());

            Meta { auth , token : auth_token.clone() }
        })
        .start_http(&"127.0.0.1:8000".parse().expect("160: cant parse rpc start addr"))
        .expect("161: cant start server");
        println!("rpc on : 127.0.0.1:8000");
        server.wait();
    });
}