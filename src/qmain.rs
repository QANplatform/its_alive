use crate::user_client::{start_client, start_sync_sub, start_stdin_handler};
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
use crate::pk::{PATHNAME, PetKey};
use crate::event::{SyncType, Event};
use crate::block::{Block, merge};
use crate::conset::ConsensusSettings;
use crate::util::{blake2b, vec_to_arr};
use crate::sync::{sync, genesis_getter};
#[cfg(feature = "quantum")]
use glp::glp::{GlpPk, gen_pk};
use rocksdb::DB;

#[cfg(feature = "quantum")]
pub fn qmain() -> Result<(), Box<dyn std::error::Error>> {
    println!("quantum_edition");
    // crate::gendata::gen_data();
    //     Ok(())
    // }

    let config = crate::config::get_config();
    let opts = ClientOptions::builder()
        .cluster_uris(config.bootstrap)
        .connect_timeout(Duration::from_secs(10))
        .reconnect_attempts(255)
        .build().expect("building nats client failed");

    let keys = if std::path::Path::new(PATHNAME).exists(){
        PetKey::from_pem(PATHNAME)
    }else{
        let pk = PetKey::new();
        pk.write_pem();
        pk
    };
    let mypk_hash = blake2b(&keys.get_glp_pk_bytes());
    let (sndr, recv) = std::sync::mpsc::sync_channel(777);

    let mut client = start_client(opts, &sndr);
    
    let mut head : Block = genesis_getter("NEMEZIS", &keys, &client);
    let nemezis_hash = head.hash();
    let mut block_height = sync(&client, config.spv);
    println!("genezis hash: {:?}", nemezis_hash);
    let consensus_settings = ConsensusSettings::default();

    let mut txdb = DB::open_default("qtx.db").expect("cannot open txdb");
    let mut blockdb = DB::open_default("qdb.db").expect("cannot open blockdb");
    let mut pubkeys = DB::open_default("qpubkeys.db").expect("cannot open pubkeydb");
    let mut accounts = DB::open_default("qaccounts.db").expect("cannot open accountsdb");
    pubkeys.put(mypk_hash, &keys.get_glp_pk_bytes());
    pubkeys.flush().unwrap();
    let mut mempool : HashMap<[u8;32], Transaction> = HashMap::new();

    let mut vm = Arc::new(RwLock::new(crate::vm::VM::new()));
    let mut pool_size : usize = 0;

    client.publish("PubKey", &keys.get_glp_pk_bytes(), None);
    start_stdin_handler(&sndr);
    start_sync_sub(&sndr, &client);

    let mut txdb = Arc::new(txdb);
    let mut blockdb = Arc::new(blockdb);
    let mut accounts = Arc::new(accounts);
    crate::rpc::start_rpc(sndr, Arc::clone(&blockdb), Arc::clone(&txdb), Arc::clone(&accounts), config.rpc_auth, Arc::clone(&vm));

    println!("main functionality starting");
    'main:loop{
        let ev = recv.recv().expect("internal channel failed on receive");
        match ev {
            Event::Block(bl)=>{
                let b : Block = serde_json::from_slice(&bl).unwrap();
                println!("my_head: {:?} \nincoming_head: {:?}", &head.hash(), b.hash());
                let pubkey : GlpPk = if b.proposer_pub == mypk_hash { keys.get_glp_pk() }else{
                    match pubkeys.get(&b.proposer_pub).expect("db error"){
                        Some(pk) => {
                            GlpPk::from_bytes(&pk)
                        }, None => {
                            let pubkey_vec : Vec<u8> = match client.request("PubKey", &b.proposer_pub, std::time::Duration::new(4,0)){
                                Ok(pk) => pk.payload,
                                Err(_) => continue'main
                            };
                            let pubkey = GlpPk::from_bytes(&pubkey_vec);
                            pubkeys.put(&b.proposer_pub, pubkey_vec);
                            pubkeys.flush().unwrap();
                            pubkey
                        }
                    }
                };
                if !b.verify(&pubkey) || b.hash() == head.hash() { continue'main }
                // if blockdb.get_pinned("block".to_owned()+&b.height.to_string()).expect("blockdb failed").is_some(){continue'main}
                match blockdb.get_pinned(&b.hash()) {
                    Err(_)      =>{panic!("db failure")}
                    Ok(Some(_)) =>{
                        //TODO consensus check
                        if b.hash() == head.hash() && b.sig[0] < head.sig[0]{
                            head = b;
                            blockdb.put("block".to_owned()+&block_height.to_string(), &head.hash()).unwrap();
                            blockdb.put(head.hash(), bl).unwrap();
                            blockdb.flush().unwrap();
                            println!("new head accepted: {:?}", &head.hash());
                        }
                        continue'main
                    }
                    Ok(None) => {
                        if b.height == head.height && b.merkle() == head.merkle() && head.timestamp() < b.timestamp(){
                            blockdb.delete(head.hash());
                            head = b;
                            blockdb.put("block".to_owned()+&head.height.to_string(), head.hash());
                            blockdb.put(head.hash(), bl).unwrap();
                            blockdb.flush().unwrap();
                            println!("new head accepted: {:?}", &head.hash());
                            continue'main
                        }
                        let tree = static_merkle_tree::Tree::from_hashes(b.hashedblock.blockdata.txes.clone(),merge);
                        let merkle_root : Vec<u8> = tree.get_root_hash().expect("couldn't get root while building merkle tree on received block").to_vec();
                        if merkle_root!=b.hashedblock.blockdata.merkle_root { continue'main }
                        for k in b.hashedblock.blockdata.txes.iter() {
                            if !mempool.contains_key(k){
                                if txdb.get_pinned(&k).expect("txdb failure").is_some(){continue'main}
                                let req_tx = match client.request(
                                    "Synchronize", 
                                    &serde_json::to_vec(&SyncType::TransactionAtHash(k.clone())).expect("couldn't serialize request for transaction"),
                                    std::time::Duration::new(4,0)){
                                        Ok(h)=>h.payload,
                                        Err(e)=>{ println!("{}",e); continue'main }
                                };
                                let tx : Transaction = serde_json::from_slice(&req_tx).unwrap();
                                let pubkey = if b.proposer_pub == mypk_hash { keys.get_glp_pk() }else{
                                    match pubkeys.get(&b.proposer_pub).expect("db error"){
                                        Some(pk) => {
                                            GlpPk::from_bytes(&pk) 
                                        }, None => {
                                            let pubkey_vec : Vec<u8> = match client.request("PubKey", &b.proposer_pub, std::time::Duration::new(4,0)){
                                                Ok(pk) => pk.payload,
                                                Err(_) => continue'main
                                            };
                                            let pubkey = GlpPk::from_bytes(&pubkey_vec);
                                            pubkeys.put(&b.proposer_pub ,&pubkey_vec);
                                            pubkeys.flush().unwrap();
                                            pubkey
                                        }
                                    }
                                };
                                if tx.verify(&pubkey){
                                    mempool.insert(*k, tx);
                                }else{
                                    panic!("tx invalid in chain");
                                }
                            }
                        }

                        for k in b.hashedblock.blockdata.txes.iter(){
                            match mempool.remove(k){
                                Some(x)=>{
                                    txdb.put(k, serde_json::to_vec(&x).unwrap()).expect("txdb failed while making verifying db");
                                },
                                None=>{
                                    panic!("memory pool didn't hold a transaction i already ask for and supposedly received");
                                }
                            }
                        }
                        block_height+=1;
                        head = b;
                        let head_hash = &head.hash();
                        blockdb.put("height", block_height.to_string()).expect("couldn't store new chain height");
                        blockdb.put("block".to_owned() + &block_height.to_string(), &head_hash).expect("couldn't store new block hash to its height");
                        blockdb.put(&head_hash, bl).expect("failed to put received, verified and validated block in db");
                        blockdb.flush().unwrap();
                        txdb.flush().unwrap();
                        println!("at height {} is block {:?}", block_height, head_hash);
                        pool_size = 0;
                    }
                }
            },
            Event::Transaction(trax)=>{
                //handle incoming transaction
                let tx : Transaction = serde_json::from_slice(&trax).unwrap();
                let pubkey = if tx.pubkey == mypk_hash { keys.get_glp_pk() }else{
                     match pubkeys.get(&tx.pubkey).expect("db error"){
                        Some(pk) => {
                            GlpPk::from_bytes(&pk)
                        }, None => {
                            let pubkey_vec : Vec<u8> = match client.request("Pubkey", &tx.pubkey, std::time::Duration::new(4,0)){
                                Ok(pk) => pk.payload,
                                Err(_) => continue'main
                            };
                            let pubkey = GlpPk::from_bytes(&pubkey_vec);
                            pubkeys.put(&tx.pubkey ,pubkey_vec);
                            pubkeys.flush().unwrap();
                            pubkey
                        }
                    }
                };
                if tx.verify(&pubkey){
                    pool_size += tx.len();
                    let txh = tx.hash();
                    let recipient = tx.transaction.recipient;
                    match mempool.insert(txh, tx){
                        Some(_)=>{continue'main},
                        None=>{
                            match accounts.get(&recipient){
                                Ok(Some(value))=>{
                                    accounts.put(recipient, 
                                        (String::from_utf8(value).expect("couldn't read stored account tx count").parse::<u64>()
                                            .expect("couldn't parse account tx count")+1).to_string())
                                                .expect("account db failed");
                                },
                                Ok(None)=>{accounts.put(recipient,1.to_string()).expect("couldn't put new new account into db");},
                                Err(_)=>{panic!("account db error")}
                            }
                        }
                    }     
                }
                if consensus_settings.check_limiters(mempool.len(),pool_size,head.timestamp()){
                    let mut txhashese: Vec<[u8;32]> = mempool.iter().map(|(k, v)| {
                        txdb.put(k, serde_json::to_vec(&v).unwrap()).expect("txdb failure while making block");
                        k.to_owned()
                    } ).collect();
                    txhashese.sort();
                    for k in &txhashese{
                        // println!("{:?}", k);
                        mempool.remove(k).unwrap();
                    }
                    pool_size = 0;
                    block_height +=1;
                    head = Block::new(head.hash(), txhashese, &keys.glp, block_height);
                    let head_hash = head.hash();
                    let serde_head = serde_json::to_vec(&head).expect("couldn't serialize block to hash while making it");
                    blockdb.put("height", block_height.to_string()).expect("couldn't store new height while making block");
                    blockdb.put("block".to_owned()+&block_height.to_string(), &head_hash).expect("couldn't store block hash to its height");
                    blockdb.put(&head_hash, &serde_head);
                    println!("at height {} is block {:?}", block_height, head_hash);
                    client.publish("block.propose", &serde_head, None);
                }
            },
            Event::RawTransaction(tx)=>{
                //check transaction validity
                client.publish("tx.broadcast", &tx, None);
            },
            Event::PublishTx(to, data, kp)=>{
                //sender validity
                let tx = Transaction::new(TxBody::new(to, data), &kp);
                client.publish("tx.broadcast", &serde_json::to_vec(&tx).unwrap(), None);
            },

            Event::GetHeight(sendr)=>{
                sendr.send(block_height).expect("couldn't send height to rpc");
            },
            Event::GetTx(hash, sendr)=>{
                sendr.send(match mempool.get(&hash){
                    Some(t)=>serde_json::to_vec(t).unwrap(),
                    None=>continue
                });
            }
            Event::Chat(s)=>{
                //incoming chat
                // println!("{:?}",s);
                let tx = Transaction::new(TxBody::new([0;32], s), &keys.glp);
                client.publish("tx.broadcast", &serde_json::to_vec(&tx).unwrap(), None);
            },
            Event::PubKey(pubk, r)=>{
                match r {
                    Some(to)=>{
                        match pubkeys.get(&pubk).expect("db error"){
                            Some(pk) => client.publish(&to, &pk, None),
                            None => continue'main
                        };
                    },None=>{
                        let pkhash = blake2b(&pubk);
                        if pubkeys.get_pinned(&pkhash).expect("db error").is_none(){
                            pubkeys.put(pkhash ,pubk);
                            pubkeys.flush().unwrap();
                            client.publish("pubkey", &keys.get_glp_pk_bytes(), None);
                        }
                    }
                };
                
            },
            Event::VmBuild(file_name, main_send)=>{
                loop{
                    match vm.try_write(){
                        Ok(mut v)=>{
                            let ret = v.build_from_file("./contracts/".to_owned()+&file_name);
                            main_send.send(ret).expect("couldn't return new smart contract hash to rpc");
                            break
                        }
                        Err(_)=>{ continue }
                    }
                }
            },
            Event::Synchronize(s, r)=>{
                client.publish(&r, 
                &match serde_json::from_slice(&s).expect("couldn't deserialize SyncType on received request"){
                    SyncType::GetHeight => {
                        //chain height
                        // println!("GetHeight");
                        block_height.to_string().as_bytes().to_vec()
                    },
                    SyncType::GetNemezis => {
                        println!("someone asked for genesis");
                        match blockdb.get(&nemezis_hash).expect("couldn't get my genesis block when someone asked for it"){
                            Some(b)=> b,
                            None=> panic!("no genezis block?!")
                        }
                    }
                    SyncType::AtHeight(h) => {
                        //block hash at h height
                        // println!("got asked height {}", h);
                        match blockdb.get("block".to_string()+&h.to_string()).expect("couldn't get block at hash"){
                            Some(h)=>h,
                            None=> {println!("i'm not this high: {}", h);continue'main}
                        }
                    },
                    SyncType::TransactionAtHash(hash) => {
                        //get transaction at hash
                        // println!("got asked tx hash {:?}", hash);
                        match mempool.get(&hash){
                            Some(t) => serde_json::to_vec(&t).expect("couldn't serialize transaction when someone asked for it"),
                            None => match txdb.get(hash).expect("someone asked for a transaction i don't have in mempool or db"){
                                Some(x)=> x,
                                None => {println!("i don't have this tx");continue'main}
                            }
                        }
                    },
                    SyncType::BlockAtHash(hash) => {
                        //get block at hash       
                        println!("got asked block hash {:?}", &hash);  
                        match blockdb.get(&hash).expect("blockdb failure when someone asked for it"){
                            Some(b) => {println!("i can reply"); b}, 
                            None => {println!("someone asked for a block i don't have: {:?}", &hash); continue'main}
                        }
                    },

                    _ => { println!("wrong SyncMessage");continue'main }
                }, 
                None);
            },
        }
    }
}
