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
use crate::event::{SyncType, Event};
use crate::block::{Block, merge};
use crate::conset::ConsensusSettings;
use crate::util::{do_hash, vec_to_arr};
use crate::sync::{sync, genesis_getter};
use crate::error::QanError;
use rocksdb::DB;

#[cfg(not(feature = "quantum"))]
pub fn ecmain() -> Result<(), Box<dyn std::error::Error>> {
    // crate::gendata::gen_data();
    //     Ok(())
    // }

    let (config, log_handle) = crate::config::Config::get_config()?;
    let opts = ClientOptions::builder()
        .cluster_uris(config.bootstrap)
        .connect_timeout(Duration::from_secs(10))
        .reconnect_attempts(255)
        .build().unwrap();

    let keys = if std::path::Path::new(PATHNAME).exists(){
        PetKey::from_pem(PATHNAME)?
    }else{
        let pk = PetKey::new();
        pk.write_pem();
        pk
    };
    let mypk_hash = do_hash(&keys.ec.public.to_bytes().to_vec());
    let (sndr, recv) = std::sync::mpsc::sync_channel(777);

    let mut client = start_client(opts, &sndr)?;
    
    let mut head : Block = genesis_getter("qNEMEZIS", &keys, &client)?;
    let nemezis_hash = head.hash();
    let mut block_height = sync(&client, config.spv, &mut head)?;
    info!("genezis hash: {:?}", hex::encode(&nemezis_hash));
    let consensus_settings = ConsensusSettings::default();

    let mut txdb = DB::open_default("tx.db").map_err(|e|QanError::Database(e))?;
    let mut blockdb = DB::open_default("db.db").map_err(|e|QanError::Database(e))?;
    let mut accounts = DB::open_default("accounts.db").map_err(|e|QanError::Database(e))?;
    let mut pubkeys = DB::open_default("pubkeys.db").map_err(|e|QanError::Database(e))?;
    pubkeys.put(mypk_hash, &keys.ec.public.to_bytes()).map_err(|e|QanError::Database(e))?;
    let mut mempool : HashMap<[u8;32], Transaction> = HashMap::new();
    let mut roots : HashMap<[u8;32], [u8;32]> = HashMap::new();
    let mut vm = Arc::new(RwLock::new(crate::vm::VM::new()));
    let mut pool_size : usize = 0;

    client.publish("PubKey", &keys.ec.public.to_bytes(), None).map_err(|e|QanError::Nats(e))?;
    start_stdin_handler(&sndr);
    let mut txdb = Arc::new(txdb);
    let mut blockdb = Arc::new(blockdb);
    let mut accounts = Arc::new(accounts);
    crate::rpc::start_rpc(sndr, Arc::clone(&blockdb), Arc::clone(&txdb), Arc::clone(&accounts), config.rpc_auth, Arc::clone(&vm));

    println!("main functionality starting");
    'main:loop{
        let ev = recv.recv().expect("internal channel failed on receive");
        match ev {
            Event::Block(bl)=>{
                let b : Block = serde_json::from_slice(&bl).map_err(|e|QanError::Serde(e))?;
                info!("my_head: {:?} \nincoming_head: {:?}", hex::encode(&head.hash()), hex::encode(b.hash()));
                let pubkey : PublicKey = if b.proposer_pub == mypk_hash { keys.ec.public }else{
                     match pubkeys.get(&b.proposer_pub).map_err(|e|QanError::Database(e))?{
                        Some(pk) => {
                            PublicKey::from_bytes(&pk).unwrap()
                        }, None => {
                            let pubkey_vec : Vec<u8> = match client.request("PubKey", &b.proposer_pub, std::time::Duration::new(4,0)){
                                Ok(pk) => pk.payload,
                                Err(_) => continue'main
                            };
                            pubkeys.put(&b.proposer_pub ,&pubkey_vec).map_err(|e|QanError::Database(e))?;
                            PublicKey::from_bytes(&pubkey_vec).unwrap()
                        }
                    }
                };
                if !b.verify(&pubkey)? || b.hash() == head.hash() { continue'main }
                if b.height > block_height+1{
                    block_height = sync(&client, config.spv, &mut head)?;
                }else if b.height == block_height+1 {
                    if b.prev_hash() != head.hash() { continue'main }
                }else {
                    continue'main
                }
                match blockdb.get_pinned(&b.hash()) {
                    Err(_)      =>{panic!("db failure")}
                    Ok(Some(_)) =>{
                        //TODO consensus check
                        if b.hash() == head.hash() && b.sig[0] < head.sig[0]{
                            head = b;
                            blockdb.put("block".to_owned()+&block_height.to_string(), &head.hash()).map_err(|e|QanError::Database(e))?;
                            blockdb.put(head.hash(), bl).map_err(|e|QanError::Database(e))?;
                            blockdb.flush().map_err(|e|QanError::Database(e))?;
                            info!("new head accepted: {:?}", hex::encode(&head.hash()));
                        }
                        continue'main
                    }
                    Ok(None) => {
                        if b.height == head.height && b.merkle() == head.merkle() && head.timestamp() < b.timestamp(){
                            blockdb.delete(head.hash()).map_err(|e|QanError::Database(e))?;
                            head = b;
                            blockdb.put("block".to_owned()+&head.height.to_string(), head.hash()).map_err(|e|QanError::Database(e))?;
                            blockdb.put(head.hash(), bl).map_err(|e|QanError::Database(e))?;
                            blockdb.flush().map_err(|e|QanError::Database(e))?;
                            info!("new head accepted: {:?}", hex::encode(&head.hash()));
                            continue'main
                        }
                        let tree = static_merkle_tree::Tree::from_hashes(b.hashedblock.blockdata.txes.clone(),merge);
                        let merkle_root : Vec<u8> = tree.get_root_hash().expect("couldn't get root while building merkle tree on received block").to_vec();
                        if merkle_root!=b.hashedblock.blockdata.merkle_root { continue'main }
                        for k in b.hashedblock.blockdata.txes.iter() {
                            if !mempool.contains_key(k){
                                if txdb.get_pinned(&k).map_err(|e|QanError::Database(e))?.is_some(){continue'main}
                                let req_tx = match client.request(
                                    "Synchronize", 
                                    &serde_json::to_vec(&SyncType::TransactionAtHash(k.clone())).map_err(|e|QanError::Serde(e))?,
                                    std::time::Duration::new(4,0)){
                                        Ok(h)=>h.payload,
                                        Err(e)=>{ error!("{}",e); continue'main }
                                };
                                let tx : Transaction = serde_json::from_slice(&req_tx).map_err(|e|QanError::Serde(e))?;
                                let pubkey = if b.proposer_pub == mypk_hash { keys.ec.public }else{
                                    match pubkeys.get(&b.proposer_pub).map_err(|e|QanError::Database(e))?{
                                        Some(pk) => {
                                            PublicKey::from_bytes(&pk).unwrap() 
                                        }, None => {
                                            let pubkey_vec : Vec<u8> = match client.request("PubKey", &b.proposer_pub, std::time::Duration::new(1,0)){
                                                Ok(pk) => pk.payload,
                                                Err(_) => continue'main
                                            };
                                            pubkeys.put(&b.proposer_pub ,&pubkey_vec).map_err(|e|QanError::Database(e))?;
                                            PublicKey::from_bytes(&pubkey_vec).unwrap()
                                        }
                                    }
                                };
                                if tx.verify(&pubkey)?{
                                    mempool.insert(*k, tx);
                                }else{
                                    panic!("tx invalid in chain");
                                }
                            }
                        }

                        for k in b.hashedblock.blockdata.txes.iter(){
                            match mempool.remove(k){
                                Some(x)=>{
                                    txdb.put(k, serde_json::to_vec(&x).map_err(|e|QanError::Serde(e))?).map_err(|e|QanError::Database(e))?;
                                },
                                None=>{
                                    panic!("memory pool didn't hold a transaction i already ask for and supposedly received");
                                }
                            }
                        }
                        block_height+=1;
                        head = b;
                        let head_hash = &head.hash();
                        blockdb.put("height", block_height.to_string()).map_err(|e|QanError::Database(e))?;
                        blockdb.put("block".to_owned() + &block_height.to_string(), &head_hash).map_err(|e|QanError::Database(e))?;
                        blockdb.put(&head_hash, bl).map_err(|e|QanError::Database(e))?;
                        blockdb.flush().map_err(|e|QanError::Database(e))?;
                        txdb.flush().map_err(|e|QanError::Database(e))?;
                        info!("at height {} is block {:?}", block_height, hex::encode(head_hash));
                        pool_size = 0;
                    }
                }
            },
            Event::Transaction(trax)=>{
                //handle incoming transaction
                let tx : Transaction = serde_json::from_slice(&trax).map_err(|e|QanError::Serde(e))?;
                let pubkey = if tx.pubkey == mypk_hash { keys.ec.public }else{
                     match pubkeys.get(&tx.pubkey).map_err(|e|QanError::Database(e))?{
                        Some(pk) => {
                            PublicKey::from_bytes(&pk).unwrap()
                        }, None => {
                            let pubkey_vec : Vec<u8> = match client.request("Pubkey", &tx.pubkey, std::time::Duration::new(4,0)){
                                Ok(pk) => pk.payload,
                                Err(_) => continue'main
                            };
                            pubkeys.put(&tx.pubkey ,&pubkey_vec).map_err(|e|QanError::Database(e))?;
                            PublicKey::from_bytes(&pubkey_vec).unwrap()
                        }
                    }
                };
                if tx.verify(&pubkey)?{
                    pool_size += tx.len();
                    let txh = tx.hash()?;
                    let recipient = tx.transaction.recipient;
                    match mempool.insert(txh, tx){
                        Some(_)=>{continue'main},
                        None=>{
                            match accounts.get(&recipient){
                                Ok(Some(value))=>{
                                    accounts.put(recipient, 
                                        (String::from_utf8(value).expect("couldn't read stored account tx count").parse::<u64>()
                                            .expect("couldn't parse account tx count")+1).to_string())
                                                .map_err(|e|QanError::Database(e))?;
                                },
                                Ok(None)=>{accounts.put(recipient,1.to_string()).map_err(|e|QanError::Database(e))?;},
                                Err(_)=>{panic!("account db error")}
                            }
                            // if tx.transaction.data.is_some(){
                            //     let dat = tx.get_sc_call().unwrap();
                            //     match roots.get(&dat.sc_hash){
                            //         Some(x) => if x == dat.prev_hash{
                            //             let params = crate::vm::parse_values(dat.params);
                            //             let ret = vm.read().unwrap().call_fun(&dat.sc_hash, &dat.func, params);
                            //             if dat.res_root == ret{
                            //                 roots.insert(&dat.sc_hash, ret);
                            //             }
                            //         }
                            //     };
                            // }
                        }
                    }     
                }
                if consensus_settings.check_limiters(mempool.len(),pool_size,head.timestamp()){
                    let mut txhashese: Vec<[u8;32]> = mempool.iter().map(|(k, v)| {
                        txdb.put(k, serde_json::to_vec(&v).unwrap()).unwrap();
                        k.to_owned()
                    } ).collect();
                    txhashese.sort();
                    for k in &txhashese{
                        trace!("{}", hex::encode(k));
                        mempool.remove(k).unwrap();
                    }
                    pool_size = 0;
                    block_height +=1;
                    head = Block::new(head.hash(), txhashese, &keys.ec, block_height)?;
                    let head_hash = head.hash();
                    let serde_head = serde_json::to_vec(&head).map_err(|e|QanError::Serde(e))?;
                    blockdb.put("height", block_height.to_string()).map_err(|e|QanError::Database(e))?;
                    blockdb.put("block".to_owned()+&block_height.to_string(), &head_hash).map_err(|e|QanError::Database(e))?;
                    blockdb.put(&head_hash, &serde_head).map_err(|e|QanError::Database(e))?;
                    info!("at height {} is block {:?}", block_height, hex::encode(&head_hash));
                    client.publish("block.propose", &serde_head, None).map_err(|e|QanError::Nats(e))?;
                }
            },
            Event::RawTransaction(tx)=>{
                client.publish("tx.broadcast", &tx, None).map_err(|e|QanError::Nats(e))?;
            },
            // Event::PublishTx(to, data, kp)=>{
            //     let tx = Transaction::new(TxBody::new(to, 0, data), &kp)?;
            //     client.publish("tx.broadcast", &serde_json::to_vec(&tx).map_err(|e|QanError::Serde(e))?, None).map_err(|e|QanError::Nats(e))?;
            // },

            Event::GetHeight(sendr)=>{
                sendr.send(block_height).expect("couldn't send height to rpc");
            },
            Event::GetTx(hash, sendr)=>{
                sendr.send(match mempool.get(&hash){
                    Some(t)=>serde_json::to_vec(t).map_err(|e|QanError::Serde(e))?,
                    None=>continue
                });
            }
            Event::PubKey(pubk, r)=>{
                match r {
                    Some(to)=>{
                        match pubkeys.get(&pubk).map_err(|e|QanError::Database(e))?{
                            Some(pk) => client.publish(&to, &pk, None).map_err(|e|QanError::Nats(e))?,
                            None => continue'main
                        };
                    },None=>{
                        let pkhash = do_hash(&pubk);
                        if pubkeys.get_pinned(&pkhash).map_err(|e|QanError::Database(e))?.is_none(){
                            pubkeys.put(pkhash ,pubk).map_err(|e|QanError::Database(e))?;
                            client.publish("pubkey", &keys.ec.public.to_bytes(), None).map_err(|e|QanError::Nats(e))?;
                        }
                    }
                };
                
            },
            Event::VmBuild(file_name, main_send)=>{
                loop{
                    match vm.try_write(){
                        Ok(mut v)=>{
                            let ret = v.build_from_file("./contracts/".to_owned()+&file_name);
                            let wtvr = do_hash(&ret.as_bytes().to_vec());
                            roots.insert(wtvr, wtvr);
                            main_send.send(ret).expect("couldn't return new smart contract hash to rpc");
                            break
                        }
                        Err(_)=>{ continue }
                    }
                }
            },
            Event::Synchronize(s, r)=>{
                client.publish(&r, 
                &match serde_json::from_slice(&s).map_err(|e|QanError::Serde(e))?{
                    SyncType::GetHeight => {
                        //chain height
                        // println!("GetHeight");
                        block_height.to_string().as_bytes().to_vec()
                    },
                    SyncType::GetNemezis => {
                        info!("someone asked for genesis");
                        match blockdb.get(&nemezis_hash).map_err(|e|QanError::Database(e))?{
                            Some(b)=> b,
                            None=> panic!("no genezis block?!")
                        }
                    }
                    SyncType::AtHeight(h) => {
                        //block hash at h height
                        // println!("got asked height {}", h);
                        match blockdb.get("block".to_string()+&h.to_string()).map_err(|e|QanError::Database(e))?{
                            Some(h)=>h,
                            None=> {println!("i'm not this high : {}", h);continue'main}
                        }
                    },
                    SyncType::TransactionAtHash(hash) => {
                        //get transaction at hash
                        // println!("got asked tx hash {:?}", hash);
                        match mempool.get(&hash){
                            Some(t) => serde_json::to_vec(&t).map_err(|e|QanError::Serde(e))?,
                            None => match txdb.get(hash).map_err(|e|QanError::Database(e))?{
                                Some(x)=> x,
                                None => {println!("i don't have this tx: {}", hex::encode(&hash));continue'main}
                            }
                        }
                    },
                    SyncType::BlockAtHash(hash) => {
                        //get block at hash       
                        info!("got asked block hash {:?}", &hash);  
                        match blockdb.get(&hash).map_err(|e|QanError::Database(e))?{
                            Some(b) => {println!("i can reply"); b}, 
                            None => {println!("someone asked for a block i don't have: {}", hex::encode(&hash)); continue'main}
                        }
                    },

                    _ => { error!("wrong SyncMessage");continue'main }
                }, 
                None).map_err(|e|QanError::Nats(e))?;
            },
        }
    }
}
