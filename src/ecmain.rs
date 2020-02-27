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
    let opts = ClientOptions::builder()
        .cluster_uris(config.bootstrap.clone())
        .connect_timeout(Duration::from_secs(10))
        .reconnect_attempts(255)
        .build().expect("building nats client failed");
    let mut txdb = DB::open_default("tx.db").expect("cannot open txdb");
    let mut blockdb = DB::open_default("db.db").expect("cannot open blockdb");
    let mut accounts = DB::open_default("accounts.db").expect("cannot open accountsdb");

    let keys = if std::path::Path::new(PATHNAME).exists(){
        PetKey::from_pem(PATHNAME)
    }else{
        let pk = PetKey::new();
        pk.write_pem();
        pk
    };

    let (sndr, recv) = std::sync::mpsc::sync_channel(777);

    let mut client = start_client(opts, sndr.clone());
    let (mut nemezis_hash, mut head) : (String, Block) = match blockdb.get("block0"){
        Ok(Some(n)) => {
            match blockdb.get(&String::from_utf8_lossy(&n).to_string()){
                Ok(Some(n)) => {
                    let n : Block = serde_json::from_str(&String::from_utf8_lossy(&n).to_string()).expect("couldn't deserialize genesis block i have");
                    (n.hash(), n)
                },
                Ok(None)=>panic!("there is a block0 hash but no genesis block"),
                Err(e)=>panic!(e)
            }
        },
        Ok(None) => {
            let head : Block = if std::path::Path::new("NEMEZIS").exists(){
                let mut nemezis = File::open(Path::new("NEMEZIS")).expect("I have a genesis block but also have filesystem problems");
                let mut nemezis_buffer = String::new();
                nemezis.read_to_string(&mut nemezis_buffer);
                serde_json::from_str(&nemezis_buffer).expect("cannot deserialize genesis block")
            }else{
                match client.request("Synchronize", &serde_json::to_vec(&SyncType::GetNemezis).expect("cannot serialize genesis block request"), std::time::Duration::new(8,0)){
                    Ok(n)=>{
                        let block_vec = match serde_json::from_slice(&n.payload).expect("cannot deserialize SyncType when getting genesis") {
                            SyncType::Block(h)=>h,
                            _ => panic!("not a block as a block in sync")
                        };
                        serde_json::from_str(&String::from_utf8_lossy(&block_vec).to_string()).expect("cannot deserialize genesis block")
                    }Err(_) => {
                        let (b, t) = crate::nemezis::generate_nemezis_block(&keys.ec);
                        txdb.put(t.hash(), t.serialize());
                        b
                    }
                }
            };
            blockdb.put(head.hash(), &serde_json::to_string(&head).expect("serialization of genesis failed")).expect("cannot place nemezis hash in db");
            blockdb.put("block0", head.hash()).expect("cannot put nemezis block in db");
            blockdb.flush().unwrap();
            (head.hash(), head)
        },
        Err(e) => panic!(e)
    };
    println!("genezis hash: {}", nemezis_hash);
    let consensus_settings = ConsensusSettings::default();

    let mut pubkeys : HashMap<String, PublicKey> = HashMap::new();
    let mut mempool : HashMap<String, Transaction> = HashMap::new();

    let mut txdb = Arc::new(txdb);
    let mut blockdb = Arc::new(blockdb);
    let mut accounts = Arc::new(accounts);
    let mut vm = Arc::new(RwLock::new(crate::vm::VM::new()));
    let mut tvm = vm.clone();
    let mut pool_size : usize = 0;
    let mut block_height : u64 = match blockdb.get("height"){
        Ok(Some(h))=>String::from_utf8_lossy(&h).parse::<u64>().expect("cannot parse my stored chain height before sync"),
        Ok(None)=>{blockdb.put("height",0.to_string()).unwrap(); 0},
        Err(e)=>panic!(e)
    };

    // client.publish("PubKey", &keys.ec.public.to_bytes(), None).unwrap();
    // std::thread::sleep(Duration::new(10,0));

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
            match blockdb.get(&block_hash) {
                Err(_)      =>{ panic!("db failure") }
                Ok(Some(b)) =>{ println!("During Sync I found a block I already have: {}", block_hash);}
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
                        match txdb.get(&txh) {
                            Err(_)      =>{panic!("db failure")}
                            Ok(Some(b)) =>{ continue }
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

        let mut block_height : u64 = match blockdb.get("height"){
            Ok(Some(h))=>String::from_utf8_lossy(&h).parse::<u64>().expect("cannot parse stored chain height after sync"),
            Ok(None)=>{blockdb.put("height",0.to_string()).unwrap(); 0},
            Err(e)=>panic!(e)
        };
        println!("end sync: {}", crate::util::timestamp());
        println!("{}",block_height);
    }

    start_stdin_handler(sndr.clone());
    start_sync_sub(sndr.clone(), &client);
    crate::rpc::start_rpc(sndr.clone(), blockdb.clone(), txdb.clone(), Arc::clone(&accounts), config.rpc_auth.clone(), tvm);

    println!("main functionality starting");
    'main:loop{
        let ev = recv.recv().expect("internal channel failed on receive");
        match ev {
            Event::Block(b)=>{
                println!("my_head: {} \nincoming_head: {}", &head.hash(), b.hash());
                if !b.verify() || b.hash() == head.hash() { continue'main }
                match blockdb.get(&b.hash()) {
                    Err(_)      =>{panic!("db failure")}
                    Ok(Some(_)) =>{
                        //TODO consensus check
                        if b.hash() == head.hash() && b.sig[0] < head.sig[0]{
                            head = b;
                            blockdb.put("block".to_owned()+&block_height.to_string(), &head.hash()).unwrap();
                            blockdb.put(head.hash(), serde_json::to_string(&head).unwrap()).unwrap();
                            blockdb.flush().unwrap();
                            println!("new head accepted: {}", &head.hash());
                        }
                        continue'main
                    }
                    Ok(None) => {
                        if b.height == head.height && b.merkle() == head.merkle() && head.timestamp() < b.timestamp(){
                            blockdb.delete(head.hash());
                            head = b;
                            blockdb.put("block".to_owned()+&head.height.to_string(), head.hash());
                            blockdb.put(head.hash(), serde_json::to_string(&head).unwrap()).unwrap();
                            blockdb.flush().unwrap();
                            println!("new head accepted: {}", &head.hash());
                            continue'main
                        }
                        let tree = static_merkle_tree::Tree::from_hashes(b.hashedblock.blockdata.txes.clone(),merge);
                        let merkle_root : Vec<u8> = tree.get_root_hash().expect("couldn't get root while building merkle tree on received block").to_vec();
                        if merkle_root!=b.hashedblock.blockdata.merkle_root { continue'main }
                        for k in b.hashedblock.blockdata.txes.iter() {
                            let hexed = hex::encode(k);
                            if !mempool.contains_key(&hex::encode(k)){ 
                                let req_tx = match client.request(
                                    "Synchronize", 
                                    &serde_json::to_vec(&SyncType::TransactionAtHash(hexed.clone())).expect("couldn't serialize request for transaction"),
                                    std::time::Duration::new(8,0)){
                                        Ok(h)=>h.payload,
                                        Err(e)=>{ println!("{}",e); continue'main }
                                };
                                let tx : Transaction = match serde_json::from_slice(&req_tx).unwrap(){
                                    SyncType::Transaction(h)=>Transaction::deserialize_slice(&h), _ => panic!()};
                                if tx.verify(){
                                    mempool.insert(hexed, tx);
                                }else{
                                    panic!("tx invalid in chain");
                                }
                            }
                        }

                        for k in b.hashedblock.blockdata.txes.iter(){
                            let hexed = hex::encode(k);
                            match mempool.remove(&hexed){
                                Some(x)=>{
                                    txdb.put(k, x.serialize()).expect("txdb failed while making verifying db");
                                },
                                None=>{
                                    panic!("memory pool didn't hold a transaction i already ask for and supposedly received");
                                }
                            }
                        }
                        block_height+=1;
                        head = b.clone();
                        let head_hash = &head.hash();
                        blockdb.put("height", block_height.to_string()).expect("couldn't store new chain height");
                        blockdb.put("block".to_owned() + &block_height.to_string(), &head_hash).expect("couldn't store new block hash to its height");
                        blockdb.put(&head_hash, serde_json::to_string(&head).expect("156")).expect("failed to put received, verified and validated block in db");
                        blockdb.flush().unwrap();
                        txdb.flush().unwrap();
                        println!("at height {} is block {}", block_height, head_hash);
                        pool_size = 0;
                    }
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
                    let mut txhashese: Vec<String> = mempool.iter().map(|(k, v)| {
                        txdb.put(k.clone(), v.serialize()).expect("txdb failure while making block");
                        k.clone()
                    } ).collect();
                    
                    txhashese.sort();
                    let txhashes: Vec<[u8;32]> = txhashese.iter().map(|k| {
                        println!("{}",k);
                        mempool.remove(k);
                        vec_to_arr(&hex::decode(k.clone()).expect("hex decode failed"))
                    } ).collect();
                    pool_size = 0;
                    block_height +=1;
                    head = Block::new(head.hash(), txhashes, &keys.ec, block_height);
                    let head_hash = head.hash();
                    blockdb.put("height", block_height.to_string()).expect("couldn't store new height while making block");
                    blockdb.put("block".to_owned()+&block_height.to_string(), &head_hash).expect("couldn't store block hash to its height");
                    blockdb.put(&head_hash, serde_json::to_string(&head).expect("couldn't store block to hash while making it"));
                    println!("at height {} is block {}", block_height, head_hash);
                    client.publish("block.propose", &head.block_to_blob(), None);
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
                sendr.send(block_height).expect("couldn't send height to rpc");
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
                let hexhash = hex::encode(blake2b(&pubk));
                match pubkeys.get(&hexhash){
                    Some(_)=>{ continue }
                    None=>{
                        // println!("{:?}", pubk);
                        let pk = PublicKey::from_bytes(&pubk).expect("couldn't read public key");
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
                            main_send.send(ret).expect("couldn't return new smart contract hash to rpc");
                            break
                        }
                        Err(_)=>{ continue }
                    }
                }
            },
            Event::Synchronize(s, r)=>{
                let dat = match serde_json::from_slice(&s).expect("couldn't deserialize SyncType on received request"){
                    SyncType::GetHeight => {
                        //chain height
                        println!("GetHeight");
                        SyncType::Height(block_height)
                    },
                    SyncType::GetNemezis => {
                        SyncType::Block(match &blockdb.get(&nemezis_hash).expect("couldn't get my genesis block when someone asked for it"){
                            Some(b)=>b.to_vec(),
                            None=> panic!("no genezis block?!")
                        })
                    }
                    SyncType::AtHeight(h) => {
                        //block hash at h height
                        println!("got asked height {}", h);
                        SyncType::BlockHash(String::from_utf8_lossy(match &blockdb.get("block".to_string()+&h.to_string()).expect("couldn't get block at hash"){
                            Some(h)=>h,
                            None=> continue'main
                        }).to_string())
                    },
                    SyncType::TransactionAtHash(hash) => {
                        //get transaction at hash
                        println!("got asked tx hash {}", hash);
                        let tx = match mempool.get(&hash){
                            Some(t) => serde_json::to_vec(&t).expect("couldn't serialize transaction when someone asked for it"),
                            None => match txdb.get(hash).expect("someone asked for a transaction i don't have in mempool or db"){
                                Some(x)=> x,
                                None => continue'main
                            }
                        };
                        SyncType::Transaction(tx)
                    },
                    SyncType::BlockAtHash(hash) => {
                        //get block at hash       
                        println!("got asked block hash {}", &hash);  
                        SyncType::Block(match blockdb.get(&hash).expect("blockdb failure when someone asked for it"){
                            Some(b) => b, None => {println!("someone asked for a block i don't have: {}", &hash); continue'main}
                        })
                    },
                    _ => { continue'main }
                };
                // println!("dat: {:?}", dat);
                client.publish(&r, &serde_json::to_vec(&dat).expect("couldn't serialize an answer to a syncronization request"), None);
            },
        }
    }
}