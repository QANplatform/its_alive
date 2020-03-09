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
#[cfg(feature = "quantum")]
use glp::glp::GlpPk;
use crate::pk::{PetKey, PATHNAME};
use ed25519_dalek::PublicKey;
use crate::event::{SyncType, Event};
use crate::block::{Block, merge};
use crate::conset::ConsensusSettings;
use crate::util::{blake2b, vec_to_arr};
use crate::error::QanError;
use rocksdb::DB;

pub fn genesis_getter(
    genesis : &str, 
    keys    : &PetKey,
    client  : &Client)-> Result<Block, QanError>{
    #[cfg(feature = "quantum")]
    let mut txdb = DB::open_default("qtx.db").expect("cannot open txdb");
    #[cfg(feature = "quantum")]
    let mut blockdb = DB::open_default("qdb.db").expect("cannot open blockdb");
    #[cfg(not(feature = "quantum"))]
    let mut txdb = DB::open_default("tx.db").expect("cannot open txdb");
    #[cfg(not(feature = "quantum"))]
    let mut blockdb = DB::open_default("db.db").expect("cannot open blockdb");
    let head = match blockdb.get("block0"){
        Ok(Some(n)) => {
            println!("found zero block hash in db");
            match blockdb.get(&n){
                Ok(Some(n)) => {
                    println!("found genesis block in db");
                    serde_json::from_slice(&n).map_err(|e|QanError::Serde(e))?
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
                serde_json::from_slice(&nemezis_buffer).map_err(|e|QanError::Serde(e))?
            }else{
                match client.request("Synchronize", &serde_json::to_vec(&SyncType::GetNemezis).map_err(|e|QanError::Serde(e))?, std::time::Duration::new(8,0)){
                    Ok(n)=>{
                        println!("found no genesis block, I'll ask the others");
                        serde_json::from_slice(&n.payload).map_err(|e|QanError::Serde(e))?
                    }Err(_) => {
                        println!("had to make a genesis block");
                        let (b, t) = crate::nemezis::generate_nemezis_block(&keys)?;
                        let tx = serde_json::to_vec(&t).map_err(|e|QanError::Serde(e))?;
                        txdb.put(t.hash()?, tx);
                        txdb.flush().unwrap();
                        b
                    }
                }
            };
            blockdb.put(head.hash(), &serde_json::to_vec(&head).map_err(|e|QanError::Serde(e))?).expect("cannot place nemezis hash in db");
            blockdb.put("block0", head.hash()).expect("cannot put nemezis block in db");
            blockdb.flush().unwrap();
            head
        },
        Err(e) => panic!(e)
    };
    Ok(head)
}

pub fn sync(client : &Client, spv : u64) -> Result<u64, QanError>{
    #[cfg(feature = "quantum")]
    let mut txdb = DB::open_default("qtx.db").expect("cannot open txdb");
    #[cfg(feature = "quantum")]
    let mut blockdb = DB::open_default("qdb.db").expect("cannot open blockdb");
    #[cfg(feature = "quantum")]
    let mut pubkeys = DB::open_default("qpubkeys.db").expect("cannot open blockdb");
    #[cfg(not(feature = "quantum"))]
    let mut txdb = DB::open_default("tx.db").expect("cannot open txdb");
    #[cfg(not(feature = "quantum"))]
    let mut blockdb = DB::open_default("db.db").expect("cannot open blockdb");
    #[cfg(not(feature = "quantum"))]
    let mut pubkeys = DB::open_default("pubkeys.db").expect("cannot open blockdb");
    let mut block_height : u64 = match blockdb.get("height"){
        Ok(Some(h))=>String::from_utf8_lossy(&h).parse::<u64>().expect("cannot parse my stored chain height before sync"),
        Ok(None)=>{blockdb.put("height",0.to_string()).unwrap(); 0},
        Err(e)=>panic!(e)
    };

    let chain_height = match client.request("Synchronize", &serde_json::to_vec(&SyncType::GetHeight).map_err(|e|QanError::Serde(e))?, std::time::Duration::new(8,0)){
        Ok(h)=>String::from_utf8_lossy(&h.payload).to_string().parse::<u64>().unwrap(),
        Err(_) => 0
    };
    println!("I have {} block, the chain is {} long",block_height, chain_height);

    // for (k,_) in blockdb.iterator(rocksdb::IteratorMode::Start){
    //      println!("{}",String::from_utf8_lossy(&k));
    // }
    println!("I could verify {} of my blocks",block_height);
    if spv != 0 && chain_height >= spv { block_height = chain_height - spv; }
    else{
        for i in 0..block_height{
            match blockdb.get("block".to_owned()+&i.to_string()).expect("block db failed") {
                Some(h) => println!("{:?}",&h),
                None => {block_height = i-1; break},
            }
        }
    }
    if block_height == 0 && chain_height == 0 { return Ok(0) }
    if chain_height >= 1 && block_height == 0 && spv == 0 { block_height = 1; }
    if chain_height > block_height {
        println!("start sync: {}", crate::util::timestamp());
        'blockloop:while block_height < chain_height{
            let block_hash = client.request("Synchronize", 
                &serde_json::to_vec(&SyncType::AtHeight(block_height)).map_err(|e|QanError::Serde(e))?
                ,std::time::Duration::new(8,0))
                    .expect(&format!("sync failed at getting blockheight: {}", block_height.to_string())).payload;
            match blockdb.get_pinned(&block_hash) {
                Err(_)      =>{ panic!("db failure") }
                Ok(Some(_)) =>{ println!("During Sync I found a block I already have: {:?}", block_hash);}
                Ok(None)    =>{
                    let req_block = match client.request("Synchronize", 
                        &serde_json::to_vec(&SyncType::BlockAtHash(crate::util::vec_to_arr(&block_hash))).map_err(|e|QanError::Serde(e))? 
                        ,std::time::Duration::new(8,0)){
                            Ok(r)=>r.payload,
                            Err(_)=>continue
                        };
                        // println!("got blockdata");
                    let block : Block = serde_json::from_slice(&req_block).map_err(|e|QanError::Serde(e))?;
                    // println!("asking for pubkey : {:?}", &block.proposer_pub);
                    let pubkey = match pubkeys.get(&block.proposer_pub).expect("db error"){
                        Some(pk) => {
                            // println!("got pubkey for block");
                            #[cfg(feature = "quantum")]
                            let pk = GlpPk::from_bytes(&pk);
                            #[cfg(not(feature = "quantum"))]
                            let pk = PublicKey::from_bytes(&pk).unwrap();
                            pk
                        }, None => {
                            // println!("dont got pubkey for block");
                            let pubkey_vec : Vec<u8> = match client.request("PubKey", &block.proposer_pub, std::time::Duration::new(8,0)){
                                Ok(pk) => pk.payload,
                                Err(_) => continue'blockloop
                            };
                            #[cfg(feature = "quantum")]
                            let pubkey = GlpPk::from_bytes(&pubkey_vec);
                            #[cfg(not(feature = "quantum"))]
                            let pubkey = PublicKey::from_bytes(&pubkey_vec).unwrap();
                            pubkeys.put(&block.proposer_pub ,pubkey_vec);
                            pubkey
                        }
                    };
                    if !block.verify(&pubkey)? {
                        panic!("found cryptographically invalid transaction in chain");
                    }
                    'txloop:for txh in &block.hashedblock.blockdata.txes{
                        match txdb.get_pinned(&txh) {
                            Err(_)      =>{panic!("db failure")}
                            Ok(Some(_)) =>{ continue }
                            Ok(None)    =>{
                                let req_tx = client.request("Synchronize", 
                                    &serde_json::to_vec(&SyncType::TransactionAtHash(*txh)).map_err(|e|QanError::Serde(e))? ,std::time::Duration::new(8,0))
                                        .expect(&format!("sync failed at getting txh: {:?}", &txh)).payload;
                                match serde_json::from_slice::<Transaction>(&req_tx){
                                    Ok(tx) => {
                                        let pubkey = match pubkeys.get(&tx.pubkey).expect("db error"){
                                            Some(pk) => {
                                                // println!("got pubkey");
                                                #[cfg(feature = "quantum")]
                                                let pk = GlpPk::from_bytes(&pk);
                                                #[cfg(not(feature = "quantum"))]
                                                let pk = PublicKey::from_bytes(&pk).unwrap();
                                                pk
                                            }, None => {
                                                let pubkey_vec : Vec<u8> = match client.request("PubKey", &tx.pubkey, std::time::Duration::new(8,0)){
                                                    Ok(pk) => pk.payload,
                                                    Err(_) => continue'blockloop
                                                };
                                                // println!("didnt have pubkey but someone gave it to me");
                                                #[cfg(feature = "quantum")]
                                                let pubkey = GlpPk::from_bytes(&pubkey_vec);
                                                #[cfg(not(feature = "quantum"))]
                                                let pubkey = PublicKey::from_bytes(&pubkey_vec).unwrap();
                                                pubkeys.put(&tx.pubkey ,pubkey_vec);
                                                pubkey
                                            }
                                        };
                                        if tx.verify(&pubkey)?{
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
    Ok(block_height)
}