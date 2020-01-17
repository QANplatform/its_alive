use crate::user_client::{start_client, start_stdin_handler};
use crate::transaction::{Transaction, TxBody};
use natsclient::{self, ClientOptions};
use std::{
    sync::{Arc, RwLock},
    io::Read,
    collections::HashMap,
};
use crate::pk::{PATHNAME, PetKey};
use crate::event::Event;
use crate::block::{Block, merge};
use crate::config::Config;
use crate::util::{blake2b, vec_to_arr};
#[cfg(feature = "quantum")]
use glp::glp::{GlpPk, gen_pk};
use rocksdb::DB;

#[cfg(feature = "quantum")]
pub fn qmain() -> Result<(), Box<dyn std::error::Error>> {
    println!("q_edition");
    pretty_env_logger::init();

    let uris =  if std::env::args().len() > 1 {
        vec![std::env::args().nth(1).unwrap().into()]
    }else{
        vec!["nats://127.0.0.1:4222".into()]
    };

    info!("Starting market service...");
    let opts = ClientOptions::builder()
        .cluster_uris(uris)
        .connect_timeout(std::time::Duration::from_secs(10))
        .reconnect_attempts(255)
        .build().expect("38:clientoptions builder");

    let keys = if std::path::Path::new(PATHNAME).exists(){
        PetKey::from_pem()
    }else{
        PetKey::new()
    };
    keys.write_pem();

    crate::nemezis::generate_nemezis_block(&keys.glp);
    let mut nemezis = std::fs::File::open(std::path::Path::new("qNEMEZIS")).unwrap();
    let mut nemezis_buffer = String::new();
    nemezis.read_to_string(&mut nemezis_buffer);

    let txdb = DB::open_default("tx.db").unwrap();
    let txdb = Arc::new(txdb);

    let mut last_block : Block = serde_json::from_str(&nemezis_buffer).unwrap();
    let mut last_hash = last_block.hash();

    let blockdb = DB::open_default("db.db").unwrap();
    blockdb.put(last_hash.clone(), &nemezis_buffer).unwrap();
    blockdb.put("block0", last_hash.clone()).unwrap();
    let blockdb = Arc::new(blockdb);


    let mut pubkeys : HashMap<Vec<u8>, GlpPk> = HashMap::new();

    let mut mempool : HashMap<String, Transaction> = HashMap::new();
    let mut mempool = Arc::new(RwLock::new(mempool));

    let mut accounts : HashMap<String, u64> = HashMap::new();
    let mut accounts = Arc::new(RwLock::new(accounts));

    let (sndr, recv) = std::sync::mpsc::sync_channel(777);

    start_stdin_handler(sndr.clone());
    
    crate::rpc::start_rpc(sndr.clone(), blockdb.clone(), txdb.clone(), Arc::clone(&mempool), Arc::clone(&accounts));

    let mut client = start_client(opts, sndr.clone());
    let config = Config::default();
    let mut pool_size : usize = 0;
    let mut block_height : u64 = 0;
    client.publish("PubKey", &gen_pk(&keys.glp).to_bytes(), None);
    loop{
        let ev = recv.recv().unwrap();
        match ev {
            Event::Block(b)=>{
                if b.validate() {
                    for k in b.hashedblock.blockdata.txes.iter() {
                        if !mempool.read().unwrap().contains_key(&hex::encode(k)){ continue }
                    }
                    let tree = static_merkle_tree::Tree::from_hashes(b.hashedblock.blockdata.txes.clone(),merge);
                    let merkle_root : Vec<u8> = tree.get_root_hash().unwrap().to_vec();
                    if merkle_root!=b.hashedblock.blockdata.merkle_root {continue}
                    loop{
                        match mempool.try_write() {
                            Ok(mut pool) => {
                                for k in b.hashedblock.blockdata.txes.iter(){
                                    match pool.remove(&hex::encode(k)){
                                        Some(x)=>{
                                            txdb.put(k, x.serialize());
                                        },
                                        None=>continue
                                    }
                                }
                                break
                            },
                            Err(_) => continue,
                        }
                    };
                    block_height+=1;
                    last_hash = b.hash();
                    last_block = b.clone();

                    let lhs = &last_hash;
                    println!("blockhash: {}", lhs);

                    blockdb.put(&lhs, &serde_json::to_string(&last_block).unwrap()).unwrap();
                    blockdb.put("block".to_owned()+&block_height.to_string(),lhs);

                    pool_size = 0;
                }
            },
            Event::Transaction(tx)=>{
                //handle incoming transaction
                if tx.validate(){
                    println!("valid transaction");
                    pool_size += tx.len();
                    let txh = hex::encode(tx.hash());
                    println!("tx hash: {}",txh);
                    let recipient = hex::encode(&tx.transaction.recipient);
                    loop{
                        match mempool.try_write() {
                            Ok(mut pool) => {
                                match pool.insert(txh, tx){
                                    Some(_)=>break,
                                    None=>{
                                        loop{
                                            match accounts.try_write() {
                                                Ok(mut accs) => {
                                                    println!("{}",recipient);
                                                    *accs.entry(recipient).or_insert(0)+=1;
                                                    break
                                                },
                                                Err(_) => continue,
                                            }
                                        };
                                    }
                                }
                                break
                            },
                            Err(_) => continue,
                        }
                    };

                }
                if config.check_limiters(mempool.read().unwrap().len(),pool_size,last_block.timestamp()){
                    let mut txhashese: Vec<String> = mempool.read().unwrap().iter().map(|(k, v)| {
                        txdb.put(k.clone(), v.serialize());
                        k.clone()
                    } ).collect();
                    loop{
                        match mempool.try_write() {
                            Ok(mut pool) => {
                                pool.clear();
                                break
                            },
                            Err(_) => continue,
                        }
                    };
                    txhashese.sort();
                    let txhashes: Vec<[u8;32]> = txhashese.iter().map(|k| {println!("{}",k);vec_to_arr(&hex::decode(k.clone()).unwrap())} ).collect();
                    last_block = Block::new(last_hash.clone(), txhashes, &keys.glp);
                    client.publish("block.propose", &last_block.block_to_blob(), None);
                }
            },
            Event::RawTransaction(tx)=>{
                client.publish("tx.broadcast", &tx.serialize().as_bytes(), None);
            },
            Event::PublishTx(to, data)=>{
                //TODO sender validity
                let tx = Transaction::new(TxBody::new(to, data), &keys.glp);
                client.publish("tx.broadcast", &tx.serialize().as_bytes(), None);
            },
            Event::String(s)=>{
                //from stdin
                client.publish("chat", s.as_bytes(), None);
            },
            Event::Request(r)=>{
                match r.as_ref() {
                    "pubkey" => { client.publish("PubKey", &gen_pk(&keys.glp).to_bytes(), None); },
                    _ => {},
                }
            },
            Event::PubKey(pubk)=>{
                let pk = glp::glp::GlpPk::from_bytes(&pubk);
                pubkeys.insert(blake2b(&pubk).to_vec() ,pk);
            },
            Event::Chat(s)=>{
                let tx = Transaction::new(TxBody::new([0;32], s.as_bytes().to_vec()), &keys.glp);
                client.publish("tx.broadcast", &tx.serialize().as_bytes(), None);
                println!("{}", s);
            }
        }
    }
}