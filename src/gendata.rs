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
use crate::util::{do_hash, vec_to_arr};
use crate::sync::{sync, genesis_getter};
use crate::error::QanError;
#[cfg(feature = "quantum")]
use glp::glp::{GlpPk, gen_pk};
use rocksdb::DB;

#[cfg(feature = "quantum")]
pub fn gen_data() -> Result<(), QanError>{
    let genkeys = crate::pk::PetKey::new();
    let mut txdb = DB::open_default("qtx.db").map_err(|e|QanError::Database(e))?;
    let mut blockdb = DB::open_default("qdb.db").map_err(|e|QanError::Database(e))?;
    let mut pkeys = DB::open_default("qpubkeys.db").map_err(|e|QanError::Database(e))?;
    let mahgenkey = genkeys.get_glp_pk_bytes();
    pkeys.put(do_hash(&mahgenkey),&mahgenkey);
    pkeys.flush().map_err(|e|QanError::Database(e))?;

    let (mut head, tx) = crate::nemezis::generate_nemezis_block(&genkeys)?;
    let mut block_height = 0;

    blockdb.put("height", &block_height.to_string()).map_err(|e|QanError::Database(e))?;
    blockdb.put("block".to_owned() + &block_height.to_string(), &head.hash()).map_err(|e|QanError::Database(e))?;
    blockdb.put(&head.hash(), serde_json::to_vec(&head).map_err(|e|QanError::Serde(e))?).map_err(|e|QanError::Database(e))?;
    txdb.put(tx.hash()?, serde_json::to_vec(&tx).map_err(|e|QanError::Serde(e))?).map_err(|e|QanError::Database(e))?;
    println!("start at :{}", crate::util::timestamp());
    for i in 0..1001{
        let keys = crate::pk::PetKey::new();
        let mahkey = keys.get_glp_pk_bytes();
        pkeys.put(do_hash(&mahkey),&mahkey);
        pkeys.flush().map_err(|e|QanError::Database(e))?;
        let mut tx_es = Vec::new();
        for j in 0..12{
            let tx = Transaction::new(TxBody::new([0;32], crate::util::urandom(980)), &keys.glp)?;
            txdb.put(tx.hash()?, serde_json::to_vec(&tx).map_err(|e|QanError::Serde(e))?).map_err(|e|QanError::Database(e))?;
            tx_es.push(tx.hash()?);
        }
        block_height+=1;
        head = Block::new(head.hash(), tx_es, &keys.glp, block_height)?;
        blockdb.put("height", block_height.to_string()).map_err(|e|QanError::Database(e))?;
        blockdb.put("block".to_owned() + &block_height.to_string(), &head.hash()).map_err(|e|QanError::Database(e))?;
        blockdb.put(&head.hash(), serde_json::to_vec(&head).map_err(|e|QanError::Serde(e))?).map_err(|e|QanError::Database(e))?;
        blockdb.flush().map_err(|e|QanError::Database(e))?;
        txdb.flush().map_err(|e|QanError::Database(e))?;
        println!("block {} done at:{}", i, crate::util::timestamp());
    }
    println!("done");
    Ok(())
}

//16 block
//8192 tx/block