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
use crate::block::{Block, merge};
use crate::conset::ConsensusSettings;
use crate::util::{blake2b, vec_to_arr};
use rocksdb::DB;

pub fn genesis_getter(
    genesis : &str, 
    keys    : &PetKey,
    client  : &Client)-> Block{
    let mut txdb = DB::open_default("qtx.db").expect("cannot open txdb");
    let mut blockdb = DB::open_default("qdb.db").expect("cannot open blockdb");
    match blockdb.get("block0"){
        Ok(Some(n)) => {
            println!("found zero block hash in db");
            match blockdb.get(&n){
                Ok(Some(n)) => {
                    println!("found genesis block in db");
                    serde_json::from_slice(&n).expect("couldn't deserialize genesis block i have")
                },
                Ok(None)=>panic!("there is a block0 hash but no genesis block"),
                Err(e)=>panic!(e)
            }
        },
        Ok(None) => {
            let head = if std::path::Path::new("genesis").exists(){
                println!("no zero block in db, but there is a file");
                let mut nemezis = File::open(Path::new("genesis")).expect("I have a genesis block but also have filesystem problems");
                let mut nemezis_buffer = Vec::new();
                nemezis.read_to_end(&mut nemezis_buffer);
                serde_json::from_slice(&nemezis_buffer).expect("cannot deserialize genesis block")
            }else{
                match client.request("Synchronize", &serde_json::to_vec(&SyncType::GetNemezis).expect("cannot serialize genesis block request"), std::time::Duration::new(8,0)){
                    Ok(n)=>{
                        println!("found no genesis block, I'll ask the others");
                        serde_json::from_slice(&n.payload).expect("cannot deserialize genesis block")
                    }Err(_) => {
                        println!("had to make a genesis block");
                        let (b, t) = crate::nemezis::generate_nemezis_block(&keys);
                        txdb.put(t.hash(), serde_json::to_vec(&t).unwrap());
                        txdb.flush().unwrap();
                        b
                    }
                }
            };
            blockdb.put(head.hash(), &serde_json::to_vec(&head).expect("serialization of genesis failed")).expect("cannot place nemezis hash in db");
            blockdb.put("block0", head.hash()).expect("cannot put nemezis block in db");
            blockdb.flush().unwrap();
            head
        },
        Err(e) => panic!(e)
    }
}

pub fn sync(client : &Client) -> u64{
    let mut txdb = DB::open_default("qtx.db").expect("cannot open txdb");
    let mut blockdb = DB::open_default("qdb.db").expect("cannot open blockdb");
    let mut block_height : u64 = match blockdb.get("height"){
        Ok(Some(h))=>String::from_utf8_lossy(&h).parse::<u64>().expect("cannot parse my stored chain height before sync"),
        Ok(None)=>{blockdb.put("height",0.to_string()).unwrap(); 0},
        Err(e)=>panic!(e)
    };

    let chain_height = match client.request("Synchronize", &serde_json::to_vec(&SyncType::GetHeight).expect("cannot serialize SyncType chain height request"), std::time::Duration::new(8,0)){
        Ok(h)=>String::from_utf8_lossy(&h.payload).to_string().parse::<u64>().unwrap(),
        Err(_) => 0
    };
    println!("I have {} block, the chain is {} long",block_height, chain_height);
    for i in 0..block_height{
        match blockdb.get("block".to_owned()+&i.to_string()).expect("block db failed") {
            Some(h) => println!("{:?}",&h),
            None => {block_height = i-1; break},
        }
    }

    // for (k,_) in blockdb.iterator(rocksdb::IteratorMode::Start){
    //      println!("{}",String::from_utf8_lossy(&k));
    // }
    println!("I could verify {} of my blocks",block_height);

    if chain_height > block_height{
        println!("start sync: {}", crate::util::timestamp());
        'blockloop:while block_height < chain_height{
            let block_hash = client.request("Synchronize", 
                &serde_json::to_vec(&SyncType::AtHeight(block_height)).expect("couldn't serialize request for block hash at height") 
                ,std::time::Duration::new(8,0))
                    .expect(&format!("sync failed at getting blockheight: {}", block_height.to_string())).payload;
            match blockdb.get_pinned(&block_hash) {
                Err(_)      =>{ panic!("db failure") }
                Ok(Some(_)) =>{ println!("During Sync I found a block I already have: {:?}", block_hash);}
                Ok(None)    =>{
                    let req_block = match client.request("Synchronize", 
                        &serde_json::to_vec(&SyncType::BlockAtHash(crate::util::vec_to_arr(&block_hash))).expect("couldn't serialize request for block at hash") 
                        ,std::time::Duration::new(8,0)){
                            Ok(r)=>r.payload,
                            Err(_)=>continue
                        };
                    let block : Block = serde_json::from_slice(&req_block).expect("couldn't deserialize block");
                    if !block.verify() {
                        panic!("found cryptographically invalid transaction in chain");
                    }
                    'txloop:for txh in &block.hashedblock.blockdata.txes{
                        match txdb.get_pinned(&txh) {
                            Err(_)      =>{panic!("db failure")}
                            Ok(Some(_)) =>{ continue }
                            Ok(None)    =>{
                                let req_tx = client.request("Synchronize", 
                                    &serde_json::to_vec(&SyncType::TransactionAtHash(*txh)).expect("couldn't serialize transaction request") ,std::time::Duration::new(8,0))
                                        .expect(&format!("sync failed at getting txh: {:?}", &txh)).payload;
                                match serde_json::from_slice::<Transaction>(&req_tx){
                                    Ok(tx) => {
                                        if tx.verify(){
                                            txdb.put(&txh, req_tx).expect("txdb failed");
                                        }else{
                                            panic!("found cryptographically invalid transaction in chain");
                                        }
                                    }, Err(e) => panic!("")
                                } 
                            }
                        }
                    }
                    blockdb.put("block".to_owned()+&block_height.to_string(), block.hash()).expect("blockdb failed");
                    blockdb.put(&block_hash, req_block).expect("blockdb failed");
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