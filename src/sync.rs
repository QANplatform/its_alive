use crate::user_client::{start_client, start_sync_sub, start_stdin_handler};
use crate::transaction::{Transaction, TxBody};
use natsclient::{self, ClientOptions, Client};
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
use crate::event::{SyncType, Event};
use crate::block::{Block, merge, SyncBlock};
use crate::conset::ConsensusSettings;
use crate::util::{blake2b, vec_to_arr};
use rocksdb::DB;

pub fn genesis_getter(
    genesis : &str, 
    keys    : &PetKey,
    blockdb : &mut DB, 
    txdb    : &mut DB,
    client  : &Client)-> Block{
    match blockdb.get("block0"){
        Ok(Some(n)) => {
            println!("found zero block hash in db");
            match blockdb.get(&String::from_utf8_lossy(&n).to_string()){
                Ok(Some(n)) => {
                    println!("found genesis block in db");
                    serde_json::from_str(&String::from_utf8_lossy(&n).to_string()).expect("couldn't deserialize genesis block i have")
                },
                Ok(None)=>panic!("there is a block0 hash but no genesis block"),
                Err(e)=>panic!(e)
            }
        },
        Ok(None) => {
            let head = if std::path::Path::new("genesis").exists(){
                println!("no zero block in db, but there is a file");
                let mut nemezis = File::open(Path::new("genesis")).expect("I have a genesis block but also have filesystem problems");
                let mut nemezis_buffer = String::new();
                nemezis.read_to_string(&mut nemezis_buffer);
                serde_json::from_str(&nemezis_buffer).expect("cannot deserialize genesis block")
            }else{
                match client.request("Synchronize", &serde_json::to_vec(&SyncType::GetNemezis).expect("cannot serialize genesis block request"), std::time::Duration::new(8,0)){
                    Ok(n)=>{
                        println!("found no genesis block, I'll ask the others");
                        let block_vec = match serde_json::from_slice(&n.payload).expect("cannot deserialize SyncType when getting genesis") {
                            SyncType::Block(h)=>h,
                            _ => panic!("not a block as a block in sync")
                        };
                        serde_json::from_str(&String::from_utf8_lossy(&block_vec).to_string()).expect("cannot deserialize genesis block")
                    }Err(_) => {
                        println!("had to make a genesis block");
                        let (b, t) = crate::nemezis::generate_nemezis_block(&keys);
                        txdb.put(t.hash(), t.serialize());
                        txdb.flush().unwrap();
                        b
                    }
                }
            };
            blockdb.put(head.hash(), &serde_json::to_string(&head).expect("serialization of genesis failed")).expect("cannot place nemezis hash in db");
            blockdb.put("block0", head.hash()).expect("cannot put nemezis block in db");
            blockdb.flush().unwrap();
            head
        },
        Err(e) => panic!(e)
    }
}

pub fn sync(
    blockdb : &mut DB, 
    txdb    : &mut DB,
    client  : &Client,
    mut block_height: u64) -> u64{
    let mut block_height = block_height;
    let chain_height = match client.request("Synchronize", &serde_json::to_vec(&SyncType::GetHeight).expect("cannot serialize SyncType chain height request"), std::time::Duration::new(8,0)){
        Ok(h)=>{
            match serde_json::from_slice(&h.payload).expect("cannot deserialize SyncType at getting chain height"){SyncType::Height(h)=>h, _ => 0}
        }Err(_) => 0
    };
    println!("I have {} block, the chain is {} long",block_height, chain_height);
    for i in 0..block_height{
        println!("{}","block".to_owned()+&i.to_string());
        match blockdb.get("block".to_owned()+&i.to_string()).expect("block db failed") {
            Some(h) => println!("{}",String::from_utf8_lossy(&h)),
            None => {block_height = i-1; break},
        }
    }
    println!("I could verify {} of my blocks",block_height);

    if chain_height > block_height{
        println!("start sync: {}", crate::util::timestamp());
        'blockloop:while block_height < chain_height{
            println!("{}",block_height+1);
            let req_block_hash = client.request("Synchronize", 
                &serde_json::to_vec(&SyncType::AtHeight(block_height)).expect("couldn't serialize request for block hash at height") ,std::time::Duration::new(8,0))
                    .expect(&format!("sync failed at getting blockheight: {}", block_height.to_string())).payload;
            let block_hash : String = match serde_json::from_slice(&req_block_hash)
                .expect("cannot deserialize blockhash response") {SyncType::BlockHash(h)=>h, _ => panic!()};
            // println!("sync block: {}", block_hash);
            match blockdb.get_pinned(&block_hash) {
                Err(_)      =>{ panic!("db failure") }
                Ok(Some(_)) =>{ println!("During Sync I found a block I already have: {}", block_hash);}
                Ok(None)    =>{
                    let req_block = client.request("Synchronize", 
                        &serde_json::to_vec(&SyncType::BlockAtHash(block_hash.clone())).expect("couldn't serialize request for block at hash") ,std::time::Duration::new(8,0))
                            .expect(&format!("sync failed at getting block: {}", &block_hash)).payload;
                    let block_vec = match serde_json::from_slice(&req_block).expect("couldn't deserialize message") {
                        SyncType::Block(h)=>h,
                        _ => panic!("not a block as a block in sync")
                    };
                    // println!("{:?}", block_vec);
                    let block : Block = serde_json::from_str(&String::from_utf8_lossy(&block_vec)).expect("couldn't deserialize block");
                    if !block.verify() { //&& !(block.validate(head.timestamp(),block_height,&head.hash()) == (true,true,true)){
                        panic!("found cryptographically invalid transaction in chain");
                        // println!("block invalid in chain");
                        // continue'blockloop
                    }
                    'txloop:for txh in &block.hashedblock.blockdata.txes{
                        let txh = hex::encode(txh);
                        // println!("sync tx: {}", txh);
                        match txdb.get_pinned(&txh) {
                            Err(_)      =>{panic!("db failure")}
                            Ok(Some(_)) =>{ continue }
                            Ok(None)    =>{
                                let req_tx = client.request("Synchronize", 
                                    &serde_json::to_vec(&SyncType::TransactionAtHash(txh.clone())).expect("couldn't serialize transaction request") ,std::time::Duration::new(8,0))
                                        .expect(&format!("sync failed at getting txh: {}", &txh)).payload;
                                let tx : Transaction = match serde_json::from_slice(&req_tx).expect("couldn't deserialize transaction response"){
                                    SyncType::Transaction(h)=> Transaction::deserialize_slice(&h) ,
                                    _ => panic!("to a transaction request received something that's not a transaction")};
                                if tx.verify(){
                                    txdb.put(&txh, tx.serialize()).expect("txdb failed");
                                }else{
                                    panic!("found cryptographically invalid transaction in chain");
                                }
                            }
                        }
                    }
                    blockdb.put("block".to_owned()+&block_height.to_string(), block.hash()).expect("blockdb failed");
                    blockdb.put(&block_hash, serde_json::to_string(&block).expect("couldn't serialize block to store during sync")).expect("blockdb failed");
                }
            }
            block_height+=1;
            blockdb.put("height", block_height.to_string()).unwrap();
            txdb.flush().unwrap();
            blockdb.flush().unwrap();
        }
        println!("end sync: {}", crate::util::timestamp());
        println!("{}",block_height);
    }
    block_height
}