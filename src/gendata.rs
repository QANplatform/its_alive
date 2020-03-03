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
pub fn gen_data(){
    let keys = crate::pk::PetKey::new();
    let mut txdb = DB::open_default("qtx.db").expect("cannot open txdb");
    let mut blockdb = DB::open_default("qdb.db").expect("cannot open blockdb");
    let (mut head, tx) = crate::nemezis::generate_nemezis_block(&keys);
    let mut block_height = 0;
    blockdb.put("height", &block_height.to_string()).expect("couldn't store new chain height");
    blockdb.put("block".to_owned() + &block_height.to_string(), &head.hash()).expect("couldn't store new block hash to its height");
    blockdb.put(&head.hash(), serde_json::to_vec(&head).expect("156")).expect("failed to put received, verified and validated block in db");
    txdb.put(tx.hash(), serde_json::to_vec(&tx).unwrap());
    println!("start at :{}", crate::util::timestamp());
    for i in 0..16{
        let mut tx_es = Vec::new();
        for j in 0..8196{
            let tx = Transaction::new(TxBody::new([0;32], crate::util::urandom(980)), &keys.glp);
            txdb.put(tx.hash(), serde_json::to_vec(&tx).unwrap());
            tx_es.push(tx.hash());
        }
        block_height+=1;
        head = Block::new(head.hash(), tx_es, &keys.glp, block_height);
        blockdb.put("height", block_height.to_string()).expect("couldn't store new chain height");
        blockdb.put("block".to_owned() + &block_height.to_string(), &head.hash()).expect("couldn't store new block hash to its height");
        blockdb.put(&head.hash(), serde_json::to_vec(&head).expect("156")).expect("failed to put received, verified and validated block in db");
        blockdb.flush().unwrap();
        txdb.flush().unwrap();
        println!("block {} done at:{}", i, crate::util::timestamp());
    }
    println!("done");
}

//128 block
//8192 tx/block