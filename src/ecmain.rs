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
use crate::pk::{PetKey, PATHNAME};
use ed25519_dalek::PublicKey;
use crate::event::{SyncType, Event};
use crate::block::{Block, merge, SyncBlock};
use crate::conset::ConsensusSettings;
use crate::util::{blake2b, vec_to_arr};
use rocksdb::DB;

#[cfg(not(feature = "quantum"))]
pub fn ecmain() -> Result<(), Box<dyn std::error::Error>> {
    println!("ec_edition");
    let config = crate::config::get_config();
    
    info!("Starting market service...");
    let opts = ClientOptions::builder()
        .cluster_uris(config.bootstrap.clone())
        .connect_timeout(Duration::from_secs(10))
        .reconnect_attempts(255)
        .build().expect("58:clientoptions builder");

    let keys = if std::path::Path::new(PATHNAME).exists(){
        PetKey::from_pem(PATHNAME)
    }else{
        PetKey::new()
    };
    keys.write_pem();
    // crate::nemezis::generate_nemezis_block(&keys.ec);
    
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
    let mut accounts = DB::open_default("accounts.db").expect("");

    let txdb = Arc::new(txdb);
    let blockdb = Arc::new(blockdb);
    let mut accounts = Arc::new(accounts);
    

    let (sndr, recv) = std::sync::mpsc::sync_channel(777);

    start_stdin_handler(sndr.clone());

    let vm = Arc::new(RwLock::new(crate::vm::VM::new()));
    let tvm = vm.clone();
    let ConsensusSettings = ConsensusSettings::default();
    let mut pool_size : usize = 0;
    let mut block_height : u64 = match blockdb.get("height"){
        Ok(Some(h))=>String::from_utf8_lossy(&h).parse::<u64>().expect("72 2"),
        Ok(None)=>{blockdb.put("height",0.to_string()); 0},
        Err(e)=>panic!(e)
    };
    

    crate::rpc::start_rpc(sndr.clone(), blockdb.clone(), txdb.clone(), Arc::clone(&accounts), config.rpc_auth.clone(), tvm);
    let mut client = start_client(opts, sndr.clone(), /*hex::encode(keys.ec.public)*/);
    client.publish("PubKey", &keys.ec.public.to_bytes(), None);
    // std::thread::sleep(Duration::new(10,0));

    let height = match client.request("Synchronize", &serde_json::to_vec(&SyncType::GetHeight).expect("79"), std::time::Duration::new(8,0)){
        Ok(h)=>{
            match serde_json::from_slice(&h.payload).expect("81"){SyncType::Height(h)=>h, _ => 0}
        }Err(_) => 0
    };
    println!("{}",block_height);
    for i in 0..block_height{
        println!("{}","block".to_owned()+&i.to_string());
        match blockdb.get("block".to_owned()+&i.to_string()).expect("135") {
            Some(h) => println!("{}",String::from_utf8_lossy(&h)),
            None => {block_height = i-1; break},
        }
    }
    
    println!("start sync: {}", crate::util::timestamp());
    'blockloop:while block_height < height{
        println!("{}",block_height+1);
        let req_block_hash = client.request("Synchronize", 
            &serde_json::to_vec(&SyncType::AtHeight(block_height)).expect("86") ,std::time::Duration::new(8,0)).expect(&format!("blockheight: {}", block_height.to_string())).payload;
        let block_hash : String = match serde_json::from_slice(&req_block_hash).expect("") {SyncType::BlockHash(h)=>h, _ => panic!()};
        // println!("sync block: {}", block_hash);
        match blockdb.get(&block_hash) {
            Err(_)      =>{panic!("db failure")}
            Ok(Some(b)) =>{}
            Ok(None)    =>{
                let req_block = client.request("Synchronize", 
                    &serde_json::to_vec(&SyncType::BlockAtHash(block_hash.clone())).expect("93") ,std::time::Duration::new(8,0)).expect("93 2").payload;
                let block_vec = match serde_json::from_slice(&req_block).expect("94") {
                    SyncType::Block(h)=>h,
                     _ => panic!()
                };
                // println!("{:?}", block_vec);
                let block : Block = serde_json::from_str(&String::from_utf8_lossy(&block_vec)).expect("112");
                if !block.verify() && !(block.validate(last_block.timestamp(),block_height,&last_hash) == (true,true,true)){
                    println!("block invalid in chain");
                    continue'blockloop
                }
                'txloop:for txh in &block.hashedblock.blockdata.txes{
                    let txh = hex::encode(txh);
                    // println!("sync tx: {}", txh);
                    match txdb.get(&txh) {
                        Err(_)      =>{panic!("db failure")}
                        Ok(Some(b)) =>{}
                        Ok(None)    =>{
                            let req_tx = client.request("Synchronize", 
                                &serde_json::to_vec(&SyncType::TransactionAtHash(txh.clone())).expect("105") ,std::time::Duration::new(8,0)).expect("105 2").payload;
                            let tx : Transaction = match serde_json::from_slice(&req_tx).expect("106 2"){SyncType::Transaction(h)=>Transaction::deserialize_slice(&h), _ => panic!()};
                            if tx.verify(){
                                txdb.put(&txh, tx.serialize()).expect("108");
                            }else{
                                panic!("tx invalid in chain");
                            }
                        }
                    }
                }
                blockdb.put("block".to_owned()+&block_height.to_string(), block.hash()).expect("134");
                blockdb.put(&block_hash, serde_json::to_string(&block).expect("135")).expect("135 2")
            }
        }
        block_height+=1;
        blockdb.put("height", block_height.to_string());
        last_hash = block_hash;
        txdb.flush();
        blockdb.flush();
    }

    let mut block_height : u64 = match blockdb.get("height"){
        Ok(Some(h))=>String::from_utf8_lossy(&h).parse::<u64>().expect("72 2"),
        Ok(None)=>{blockdb.put("height",0.to_string()); 0},
        Err(e)=>panic!(e)
    };
    println!("end sync: {}", crate::util::timestamp());
    println!("{}",block_height);

    start_sync_sub(sndr.clone(), &client);

    println!("loop");
    'main:loop{
        let ev = recv.recv().expect("123: receiver failed");
        match ev {
            Event::Block(b)=>{
                println!("my_head: {} \nincoming_head: {}", &last_hash, b.hash());
                match blockdb.get(&b.hash()) {
                    Err(_)      =>{panic!("db failure")}
                    Ok(Some(b)) =>{continue'main}
                    Ok(None)    =>{}
                }
                if !b.verify() { continue'main }
                if b.hash() == last_hash {
                    if b.sig[0] < last_block.sig[0]{
                        blockdb.put("block".to_owned()+&block_height.to_string(), serde_json::to_string(&last_block).unwrap());
                        last_hash = b.hash();
                        last_block = b;
                        blockdb.flush();
                        continue'main
                    }
                }
                if b.validate(last_block.timestamp(),block_height,&last_hash) == (true,true,true){
                    for k in b.hashedblock.blockdata.txes.iter() {
                        if !mempool.contains_key(&hex::encode(k)){ continue'main }
                    }
                    let tree = static_merkle_tree::Tree::from_hashes(b.hashedblock.blockdata.txes.clone(),merge);
                    let merkle_root : Vec<u8> = tree.get_root_hash().expect("131: merkle root failed").to_vec();
                    if merkle_root!=b.hashedblock.blockdata.merkle_root { continue'main }
                    for k in b.hashedblock.blockdata.txes.iter(){
                        let hexed = hex::encode(k);
                        match mempool.remove(&hexed){
                            Some(x)=>{
                                txdb.put(k, x.serialize()).expect("162");
                            },
                            None=>{
                                println!("i ask for : {}", hexed);
                                let req_tx = client.request("Synchronize", 
                                    &serde_json::to_vec(&SyncType::TransactionAtHash(hexed.clone()))
                                        .expect("157") ,std::time::Duration::new(8,0))
                                        .expect("157 2")
                                    .payload;
                                let tx : Transaction = match serde_json::from_slice(&req_tx).expect("158"){
                                    SyncType::Transaction(h)=>Transaction::deserialize_slice(&h), _ => panic!()};
                                if tx.verify(){
                                    txdb.put(&hexed, tx.serialize()).expect("160");
                                }else{
                                    panic!("tx invalid in chain");
                                }
                            }
                        }
                    }
                    block_height+=1;
                    blockdb.put("height", block_height.to_string());
                    println!("height {}", block_height);
                    last_hash = b.hash();
                    last_block = b.clone();
                    let lhs = &last_hash;
                    println!("blockhash: {}", lhs);

                    blockdb.put(&lhs, serde_json::to_string(&last_block).expect("156")).expect("156: failed to put block in db");
                    blockdb.put("block".to_owned()+&block_height.to_string(),lhs);

                    pool_size = 0;
                }
            },
            Event::Transaction(tx)=>{
                //handle incoming transaction
                if tx.verify(){
                    pool_size += tx.len();
                    let txh = hex::encode(tx.hash());
                    let recipient = hex::encode(&tx.transaction.recipient);
                    match mempool.insert(txh.clone(), tx){
                        Some(_)=>println!("already have: {}", &txh),
                        None=>{
                            println!("inserted: {}", &txh);
                            match accounts.get(&recipient){
                                Ok(Some(value))=>{accounts.put(recipient, (String::from_utf8(value).expect("176").parse::<u64>().expect("176 2")+1).to_string());},
                                Ok(None)=>{accounts.put(recipient,1.to_string());},
                                Err(_)=>{()}
                            }
                        }
                    }     
                }
                if ConsensusSettings.check_limiters(mempool.len(),pool_size,last_block.timestamp()){
                    let mut txhashese: Vec<String> = mempool.iter().map(|(k, v)| {
                        txdb.put(k.clone(), v.serialize());
                        k.clone()
                    } ).collect();
                    
                    mempool.clear();
                    txhashese.sort();
                    let txhashes: Vec<[u8;32]> = txhashese.iter().map(|k| {
                        println!("{}",k);
                        vec_to_arr(&hex::decode(k.clone()).expect("206: hex decode failed"))
                    } ).collect();
                    block_height +=1;
                    last_block = Block::new(last_hash.clone(), txhashes, &keys.ec, block_height);
                    last_hash = last_block.hash();
                    blockdb.put("height", block_height.to_string());
                    println!("height {}", block_height);
                    blockdb.put(&last_hash, serde_json::to_string(&last_block).expect("259"));
                    blockdb.put("block".to_owned()+&block_height.to_string(), &last_hash);
                    client.publish("block.propose", &last_block.block_to_blob(), None);
                }
            },
            Event::RawTransaction(tx)=>{
                //check transaction validity
                client.publish("tx.broadcast", &tx.serialize().as_bytes(), None);
            },
            Event::PublishTx(to, data, kp)=>{
                //sender validity
                let tx = Transaction::new(TxBody::new(to, data), &kp);
                client.publish("tx.broadcast", &tx.serialize().as_bytes(), None);
            },
            Event::String(s)=>{
                //from stdin
                client.publish("chat", s.as_bytes(), None);
            },
            Event::GetHeight(sendr)=>{
                sendr.send(block_height).expect("226");
            },
            Event::GetTx(hash, sendr)=>{
                let tx = mempool.get(&hash).unwrap();
                sendr.send(tx.clone());
            }
            Event::Chat(s)=>{
                //incoming chat
                let tx = Transaction::new(TxBody::new([0;32], s.as_bytes().to_vec()), &keys.ec);
                client.publish("tx.broadcast", &tx.serialize().as_bytes(), None);
                // println!("{}", s);
            },
            Event::PubKey(pubk)=>{
                let hexhash= hex::encode(blake2b(&pubk));
                match pubkeys.get(&hexhash){
                    Some(_)=>{}
                    None=>{
                        // println!("{:?}", pubk);
                        let pk = PublicKey::from_bytes(&pubk).expect("218: public key from bytes failed");
                        pubkeys.insert(hexhash ,pk);
                        client.publish("PubKey", &keys.ec.public.to_bytes(), None);
                    }
                }
            },
            Event::VmBuild(file_name, main_send)=>{
                loop{
                    match vm.try_write(){
                        Ok(mut v)=>{
                            let ret = v.build_from_file("./contracts/".to_owned()+&file_name);
                            main_send.send(ret).expect("256");
                            break
                        }
                        Err(_)=>{ continue }
                    }
                }
            },
            Event::Synchronize(s, r)=>{
                let dat =  match serde_json::from_slice(&s).expect("264"){
                    SyncType::GetHeight => {
                        //chain height
                        // println!("GetHeight");
                        SyncType::Height(block_height)
                    },
                    SyncType::AtHeight(h) => {
                        //block hash at h height
                        SyncType::BlockHash(String::from_utf8_lossy(match &blockdb.get("block".to_string()+&h.to_string()).expect("271"){
                            Some(h)=>h,
                            None=> continue'main
                        }).to_string())
                    },
                    SyncType::TransactionAtHash(hash) => {
                        //get transaction at hash
                        let tx = match mempool.get(&hash){
                            Some(t) => serde_json::to_vec(&t).expect("300"),
                            None => match txdb.get(hash).expect("301"){
                                Some(x)=> x,
                                None => continue'main
                            }
                        };
                        SyncType::Transaction(tx)
                    },
                    SyncType::BlockAtHash(hash) => {
                        //get block at hash         
                        SyncType::Block(match blockdb.get(hash).expect("279"){
                            Some(b) => b, None => continue'main
                        })
                    },
                    _ => { continue'main }
                };
                // println!("dat: {:?}", dat);
                client.publish(&r, &serde_json::to_vec(&dat).expect("283"), None);
            },
        }
        txdb.flush();
        blockdb.flush();
    }
}