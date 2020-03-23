use std::io::Write;
use crate::error::QanError;

/// Generator function for genesis block. Default genesis transaction is the default settings for making consensus in this preconsensus version.
#[cfg(not(feature = "quantum"))]
pub fn generate_nemezis_block(keys: &crate::pk::PetKey) -> Result<(crate::block::Block, crate::transaction::Transaction), QanError>{
    // let ConsensusSettings = crate::conset::ConsensusSettings::default();
    // let nemezis_body = crate::transaction::TxBody::new([0;32], serde_json::to_vec(&ConsensusSettings).map_err(|e|QanError::Serde(e))?);
    let nemezis_body = crate::transaction::TxBody::new([0;32], 0, None); 
    let nemesis_tx = crate::transaction::Transaction::new(nemezis_body, &keys.ec)?;
    let mut nemezis_vec : Vec<[u8;32]> = Vec::new();
    nemezis_vec.push(nemesis_tx.hash()?);
    let block = crate::block::Block::new([0;32], nemezis_vec, &keys.ec, 0)?;
    let mut pemf = std::fs::File::create(std::path::Path::new("NEMEZIS")).map_err(|e|QanError::Io(e))?;
    pemf.write_all(&serde_json::to_vec(&block).map_err(|e|QanError::Serde(e))?);
    Ok((block, nemesis_tx))
}

/// Generator function for genesis block. Default genesis transaction is the default settings for making consensus in this preconsensus version.
#[cfg(feature = "quantum")]
pub fn generate_nemezis_block(keys: &crate::pk::PetKey) -> Result<(crate::block::Block, crate::transaction::Transaction), QanError>{
//     let ConsensusSettings = crate::conset::ConsensusSettings::default();
    // let nemezis_body = crate::transaction::TxBody::new([0;32], serde_json::to_vec(&ConsensusSettings).map_err(|e|QanError::Serde(e))?);
    let nemezis_body = crate::transaction::TxBody::new([0;32], 0, None);  
    let nemesis_tx = crate::transaction::Transaction::new(nemezis_body, &keys.glp)?;
    let mut nemezis_vec = Vec::new();
    nemezis_vec.push(nemesis_tx.hash()?);
    let block = crate::block::Block::new([0;32], nemezis_vec, &keys.glp, 0)?;
    let mut pemf = std::fs::File::create(std::path::Path::new("qNEMEZIS")).map_err(|e|QanError::Io(e))?;
    pemf.write_all(&serde_json::to_vec(&block).map_err(|e|QanError::Serde(e))?);
    Ok((block, nemesis_tx))
}