use crate::user_client::{start_client, start_stdin_handler};
use crate::transaction::{Transaction, TxBody};
use natsclient::{self, ClientOptions};
use std::{
    time::Duration,
    sync::{Arc, RwLock},
    path::Path,
    fs::File,
    io::Read,
    collections::HashMap,
};
use crate::pk::{PetKey, PATHNAME};
use ed25519_dalek::PublicKey;
use crate::event::Event;
use crate::block::{Block, merge};
use crate::config::Config;
use crate::util::{blake2b, vec_to_arr};
use rocksdb::DB;

#[cfg(not(feature = "quantum"))]
pub fn ecmain() -> Result<(), Box<dyn std::error::Error>> {
    println!("ec_edition");
    pretty_env_logger::init();

    let mut args = std::env::args();
    let uris = if args.len() > 1 { vec![args.nth(1).expect("50:no first arg").into()] }
    else { vec!["nats://127.0.0.1:4222".into()] };

    info!("Starting market service...");
    let opts = ClientOptions::builder()
        .cluster_uris(uris)
        .connect_timeout(Duration::from_secs(10))
        .reconnect_attempts(255)
        .build().expect("58:clientoptions builder");

    let keys = if std::path::Path::new(PATHNAME).exists(){
        PetKey::from_pem()
    }else{
        PetKey::new()
    };
    keys.write_pem();
    crate::nemezis::generate_nemezis_block(&keys.ec);
    
    let mut nemezis = File::open(Path::new("NEMEZIS")).expect("68: no nemezis file");
    let mut nemezis_buffer = String::new();
    nemezis.read_to_string(&mut nemezis_buffer);

    let txdb = DB::open_default("tx.db").expect("72: cant open txdb");

    let mut last_block : Block = serde_json::from_str(&nemezis_buffer).expect("74: cant read nemezis block");
    let mut last_hash = last_block.hash();

    let blockdb = DB::open_default("db.db").expect("77: cant open blockdb");
    blockdb.put(last_hash.clone(), &nemezis_buffer).expect("78: cant place nemezis hash in db");
    blockdb.put("block0", last_hash.clone()).expect("79: cant put nemezis block in db");

    let mut pubkeys : HashMap<String, PublicKey> = HashMap::new();
    let mut mempool : HashMap<String, Transaction> = HashMap::new();
    let mut accounts : HashMap<String, u64> = HashMap::new();

    let txdb = Arc::new(txdb);
    let blockdb = Arc::new(blockdb);
    let mut mempool = Arc::new(RwLock::new(mempool));
    let mut accounts = Arc::new(RwLock::new(accounts));

    let (sndr, recv) = std::sync::mpsc::sync_channel(777);

    start_stdin_handler(sndr.clone());
    
    crate::rpc::start_rpc(sndr.clone(), blockdb.clone(), txdb.clone(), Arc::clone(&mempool), Arc::clone(&accounts));

    let mut client = start_client(opts, sndr.clone());
    let config = Config::default();
    
    let mut pool_size : usize = 0;
    let mut block_height : u64 = 0;
    client.publish("PubKey", &keys.ec.public.to_bytes(), None);
    loop{
        let ev = recv.recv().expect("104: receiver failed");
        match ev {
            Event::Block(b)=>{
                if b.validate() {
                    for k in b.hashedblock.blockdata.txes.iter() {
                        if !mempool.read().expect("109: mempool read failed").contains_key(&hex::encode(k)){ continue }
                    }
                    let tree = static_merkle_tree::Tree::from_hashes(b.hashedblock.blockdata.txes.clone(),merge);
                    let merkle_root : Vec<u8> = tree.get_root_hash().expect("112: merkle root failed").to_vec();
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

                    blockdb.put(&lhs, serde_json::to_string(&last_block).unwrap()).expect("137: failed to put block in db");
                    blockdb.put("block".to_owned()+&block_height.to_string(),lhs);

                    pool_size = 0;
                }
            },
            Event::Transaction(tx)=>{
                //handle incoming transaction
                if tx.validate(){
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
                if config.check_limiters(mempool.read().expect("175: mempool read failed").len(),pool_size,last_block.timestamp()){
                    let mut txhashese: Vec<String> = mempool.read().expect("176: mempool read failed").iter().map(|(k, v)| {
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
                    let txhashes: Vec<[u8;32]> = txhashese.iter().map(|k| {
                        println!("{}",k);
                        vec_to_arr(&hex::decode(k.clone()).expect("190: hex decode failed"))
                    } ).collect();
                    last_block = Block::new(last_hash.clone(), txhashes, &keys.ec);
                    client.publish("block.propose", &last_block.block_to_blob(), None);
                }
            },
            Event::RawTransaction(tx)=>{
                //check transaction validity
                client.publish("tx.broadcast", &tx.serialize().as_bytes(), None);
            },
            Event::PublishTx(to, data)=>{
                //sender validity
                let tx = Transaction::new(TxBody::new(to, data), &keys.ec);
                client.publish("tx.broadcast", &tx.serialize().as_bytes(), None);
            },
            Event::String(s)=>{
                //from stdin
                client.publish("chat", s.as_bytes(), None);
            },
            Event::Request(r)=>{
                match r.as_ref() {
                    "pubkey" => { client.publish("PubKey", &keys.ec.public.to_bytes(), None); },
                    _ => {},
                }
            },
            Event::PubKey(pubk)=>{
                let pk = PublicKey::from_bytes(&pubk).expect("218: public key from bytes failed");
                pubkeys.insert(hex::encode(blake2b(&pubk)) ,pk);
            },
            Event::Chat(s)=>{
                //incoming chat
                let tx = Transaction::new(TxBody::new([0;32], s.as_bytes().to_vec()), &keys.ec);
                client.publish("tx.broadcast", &tx.serialize().as_bytes(), None);
                println!("{}", s);
            }
        }
    }
}